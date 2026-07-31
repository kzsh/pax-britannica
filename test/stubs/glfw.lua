-- Headless stand-in for the native `glfw` module.
--
-- Time advances by a fixed step every call so that the kernel's main loop never
-- blocks, and the window/input functions do nothing.

local glfw = {}

glfw.FALSE = 0
glfw.TRUE = 1
glfw.PRESS = 1
glfw.RELEASE = 0
glfw.KEY_ESC = 256
glfw.JOYSTICK_1 = 0
glfw.JOYSTICK_2 = 1
glfw.JOYSTICK_3 = 2
glfw.JOYSTICK_4 = 3

local time = 0

function glfw.Init() return true end
function glfw.Terminate() end

function glfw.GetTime()
  time = time + 1/600
  return time
end

function glfw.Sleep(seconds) time = time + seconds end

function glfw.GetDesktopMode()
  return {Width = 1024, Height = 768, RedBits = 8, GreenBits = 8, BlueBits = 8}
end

function glfw.GetVideoModes()
  return {glfw.GetDesktopMode()}
end

function glfw.OpenWindow() end
function glfw.SetWindowSize() end
function glfw.SetWindowTitle() end
function glfw.SwapBuffers() end

function glfw.GetJoystickButtons()
  return {glfw.RELEASE}
end

function glfw.SetWindowCloseCallback() end
function glfw.SetWindowSizeCallback() end
function glfw.SetKeyCallback() end
function glfw.PollEvents() end
function glfw.CloseWindow() end

return glfw
