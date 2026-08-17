-- Verifies that the three enemy-factory blueprints apply their difficulty
-- multiplier to the AI's harvest rate. The base rate comes from the 'resources'
-- script (scripts/resources.lua), and enemy_production scales it in place.
--
-- Usage: lua5.4 test/enemy_difficulty.lua

local root = arg[0]:match('^(.*)/test/enemy_difficulty%.lua$') or '.'
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

-- init runs lazily on the first update, which is where the game object gets
-- captured above; pump one frame to trigger it.
local scene = the_game.make()
scene.update(1/60)

local blueprints = require 'blueprints'
local v2 = require 'dokidoki.v2'

local BASE_HARVEST_RATE = 0.75

local cases = {
  {name = 'easy',   blueprint = blueprints.easy_enemy_factory,   multiplier = 0.8},
  {name = 'medium', blueprint = blueprints.medium_enemy_factory, multiplier = 1.0},
  {name = 'hard',   blueprint = blueprints.hard_enemy_factory,   multiplier = 1.03},
}

local player = 1
for _, case in ipairs(cases) do
  assert(case.blueprint, 'missing blueprint for "' .. case.name .. '"')

  local actor = game.actors.new(case.blueprint,
    {'transform', pos = v2(0, 0), facing = v2(0, 1)},
    {'ship', player = player})
  player = player + 1

  local expected = BASE_HARVEST_RATE * case.multiplier
  local actual = actor.resources.harvest_rate
  assert(math.abs(actual - expected) < 1e-9,
    string.format('%s: expected harvest_rate %.6f, got %.6f',
                  case.name, expected, actual))

  print(string.format('%-6s harvest_rate = %.4f (x%.2f)  OK',
                      case.name, actual, case.multiplier))
end

-- ordering sanity: more difficult means strictly more income
assert(BASE_HARVEST_RATE * 0.8 < BASE_HARVEST_RATE * 1.0, 'easy should trail medium')
assert(BASE_HARVEST_RATE * 1.0 < BASE_HARVEST_RATE * 1.03, 'medium should trail hard')

print('all difficulty tiers scale harvest_rate correctly')
