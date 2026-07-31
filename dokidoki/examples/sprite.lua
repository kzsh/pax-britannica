local gl = require 'gl'
local glfw = require "glfw"

local graphics = require "dokidoki.graphics"
local kernel = require "dokidoki.kernel"

local function make_sprite_scene ()
  local time = 0
  local sprite = false

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
    if not sprite then
      sprite = graphics.sprite_from_image("rgba.png", nil, "center")
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
    gl.glPushMatrix()
      gl.glTranslated(320, 240, 0)
      gl.glRotated(math.sin(time * math.pi) * 50, 0, 0, 1)
      gl.glColor3d(1, 1, 1)
      sprite:draw()
    gl.glPopMatrix()
  end

  return {handle_event = handle_event, update = update, draw = draw}
end

kernel.set_ratio(640/480)
kernel.start_main_loop(make_sprite_scene())

