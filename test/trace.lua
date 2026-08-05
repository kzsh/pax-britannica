-- Records a per-frame state trace of a scripted run. This is the oracle the
-- Rust port is checked against: the Rust build replays the same input schedule
-- and must produce a byte-identical file.
--
-- Usage: lua5.4 test/trace.lua [frames] [--dump FIRST:LAST] [--out FILE]
--
--   --dump FIRST:LAST  also write the full per-field state for that frame range
--                      to stderr. The summary line only carries a hash, which
--                      tells you *that* two runs diverged; re-running both with
--                      --dump around the first bad frame tells you *how*.
--
-- What is covered: every live actor, in creation order, and every number,
-- boolean, string and vector field on each of its scripts. That is a superset
-- of the game state that matters, so a divergence anywhere in the simulation
-- shows up here.
--
-- What is deliberately not covered: particle state. The emitters are driven by
-- test/stubs/particles.lua rather than the real particles.c, so their contents
-- are not authoritative. Note that this does *not* make particles irrelevant to
-- the port -- the explosion code in components/particles.lua pulls hundreds of
-- draws off the shared math.random stream, so the Rust port has to reproduce
-- the number and order of those draws exactly or every later frame diverges.

-- loaded by path, since package.path isn't set up until setup_path() runs
local harness = dofile((arg[0]:match('^(.*)/test/[^/]+%.lua$') or '.') ..
                       '/test/harness.lua')

local root = harness.setup_path(arg[0])

---- Arguments ----------------------------------------------------------------

local frames = 12000
local dump_first, dump_last
local out_path = root .. '/traces/golden.txt'

local i = 1
while i <= #arg do
  local a = arg[i]
  if a == '--dump' then
    i = i + 1
    dump_first, dump_last = arg[i]:match('^(%d+):(%d+)$')
    assert(dump_first, 'expected --dump FIRST:LAST')
    dump_first, dump_last = tonumber(dump_first), tonumber(dump_last)
  elseif a == '--out' then
    i = i + 1
    out_path = assert(arg[i], 'expected --out FILE')
  else
    frames = assert(tonumber(a), 'unrecognised argument "' .. a .. '"')
  end
  i = i + 1
end

---- Hashing ------------------------------------------------------------------

-- FNV-1a, 64-bit. Lua 5.4 integers are 64-bit and wrap on overflow, which is
-- exactly the arithmetic FNV wants.
local FNV_OFFSET_BASIS = 0xcbf29ce484222325
local FNV_PRIME = 0x100000001b3

local function fnv1a(s, hash)
  hash = hash or FNV_OFFSET_BASIS
  -- pulled out in chunks; one string.byte call per byte is measurably slower
  -- over the ~20KB a busy frame produces
  for chunk_start = 1, #s, 64 do
    local chunk_end = math.min(chunk_start + 63, #s)
    for _, byte in ipairs{s:byte(chunk_start, chunk_end)} do
      hash = (hash ~ byte) * FNV_PRIME
    end
  end
  return hash
end

---- State serialisation ------------------------------------------------------

local v2 = require 'dokidoki.v2'
local v2_mt = getmetatable(v2.zero)

-- keys on an actor that are not scripts
local NOT_A_SCRIPT = {blueprint = true, tags = true}
-- keys on a script that point back out at the world
local NOT_STATE = {self = true, game = true}

local function format_number(n)
  -- %.17g round-trips a double exactly, so the trace pins down the last bit
  return string.format('%.17g', n)
end

local function format_value(v)
  local t = type(v)
  if t == 'number' then
    return format_number(v)
  elseif t == 'boolean' then
    return tostring(v)
  elseif t == 'string' then
    return string.format('%q', v)
  elseif t == 'table' and getmetatable(v) == v2_mt then
    return '(' .. format_number(v.x) .. ' ' .. format_number(v.y) .. ')'
  end
  -- functions, and tables that are sprites, polygons, actor references and so
  -- on: not simulation state, or not comparable across a language port
  return nil
end

local function sorted_keys(t, want)
  local keys = {}
  for k, v in pairs(t) do
    if type(k) == 'string' and want(k, v) then keys[#keys+1] = k end
  end
  table.sort(keys)
  return keys
end

-- Scripts keep plenty of state in chunk-level locals rather than in their env
-- table -- `local state = 'init'` in scripts/game_flow.lua is the scene state
-- machine -- and those are invisible from the outside. They are reachable as
-- upvalues of the script's own methods, though. Upvalue order is fixed at
-- compile time, so walking them is deterministic.
local function append_upvalue_state(out, prefix, script)
  local method_names = sorted_keys(script, function (_, v)
    return type(v) == 'function'
  end)

  local seen = {}
  local seen_names = {}
  for _, method_name in ipairs(method_names) do
    local method = script[method_name]
    for i = 1, math.huge do
      local name, value = debug.getupvalue(method, i)
      if not name then break end

      -- closures share upvalues; only report each one once
      local id = debug.upvalueid(method, i)
      if not seen[id] then
        seen[id] = true
        local formatted = format_value(value)
        if formatted then
          -- distinct upvalues can share a name across methods
          seen_names[name] = (seen_names[name] or 0) + 1
          local label = seen_names[name] == 1 and name
                        or name .. '#' .. seen_names[name]
          out[#out+1] = prefix .. '^' .. label .. ' = ' .. formatted
        end
      end
    end
  end
end

local function append_actor_state(out, actor)
  out[#out+1] = actor.blueprint.name ..
                (actor.paused and ' paused' or '') ..
                (actor.hidden and ' hidden' or '')

  local script_names = sorted_keys(actor, function (k, v)
    return not NOT_A_SCRIPT[k] and type(v) == 'table'
  end)

  for _, script_name in ipairs(script_names) do
    local script = actor[script_name]
    for _, key in ipairs(sorted_keys(script, function (k)
      return not NOT_STATE[k]
    end)) do
      local formatted = format_value(script[key])
      if formatted then
        out[#out+1] = '  ' .. script_name .. '.' .. key .. ' = ' .. formatted
      end
    end
    append_upvalue_state(out, '  ' .. script_name, script)
  end
end

local function frame_state(driver)
  local live = driver.live_actors()
  local out = {}
  for _, actor in ipairs(live) do
    append_actor_state(out, actor)
  end
  return table.concat(out, '\n'), #live
end

---- Run ----------------------------------------------------------------------

local TRACKED_TAGS =
  {'factory', 'fighter', 'bomber', 'frigate', 'laser', 'bomb', 'missile'}

local driver = harness.new()

local out = assert(io.open(out_path, 'w'))
out:write('# pax-britannica state trace\n')
out:write(string.format('# frames=%d lua=%s\n', frames, _VERSION))
out:write('# frame hash live_actors ' .. table.concat(TRACKED_TAGS, ' ') .. '\n')

for frame = 1, frames do
  driver.step(frame)

  local state, live_count = frame_state(driver)

  local counts = {}
  for _, tag in ipairs(TRACKED_TAGS) do
    counts[#counts+1] = tostring(driver.count(tag))
  end

  out:write(string.format('%d %016x %d %s\n', frame, fnv1a(state), live_count,
                          table.concat(counts, ' ')))

  if dump_first and dump_first <= frame and frame <= dump_last then
    io.stderr:write('---- frame ' .. frame .. ' ----\n' .. state .. '\n')
  end
end

out:write(string.format('# scene_switches=%d\n', driver.switched_scenes))
out:close()

io.write(string.format('wrote %s (%d frames, %d scene switches)\n', out_path,
                       frames, driver.switched_scenes))

-- A run this long has to have exercised the whole game; a trace of a run that
-- never got past the title screen would be a useless oracle.
if frames >= 12000 then
  assert(driver.switched_scenes > 0, 'the game never restarted')
end
