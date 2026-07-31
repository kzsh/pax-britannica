local gl = require 'gl'
local glfw = require "glfw"

local graphics = require "dokidoki.graphics"
local kernel = require "dokidoki.kernel"

local function make_texture_scene ()
  local time = 0
  local tex = false

  local function handle_event (event)
    if event.type == 'quit' or
       event.type == 'key' and event.is_down and event.key == glfw.KEY_ESC then
      kernel.abort_main_loop()
    end
  end

  local function update (dt)
    time = time + dt
  end

  local function init_graphics ()
    if not tex then
      local image = string.char(
        255, 0,   0,   255,
        0,   255, 0,   255,
        255, 255, 255, 255,

        0,   0,   255, 255,
        0,   0,   0,   0,
        255, 255, 255, 255,

        0,   0,   0,   255,
        0,   0,   0,   255,
        128, 128, 128, 255)
      -- the image will be padded out to 4x4 for the texture, with the right
      -- and bottom edges extending out
      tex = graphics.texture_from_string(image, 3, 3, 4)
    end
    gl.glClearColor(0.3 + math.cos(time/2) * 0.1, 0, 0.75, 0)
    gl.glClear(gl.GL_COLOR_BUFFER_BIT)
    gl.glMatrixMode(gl.GL_PROJECTION)
    gl.glLoadIdentity()
    gl.glOrtho(0, 640, 0, 480, 1, -1)
    gl.glMatrixMode(gl.GL_MODELVIEW)
    gl.glLoadIdentity()

    gl.glEnable(gl.GL_BLEND)
    gl.glBlendFunc(gl.GL_SRC_ALPHA, gl.GL_ONE_MINUS_SRC_ALPHA)
  end

  local function draw ()
    init_graphics()
    tex:enable()
    gl.glPushMatrix()
      gl.glTranslated(320, 240, 0)
      gl.glColor3d(1, 1, 1)
      gl.glBegin(gl.GL_QUADS)
      gl.glTexCoord2d(0, 1) gl.glVertex2d(-200, -200)
      gl.glTexCoord2d(1, 1) gl.glVertex2d( 200, -200)
      gl.glTexCoord2d(1, 0) gl.glVertex2d( 200,  200)
      gl.glTexCoord2d(0, 0) gl.glVertex2d(-200,  200)
      gl.glEnd()
    gl.glPopMatrix()
    tex:disable()
  end

  return {handle_event = handle_event, update = update, draw = draw}
end

kernel.set_ratio(640/480)
kernel.start_main_loop(make_texture_scene())

