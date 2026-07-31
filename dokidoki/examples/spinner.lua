local gl = require "gl"
local glfw = require "glfw"

local kernel = require "dokidoki.kernel"

local function make_spinner_scene ()
  local rotation = 0

  local function handle_event (event)
    if event.type == 'quit' or
       event.type == 'key' and event.is_down and event.key == glfw.KEY_ESC then
      kernel.abort_main_loop()
    end
  end

  local function update (dt)
    rotation = rotation + dt * 45
  end

  local function init_graphics ()
    gl.glClearColor(0, 0, 0.25, 0)
    gl.glClear(gl.GL_COLOR_BUFFER_BIT)
    gl.glMatrixMode(gl.GL_PROJECTION)
    gl.glLoadIdentity()
    gl.glOrtho(0, 300, 0, 300, 1, -1)
    gl.glMatrixMode(gl.GL_MODELVIEW)
    gl.glLoadIdentity()
  end

  local function draw ()
    init_graphics()
    gl.glPushMatrix()
      gl.glTranslated(150, 150, 0)
      gl.glRotated(rotation, 0, 0, 1)
      gl.glScaled(100, 100, 100)
      gl.glColor3d(0.5, 0.25, 1)
      gl.glBegin(gl.GL_QUADS)
        gl.glVertex2d(-1, -1)
        gl.glVertex2d( 1, -1)
        gl.glVertex2d( 1,  1)
        gl.glVertex2d(-1,  1)
      gl.glEnd()
    gl.glPopMatrix()
  end

  return {handle_event = handle_event, update = update, draw = draw}
end

kernel.set_video_mode(300, 300)
kernel.set_ratio(1)
kernel.start_main_loop(make_spinner_scene())

