-- Counts how many math.random draws each particle effect takes, by running the
-- real components/particles.lua with a counting RNG.
--
-- Usage: lua5.4 test/particle_draws.lua
--
-- src/particles.rs has to consume the stream identically or every frame after
-- the first explosion diverges, so these counts are asserted in its unit tests.

local root = arg[0]:match('^(.*)/test/[^/]+%.lua$') or '.'
package.path = root .. '/test/stubs/?.lua;' .. root .. '/?.lua;' ..
               root .. '/?/init.lua;' .. package.path

local v2 = require 'dokidoki.v2'

-- components/particles.lua is a dokidoki component: it reads `game` from its
-- environment and defines its entry points as globals. Rather than stand up the
-- whole framework, load the chunk with an environment that supplies just enough.
local function fake_sprite()
  return {size = {8, 8}, tex = {name = 0}}
end

local resources = {}
setmetatable(resources, {__index = function () return fake_sprite() end})

local env = setmetatable({
  game = {
    resources = resources,
    actors = {new_generic = function () end},
    debug_keys = {key_held = function () return false end},
  },
}, {__index = _G})

local chunk = assert(loadfile(root .. '/components/particles.lua', 't', env))
chunk()

local draws = 0
local real_random = math.random
math.random = function (...)
  draws = draws + 1
  return real_random(...)
end

local function count(label, f)
  draws = 0
  f()
  print(string.format('%-28s %d', label, draws))
end

local function actor(name)
  return {
    blueprint = {name = name},
    transform = {pos = v2(0, 0)},
    ship = {velocity = v2(0, 0)},
  }
end

local function bullet(name)
  local b = actor(name)
  b.bullet = {velocity = v2(1, 0)}
  return b
end

count('add_bubble', function () env.add_bubble(v2(0, 0)) end)
count('explode (factory -> big)', function () env.explode(actor('factory')) end)
count('explode (frigate -> big)', function () env.explode(actor('frigate')) end)
count('explode (bomber -> mid)', function () env.explode(actor('bomber')) end)
count('explode (fighter -> small)', function () env.explode(actor('fighter')) end)
count('bullet_hit (laser)', function ()
  env.bullet_hit(actor('fighter'), bullet('laser'))
end)
count('bullet_hit (bomb -> mid)', function ()
  env.bullet_hit(actor('fighter'), bullet('bomb'))
end)
count('bullet_hit (missile -> tiny)', function ()
  env.bullet_hit(actor('fighter'), bullet('missile'))
end)
