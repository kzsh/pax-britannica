-- Runs the real game logic for a number of frames without a window, using the
-- stub modules in test/stubs for everything that is normally implemented in C.
--
-- Usage: lua5.4 test/headless.lua [frames]
--
-- Any error in the update/draw path fails the run, which makes this the
-- smoke test for the framework's module and script loading. The input schedule
-- lives in test/harness.lua and is shared with test/trace.lua.

-- loaded by path, since package.path isn't set up until setup_path() runs
local harness = dofile((arg[0]:match('^(.*)/test/[^/]+%.lua$') or '.') ..
                       '/test/harness.lua')

harness.setup_path(arg[0])

local frames = tonumber(arg[1]) or 1200

local TRACKED_TAGS =
  {'factory', 'fighter', 'bomber', 'frigate', 'laser', 'bomb', 'missile'}
local peak = {}

local driver = harness.new()

for frame = 1, frames do
  driver.step(frame)

  for _, tag in ipairs(TRACKED_TAGS) do
    local n = driver.count(tag)
    if n > (peak[tag] or 0) then peak[tag] = n end
  end
end

print(string.format('ran %d frames, %d scene switches requested', frames,
                    driver.switched_scenes))
print(string.format(
  'actors: %d factories, %d fighters, %d bombers, %d frigates, %d lasers',
  driver.count('factory'), driver.count('fighter'), driver.count('bomber'),
  driver.count('frigate'), driver.count('laser')))

print('peak counts: ' .. (function ()
  local parts = {}
  for _, tag in ipairs(TRACKED_TAGS) do
    parts[#parts+1] = tag .. '=' .. (peak[tag] or 0)
  end
  return table.concat(parts, ' ')
end)())

-- a run long enough to get past the player select screen has to produce ships
if frames >= 1000 then
  assert(driver.count('factory') > 0, 'no factories in play')
  assert(driver.count('fighter') + driver.count('bomber') +
         driver.count('frigate') > 0, 'no ships were produced')
end

-- long runs exercise every ship, every projectile and a scene switch
if frames >= 12000 then
  for _, tag in ipairs(TRACKED_TAGS) do
    assert((peak[tag] or 0) > 0, 'nothing with the tag "' .. tag .. '" existed')
  end
  assert(driver.switched_scenes > 0, 'the game never restarted')
  driver.game.log.print_stats()
end
