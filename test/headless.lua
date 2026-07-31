-- Runs the real game logic for a number of frames without a window, using the
-- stub modules in test/stubs for everything that is normally implemented in C.
--
-- Usage: lua5.4 test/headless.lua [frames]
--
-- Any error in the update/draw path fails the run, which makes this the
-- smoke test for the framework's module and script loading.

local root = arg[0]:match('^(.*)/test/headless%.lua$') or '.'
package.path = root .. '/test/stubs/?.lua;' .. root .. '/?.lua;' ..
               root .. '/?/init.lua;' .. package.path

local frames = tonumber(arg[1]) or 1200

-- the game reads command line flags out of the global `arg` table
arg = {'--no-music', '--windowed'}

local dokidoki_game = require 'dokidoki.game'
local kernel = require 'dokidoki.kernel'

-- grab a handle on the game object, which is otherwise private to the scene
local game
local make_game = dokidoki_game.make_game
function dokidoki_game.make_game(update_methods, draw_methods, init)
  return make_game(update_methods, draw_methods, function (g)
    game = g
    init(g)
  end)
end

local the_game = require 'the_game'

-- the kernel's own main loop needs a window, so the scene is driven directly
-- here; these two calls only make sense inside a running main loop
local switched_scenes = 0
local next_scene
function kernel.switch_scene(scene)
  switched_scenes = switched_scenes + 1
  next_scene = scene
end
function kernel.abort_main_loop() end

kernel.set_video_mode(1024, 768)
kernel.set_ratio(4/3)

-- fixed seed for reproducible runs; the game reseeds itself on startup, so that
-- call is neutered here
math.randomseed(1)
function math.randomseed() end

local scene = the_game.make()

local PLAYER_KEYS = {('A'):byte(), ('F'):byte(), ('H'):byte(), ('L'):byte()}

local TRACKED_TAGS =
  {'factory', 'fighter', 'bomber', 'frigate', 'laser', 'bomb', 'missile'}
local peak = {}

local function press(key, is_down)
  scene.handle_event{type = 'key', key = key, is_down = is_down}
end

for frame = 1, frames do
  -- two players join at the start, then everyone hammers their button
  if frame == 2 then
    press(PLAYER_KEYS[1], true)
    press(PLAYER_KEYS[2], true)
  elseif frame == 3 then
    press(PLAYER_KEYS[1], false)
    press(PLAYER_KEYS[2], false)
  elseif frame > 3 then
    -- different hold lengths per player so that every quadrant of the radial
    -- menu (fighter, bomber, frigate, upgrade) gets used
    for i, key in ipairs(PLAYER_KEYS) do
      local period = 120 * i * i
      press(key, frame % period < period * 3 // 4)
    end
  end

  scene.update(1/60)
  scene.draw()

  for _, tag in ipairs(TRACKED_TAGS) do
    local n = #game.actors.get(tag)
    if n > (peak[tag] or 0) then peak[tag] = n end
  end

  if next_scene then
    scene = next_scene
    next_scene = nil
  end
end

local function count(tag) return #game.actors.get(tag) end

print(string.format('ran %d frames, %d scene switches requested', frames,
                    switched_scenes))
print(string.format(
  'actors: %d factories, %d fighters, %d bombers, %d frigates, %d lasers',
  count('factory'), count('fighter'), count('bomber'), count('frigate'),
  count('laser')))

print('peak counts: ' .. (function ()
  local parts = {}
  for _, tag in ipairs(TRACKED_TAGS) do
    parts[#parts+1] = tag .. '=' .. (peak[tag] or 0)
  end
  return table.concat(parts, ' ')
end)())

-- a run long enough to get past the player select screen has to produce ships
if frames >= 1000 then
  assert(count('factory') > 0, 'no factories in play')
  assert(count('fighter') + count('bomber') + count('frigate') > 0,
         'no ships were produced')
end

-- long runs exercise every ship, every projectile and a scene switch
if frames >= 12000 then
  for _, tag in ipairs(TRACKED_TAGS) do
    assert((peak[tag] or 0) > 0, 'nothing with the tag "' .. tag .. '" existed')
  end
  assert(switched_scenes > 0, 'the game never restarted')
  game.log.print_stats()
end
