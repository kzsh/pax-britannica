-- Generates differential-test vectors for src/collision.rs from the Lua
-- implementation the golden trace was recorded against.
--
-- Regenerate with: lua5.4 test/collision_vectors.lua > traces/collision.txt
--
-- One case per line, all numbers %.17g:
--
--   n1 x y [x y ...] px1 py1 fx1 fy1  n2 x y [x y ...] px2 py2 fx2 fy2  hit [cx cy]
--
-- Facings are emitted as components rather than as angles so that the Rust side
-- never has to reproduce Lua's cos/sin bit for bit -- that is a libm question,
-- not a collision question, and it would only obscure a real disagreement.

local root = arg[0]:match('^(.*)/test/[^/]+%.lua$') or '.'
package.path = root .. '/test/stubs/?.lua;' .. root .. '/?.lua;' ..
               root .. '/?/init.lua;' .. package.path

local collision = require 'dokidoki.collision'
local v2 = require 'dokidoki.v2'

local function n(x) return string.format('%.17g', x) end

-- the collision shapes the game actually uses, from blueprints.lua
local SHAPES = {
  {'fighter', collision.make_rectangle(9, 6)},
  {'bomber', collision.make_rectangle(22, 14)},
  {'frigate', collision.make_rectangle(54, 36)},
  {'factory', collision.make_rectangle(170, 100)},
  {'laser', collision.make_rectangle(32, 1)},
  {'bomb', collision.make_rectangle(4, 4)},
  {'missile', collision.make_rectangle(5, 2)},
  -- non-rectangles too, to exercise the odd-vertex-count paths
  {'triangle', collision.make_polygon{v2(-10, -8), v2(12, -6), v2(0, 14)}},
  {'pentagon', collision.make_polygon{
    v2(-9, -12), v2(11, -9), v2(14, 6), v2(1, 15), v2(-11, 5)}},
  -- wound clockwise, so make_polygon has to reverse it
  {'reversed', collision.make_polygon{v2(0, 14), v2(12, -6), v2(-10, -8)}},
}

local function emit_poly(out, poly)
  local data = poly.data
  out[#out+1] = tostring(#data)
  for _, v in ipairs(data) do
    out[#out+1] = n(v.x)
    out[#out+1] = n(v.y)
  end
end

local function emit_body(out, poly, pos, facing)
  emit_poly(out, poly)
  out[#out+1] = n(pos.x)
  out[#out+1] = n(pos.y)
  out[#out+1] = n(facing.x)
  out[#out+1] = n(facing.y)
end

math.randomseed(20260805)

local hits = 0
local cases = 3000

for _ = 1, cases do
  local shape1 = SHAPES[math.random(#SHAPES)][2]
  local shape2 = SHAPES[math.random(#SHAPES)][2]

  -- kept close together so that a good share of the cases actually overlap;
  -- a file of near misses would test only the broad phase
  local spread = 60
  local pos1 = v2(math.random() * spread, math.random() * spread)
  local pos2 = v2(math.random() * spread, math.random() * spread)
  local facing1 = v2.unit(math.random() * math.pi * 2)
  local facing2 = v2.unit(math.random() * math.pi * 2)

  local body1 = {pos = pos1, facing = facing1, poly = shape1}
  local body2 = {pos = pos2, facing = facing2, poly = shape2}

  local out = {}
  emit_body(out, shape1, pos1, facing1)
  emit_body(out, shape2, pos2, facing2)

  local correction = collision.collide(body1, body2)
  if correction then
    hits = hits + 1
    out[#out+1] = '1'
    out[#out+1] = n(correction.x)
    out[#out+1] = n(correction.y)
  else
    out[#out+1] = '0'
  end

  print(table.concat(out, ' '))
end

io.stderr:write(string.format('%d cases, %d hits (%.1f%%)\n', cases, hits,
                              100 * hits / cases))
