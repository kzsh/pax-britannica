-- Headless stand-in for the native `stb_image` module.
--
-- Reads the real dimensions out of the PNG header so that sprite sizes and
-- origins match the game's assets, but returns blank pixel data since nothing
-- is ever rasterized.

local stb_image = {}

local CHANNELS = 4

local function parse_png_header(data, source)
  assert(data:sub(1, 8) == '\137PNG\r\n\26\n', source .. ' is not a png')
  assert(data:sub(13, 16) == 'IHDR', source .. ' has no IHDR chunk')
  local width, height = string.unpack('>I4>I4', data, 17)
  return width, height
end

function stb_image.load_from_string(data)
  local width, height = parse_png_header(data, 'image string')
  return string.rep('\0', width * height * CHANNELS), width, height, CHANNELS
end

function stb_image.load(filename)
  local file = io.open(filename, 'rb')
  if not file then return nil, 'could not open ' .. filename end
  local header = file:read(24)
  file:close()
  if not header then return nil, 'could not read ' .. filename end
  local width, height = parse_png_header(header, filename)
  return string.rep('\0', width * height * CHANNELS), width, height, CHANNELS
end

return stb_image
