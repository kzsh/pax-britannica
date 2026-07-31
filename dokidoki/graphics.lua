--- dokidoki.graphics
--- =================
---
--- Sprite, texture and text drawing helpers on top of the raw OpenGL bindings.

local gl = require "gl"
local glu = require "glu"
local memarray = require "memarray"
local stb_image = require "stb_image"

local will = require "dokidoki.private.will"

local graphics = {}

---- Utilities ----------------------------------------------------------------

local function to_power_of_two(x)
  return 1 << math.ceil(math.log(x, 2))
end

local function is_power_of_two(x)
  return x == to_power_of_two(x)
end

local function new_texture_name()
  local namebuffer = memarray('GLint', 1)
  gl.glGenTextures(1, namebuffer:ptr())
  return namebuffer[0]
end

local function delete_texture_name(name)
  local namebuffer = memarray("GLint", 1)
  namebuffer[0] = name
  gl.glDeleteTextures(1, namebuffer:ptr())
end

---- Textures -----------------------------------------------------------------

-- Stop it from garbage collecting the currently bound texture
local bound_texture = false

local texture_count = 0

local texture_mt = {__index = {
  enable = function (self)
    -- Enables 2D texturing and binds the texture.
    assert(self.name)
    bound_texture = self
    gl.glEnable(gl.GL_TEXTURE_2D)
    gl.glBindTexture(gl.GL_TEXTURE_2D, self.name)
  end,
  disable = function (self)
    -- Disables 2D texturing and unbinds the texture.
    assert(self.name)
    gl.glBindTexture(gl.GL_TEXTURE_2D, 0)
    gl.glDisable(gl.GL_TEXTURE_2D)
    bound_texture = false
  end,
  delete = function (self)
    -- Deletes the texture from video memory.
    --
    -- This invalidates the object, so don't use it anymore.
    if self.name then
      if bound_texture == self then self:disable() end
      delete_texture_name(self.name)
      texture_count = texture_count - 1
      self.name = false
    end
  end
}}

--- ### `make_texture(name)`
--- Makes a new texture object with the given OpenGL texture name.
---
--- The texture object is deleted automatically by the garbage collector, but it
--- is recommended that you delete it manually with `:delete()` in order to free
--- up video memory sooner.
function graphics.make_texture(name)
  local tex = setmetatable({name = name}, texture_mt)
  will.attach_will(tex, function () tex:delete() end)
  texture_count = texture_count + 1
  return tex
end

--- ### `get_texture_count()`
--- Returns the number of texture objects which are currently alive.
function graphics.get_texture_count()
  return texture_count
end

--- ### `texture_from_pointer(pointer, width, height, channels)`
--- Creates an OpenGL texture from the data pointed to by `pointer`.
---
--- `width` and `height` must be powers of two. `channels` must be 1, 2, 3, or 4
--- for V, VA, RGB, and RGBA respectively. `pointer` should point to a native
--- array of `width * height * channels` unsigned bytes.
function graphics.texture_from_pointer(pointer, width, height, channels)
  assert(is_power_of_two(width) and is_power_of_two(height),
         "non-power-of-two dimensions given")
  local format = channels == 1 and gl.GL_LUMINANCE or
                 channels == 2 and gl.GL_LUMINANCE_ALPHA or
                 channels == 3 and gl.GL_RGB or
                 channels == 4 and gl.GL_RGBA
  assert(format, "invalid channels given")

  local name = new_texture_name()
  gl.glBindTexture(gl.GL_TEXTURE_2D, name)
  gl.glTexParameterf(gl.GL_TEXTURE_2D, gl.GL_TEXTURE_MAG_FILTER, gl.GL_NEAREST)
  gl.glTexParameterf(gl.GL_TEXTURE_2D, gl.GL_TEXTURE_MIN_FILTER, gl.GL_NEAREST)
  gl.glTexParameterf(gl.GL_TEXTURE_2D, gl.GL_TEXTURE_WRAP_S, gl.GL_CLAMP)
  gl.glTexParameterf(gl.GL_TEXTURE_2D, gl.GL_TEXTURE_WRAP_T, gl.GL_CLAMP)
  glu.gluBuild2DMipmaps(gl.GL_TEXTURE_2D, format, width, height, format,
                        gl.GL_UNSIGNED_BYTE, pointer)
  gl.glBindTexture(gl.GL_TEXTURE_2D, 0)
  return graphics.make_texture(name)
end

local function image_string_to_powers_of_two(image_string, width, height,
                                             channels)
  assert(#image_string == width * height * channels,
         "image_string length doesn't match")
  assert(1 <= channels and channels <= 4, "invalid channels given")

  local new_width = to_power_of_two(width)
  local new_height = to_power_of_two(height)

  if new_width == width and new_height == height then
    return image_string, width, height, channels
  else
    local new_image = {}

    -- fill right
    for y = 1, height do
      local begin = (y-1) * width * channels + 1
      local ending = y * width * channels
      local fill = image_string:sub(ending + 1 - channels, ending)

      table.insert(new_image, image_string:sub(begin, ending))
      table.insert(new_image, string.rep(fill, new_width - width))
    end

    -- fill bottom
    local last_row = new_image[#new_image - 1] .. new_image[#new_image]
    for _ = height+1, new_height do
      table.insert(new_image, last_row)
    end

    return table.concat(new_image), new_width, new_height, channels
  end
end

--- ### `texture_from_string(image_string, width, height, channels)`
--- Creates an OpenGL texture from the data in `image_string`.
---
--- If `width` and `height` are not powers of two, the texture will be padded
--- out to the nearest greater power of two. The bytes in `image_string` are
--- treated as unsigned byte values. `channels` must be 1, 2, 3, or 4 for V, VA,
--- RGB, and RGBA respectively. `#image_string` should be
--- `width*height*channels`. The generated texture, the final width, and the
--- final height are returned.
function graphics.texture_from_string(image_string, width, height, channels)
  assert(#image_string == width * height * channels,
         "image_string length doesn't match")
  assert(1 <= channels and channels <= 4, "invalid channels given")

  if not (is_power_of_two(width) and is_power_of_two(height)) then
    image_string, width, height, channels =
      image_string_to_powers_of_two(image_string, width, height, channels)
  end

  local tex = graphics.texture_from_pointer(image_string, width, height,
                                            channels)

  return tex, width, height
end

--- ### `texture_from_image(filename)`
--- Creates an OpenGL texture from the image file given by `filename`.
---
--- Returns the created texture object, the final width, and the final height.
function graphics.texture_from_image(filename)
  return graphics.texture_from_string(assert(stb_image.load(filename)))
end

---- Sprites ------------------------------------------------------------------

local sprite_mt = {__index = {
  draw = function (self)
    local t = self.tex_rect
    local o = self.origin
    local s = self.size

    self.tex:enable()
    gl.glBegin(gl.GL_QUADS)
      gl.glTexCoord2d(t[1], t[2] + t[4])
      gl.glVertex2d(-o[1], -o[2])

      gl.glTexCoord2d(t[1] + t[3], t[2] + t[4])
      gl.glVertex2d(-o[1] + s[1], -o[2])

      gl.glTexCoord2d(t[1] + t[3], t[2])
      gl.glVertex2d(-o[1] + s[1], -o[2] + s[2])

      gl.glTexCoord2d(t[1], t[2])
      gl.glVertex2d(-o[1], -o[2] + s[2])
    gl.glEnd()
    self.tex:disable()
  end
}}

--- ### `make_sprite(tex, size, origin, tex_rect)`
--- Makes a sprite drawing the given texture. `origin` can be `"center"`.
function graphics.make_sprite(tex, size, origin, tex_rect)
  assert(tex)
  assert(size)

  if origin == "center" then
    origin = {size[1]/2, size[2]/2}
  end

  return setmetatable(
    {
      tex = tex,
      size = size,
      origin = origin or {0, 0},
      tex_rect = tex_rect or {0, 0, 1, 1}
    },
    sprite_mt)
end

--- ### `sprite_from_image_string(image_string, width, height, channels, size,
--- origin)`
--- Creates a sprite object from the given image data.
---
--- The size is the rendered size of the sprite, given as `{x, y}`. Its default
--- value is the pixel size of the image. `origin` specifies the point on the
--- sprite which should be rendered at the origin when the sprite is drawn, and
--- it's in the same units as the size. `origin` can either be `{x, y}` or
--- `"center"`, which sets it to half the size. `origin`'s default value is
--- `{0, 0}`.
function graphics.sprite_from_image_string(image_string, width, height,
                                           channels, size, origin)
  local tex, tex_w, tex_h =
    graphics.texture_from_string(image_string, width, height, channels)

  return graphics.make_sprite(
    tex,
    size or {width, height},
    origin or {0, 0},
    {0, 0, width/tex_w, height/tex_h})
end

--- ### `sprite_from_image(filename, size, origin)`
--- Creates a sprite object from the image file given by `filename`. See
--- `sprite_from_image_string` for size/origin information.
function graphics.sprite_from_image(filename, size, origin)
  local image_string, width, height, channels =
    assert(stb_image.load(filename))
  return graphics.sprite_from_image_string(
    image_string, width, height, channels, size, origin)
end

---- Fonts and Text -----------------------------------------------------------

local function font_map_line_height(font_map)
  local _, some_glyph = next(font_map)
  return some_glyph.size[2]
end

--- ### `draw_text(font_map, text)`
--- Draws `text` using the glyph sprites in `font_map`.
function graphics.draw_text(font_map, text)
  local line_height = font_map_line_height(font_map)

  local current_line = 0

  gl.glPushMatrix()
  for i = 1, #text do
    local char = text:sub(i, i)
    if char == "\n" then
      gl.glPopMatrix()
      current_line = current_line + 1
      gl.glPushMatrix()
      gl.glTranslated(0, current_line * -line_height, 0)
    else
      local sprite = font_map[char]
      if sprite then
        sprite:draw()
        gl.glTranslated(sprite.size[1], 0, 0)
      else
        error("Tried to render an unavailable character code " .. text:byte(i))
      end
    end
  end
  gl.glPopMatrix()
end

return graphics
