-- Headless stand-in for the native `glu` module.

local glu = {}

setmetatable(glu, {__index = function (t, k)
  if not k:match('^glu%u') then
    error('unknown glu member "' .. k .. '"', 2)
  end
  local value = function () end
  t[k] = value
  return value
end})

return glu
