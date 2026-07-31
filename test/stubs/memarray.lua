-- Headless stand-in for the native `memarray` module.
--
-- Backed by a plain table; `ptr()` returns the array itself, which the gl stub
-- knows how to write into.

local memarray_mt = {__index = {
  ptr = function (self) return self end
}}

return function (_type, length)
  local array = setmetatable({}, memarray_mt)
  for i = 0, length - 1 do array[i] = 0 end
  return array
end
