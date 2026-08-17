-- Drives the real player-select flow with a single human joining on each of
-- the four player keys, and asserts the CPU opponent's factory blueprint
-- matches the intended difficulty mapping:
--   player 1 (a/blue) -> easy
--   player 2 (f)       -> medium
--   player 3 (h)       -> hard
--   player 4 (l)       -> hard
--
-- Usage: lua5.4 test/cpu_difficulty_select.lua

local root = arg[0]:match('^(.*)/test/cpu_difficulty_select%.lua$') or '.'
package.path = root .. '/test/stubs/?.lua;' .. root .. '/?.lua;' ..
               root .. '/?/init.lua;' .. package.path

arg = {'--no-music', '--windowed'}

local dokidoki_game = require 'dokidoki.game'
local kernel = require 'dokidoki.kernel'

local game
local make_game = dokidoki_game.make_game
function dokidoki_game.make_game(update_methods, draw_methods, init)
  return make_game(update_methods, draw_methods, function (g)
    game = g
    init(g)
  end)
end

local the_game = require 'the_game'

function kernel.switch_scene() end
function kernel.abort_main_loop() end
kernel.set_video_mode(1024, 768)
kernel.set_ratio(4/3)
math.randomseed(1)
function math.randomseed() end

local blueprints = require 'blueprints'

local PLAYER_KEYS = {('A'):byte(), ('F'):byte(), ('H'):byte(), ('L'):byte()}

local cases = {
  {player = 1, key = PLAYER_KEYS[1], blueprint = blueprints.easy_enemy_factory,   name = 'easy'},
  {player = 2, key = PLAYER_KEYS[2], blueprint = blueprints.medium_enemy_factory, name = 'medium'},
  {player = 3, key = PLAYER_KEYS[3], blueprint = blueprints.hard_enemy_factory,   name = 'hard'},
  {player = 4, key = PLAYER_KEYS[4], blueprint = blueprints.hard_enemy_factory,   name = 'hard'},
}

local function run_case(case)
  game = nil
  local scene = the_game.make()

  -- the first update runs lazy init (which creates the key monitor), so the
  -- press has to land on frame 2. Holding it from then on picks the player;
  -- the 5s countdown and fade-out then hand off to start_game.
  for frame = 1, 450 do
    if frame == 2 then
      scene.handle_event{type = 'key', key = case.key, is_down = true}
    end
    scene.update(1/60)
  end

  local cpu, human
  for _, actor in ipairs(game.actors.get('factory')) do
    if actor.enemy_production then
      assert(not cpu, 'more than one CPU factory spawned')
      cpu = actor
    elseif actor.player_production then
      human = actor
    end
  end

  assert(human, case.name .. ': no human factory spawned')
  assert(cpu, case.name .. ': no CPU factory spawned')
  assert(cpu.blueprint == case.blueprint,
    string.format('player %d: expected %s CPU factory, got a different blueprint',
                  case.player, case.name))

  local logged = game.log.get_ai and game.log.get_ai()
  assert(logged == case.name,
    string.format('player %d: expected logged AI %q, got %q',
                  case.player, case.name, tostring(logged)))

  print(string.format('player %d (%s key) -> %s CPU (logged %q)  OK',
                      case.player, string.char(case.key), case.name, logged))
end

for _, case in ipairs(cases) do
  run_case(case)
end

print('single-player CPU difficulty mapping is correct')
