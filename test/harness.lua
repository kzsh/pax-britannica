-- Shared driver for the windowless runs.
--
-- Both test/headless.lua (smoke test) and test/trace.lua (golden trace for the
-- Rust port) go through here, so the two are guaranteed to drive the game with
-- exactly the same inputs. If they ever drifted apart the trace would stop
-- describing the run that the smoke test blesses.

local harness = {}

--- ### `setup_path(script_path)`
--- Points `package.path` at the repository root and the C-module stubs. Pass
--- `arg[0]`. Returns the root directory.
function harness.setup_path(script_path)
  local root = script_path:match('^(.*)/test/[^/]+%.lua$') or '.'
  package.path = root .. '/test/stubs/?.lua;' .. root .. '/?.lua;' ..
                 root .. '/?/init.lua;' .. package.path
  return root
end

--- The keys mapped to players 1-4.
harness.player_keys = {('A'):byte(), ('F'):byte(), ('H'):byte(), ('L'):byte()}

--- ### `new()`
--- Builds a scene and returns a driver for it. Must be called after
--- `setup_path()` and before anything else pulls in `the_game`.
function harness.new()
  -- the game reads command line flags out of the global `arg` table
  arg = {'--no-music', '--windowed'}

  local dokidoki_game = require 'dokidoki.game'
  local kernel = require 'dokidoki.kernel'

  local self = {
    -- every actor ever created by the current scene, in creation order, which
    -- is also the order the framework updates and draws them in
    spawn_order = {},
    switched_scenes = 0,
  }

  -- Grab a handle on the game object, which is otherwise private to the scene,
  -- and hook actor creation. Restarting the game builds a whole new game
  -- object, so this runs again on every scene and has to reset the actor list.
  local make_game = dokidoki_game.make_game
  function dokidoki_game.make_game(update_methods, draw_methods, init)
    return make_game(update_methods, draw_methods, function (game)
      self.game = game
      self.spawn_order = {}

      local new = game.actors.new
      function game.actors.new(...)
        local actor = new(...)
        self.spawn_order[#self.spawn_order+1] = actor
        return actor
      end

      init(game)
    end)
  end

  local the_game = require 'the_game'

  -- The kernel's own main loop needs a window, so the scene is driven directly
  -- from step() instead; these two calls only make sense inside a running main
  -- loop.
  local next_scene
  function kernel.switch_scene(scene)
    self.switched_scenes = self.switched_scenes + 1
    next_scene = scene
  end
  function kernel.abort_main_loop() end

  kernel.set_video_mode(1024, 768)
  kernel.set_ratio(4/3)

  -- Fixed seed for reproducible runs. The game reseeds itself on startup, so
  -- that call is neutered here.
  math.randomseed(1)
  function math.randomseed() end

  self.scene = the_game.make()

  local function press(key, is_down)
    self.scene.handle_event{type = 'key', key = key, is_down = is_down}
  end

  --- ### `step(frame)`
  --- Feeds one frame of scripted input, then updates and draws. `frame` is
  --- 1-based; the input schedule is a pure function of it.
  function self.step(frame)
    -- two players join at the start, then everyone hammers their button
    if frame == 2 then
      press(harness.player_keys[1], true)
      press(harness.player_keys[2], true)
    elseif frame == 3 then
      press(harness.player_keys[1], false)
      press(harness.player_keys[2], false)
    elseif frame > 3 then
      -- different hold lengths per player so that every quadrant of the radial
      -- menu (fighter, bomber, frigate, upgrade) gets used
      for i, key in ipairs(harness.player_keys) do
        local period = 120 * i * i
        press(key, frame % period < period * 3 // 4)
      end
    end

    self.scene.update(1/60)
    self.scene.draw()

    if next_scene then
      self.scene = next_scene
      next_scene = nil
    end
  end

  --- ### `live_actors()`
  --- The actors that are still alive, in creation order.
  function self.live_actors()
    local live = {}
    for _, actor in ipairs(self.spawn_order) do
      if not actor.dead then live[#live+1] = actor end
    end
    return live
  end

  --- ### `count(tag)`
  --- The number of live actors carrying `tag`.
  function self.count(tag)
    return #self.game.actors.get(tag)
  end

  return self
end

return harness
