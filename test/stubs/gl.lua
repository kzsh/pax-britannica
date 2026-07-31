-- Headless stand-in for the native `gl` module.
--
-- Every glFoo() call is a no-op and every GL_FOO constant is a number, which is
-- enough to run the game's logic without a GL context.

local gl = {}

local next_texture_name = 1

function gl.glGenTextures(n, buffer)
  for i = 0, n - 1 do
    buffer[i] = next_texture_name
    next_texture_name = next_texture_name + 1
  end
end

function gl.glDeleteTextures() end

function gl.glGetError() return 0 end

local next_constant = 0

setmetatable(gl, {__index = function (t, k)
  local value
  if k:match('^GL_') then
    next_constant = next_constant + 1
    value = next_constant
  elseif k:match('^gl%u') then
    value = function () end
  else
    error('unknown gl member "' .. k .. '"', 2)
  end
  t[k] = value
  return value
end})

return gl
