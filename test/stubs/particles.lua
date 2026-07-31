-- Headless stand-in for the native `particles` module.
--
-- Keeps the same bookkeeping as particles.c (fixed-size ring of particles with
-- damping and scaling) minus the drawing.

local PARTICLE_COUNT = 2000

local particles = {}

local emitter_mt = {__index = {}}
local emitter = emitter_mt.__index

function emitter.add_particle(self, x, y, xvel, yvel)
  self.next = self.next % PARTICLE_COUNT + 1
  self.particles[self.next] =
    {life = self.life, x = x, y = y, xvel = xvel, yvel = yvel, scale = 1}
end

function emitter.update(self)
  for i, p in pairs(self.particles) do
    if p.life > 0 then
      p.life = p.life - 1
      p.x = p.x + p.xvel
      p.y = p.y + p.yvel
      p.xvel = p.xvel * self.damping
      p.yvel = p.yvel * self.damping
      p.scale = p.scale + self.delta_scale
    else
      self.particles[i] = nil
    end
  end
end

function emitter.draw() end

function emitter.live_count(self)
  local count = 0
  for _ in pairs(self.particles) do count = count + 1 end
  return count
end

function particles.make_emitter(width, height, texture, life, damping,
                                delta_scale)
  return setmetatable({
    width = width,
    height = height,
    texture = texture,
    life = life,
    damping = damping,
    delta_scale = delta_scale,
    next = 0,
    particles = {}
  }, emitter_mt)
end

return particles
