//========================================================================
// Lua bindings for the parts of GLFW 3 that dokidoki uses.
//
// This replaces the old luaglfw binding for GLFW 2.x by Camilla Berglund;
// the naming of the Lua functions follows it, but callbacks are passed as
// functions instead of as names of global variables, and only the window,
// timing and joystick calls the framework needs are bound.
//========================================================================

#include <time.h>

#include <lua.h>
#include <lauxlib.h>

#include <GLFW/glfw3.h>

#include "luaglfw.h"

#define CALLBACKS_KEY "luaglfw.callbacks"

static lua_State *callback_state = NULL;
static GLFWwindow *window = NULL;

//************************************************************************
//****                        Handy functions                         ****
//************************************************************************

static GLFWwindow *check_window(lua_State *L)
{
    if(window == NULL)
        luaL_error(L, "no window is open");
    return window;
}

static void push_video_mode(lua_State *L, const GLFWvidmode *mode)
{
    lua_createtable(L, 0, 5);
    lua_pushinteger(L, mode->width);
    lua_setfield(L, -2, "Width");
    lua_pushinteger(L, mode->height);
    lua_setfield(L, -2, "Height");
    lua_pushinteger(L, mode->redBits);
    lua_setfield(L, -2, "RedBits");
    lua_pushinteger(L, mode->greenBits);
    lua_setfield(L, -2, "GreenBits");
    lua_pushinteger(L, mode->blueBits);
    lua_setfield(L, -2, "BlueBits");
}

// Stores the function at the top of the stack (or nil) as the callback called
// `name`, and pops it.
static void set_callback(lua_State *L, const char *name)
{
    luaL_checktype(L, -1, lua_isnil(L, -1) ? LUA_TNIL : LUA_TFUNCTION);
    luaL_getsubtable(L, LUA_REGISTRYINDEX, CALLBACKS_KEY);
    lua_insert(L, -2);
    lua_setfield(L, -2, name);
    lua_pop(L, 1);
}

// Pushes the callback called `name`, and returns whether it is a function.
static int push_callback(const char *name)
{
    lua_State *L = callback_state;
    if(L == NULL) return 0;

    luaL_getsubtable(L, LUA_REGISTRYINDEX, CALLBACKS_KEY);
    lua_getfield(L, -1, name);
    lua_remove(L, -2);

    if(lua_isfunction(L, -1)) return 1;

    lua_pop(L, 1);
    return 0;
}

//************************************************************************
//****                   Callback wrapper functions                   ****
//************************************************************************

static void windowclosefun(GLFWwindow *w)
{
    lua_State *L = callback_state;

    if(!push_callback("WindowClose")) return;

    // as in GLFW 2, a false return value cancels the close
    if(lua_pcall(L, 0, 1, 0) == LUA_OK)
    {
        int do_close = lua_toboolean(L, -1) && lua_tointeger(L, -1) != GLFW_FALSE;
        if(!do_close)
            glfwSetWindowShouldClose(w, GLFW_FALSE);
    }
    lua_pop(L, 1);
}

static void windowsizefun(GLFWwindow *w, int width, int height)
{
    lua_State *L = callback_state;
    (void)w;

    if(!push_callback("WindowSize")) return;

    lua_pushinteger(L, width);
    lua_pushinteger(L, height);
    if(lua_pcall(L, 2, 0, 0) != LUA_OK)
        lua_pop(L, 1);
}

static void keyfun(GLFWwindow *w, int key, int scancode, int action, int mods)
{
    lua_State *L = callback_state;
    (void)w;
    (void)scancode;
    (void)mods;

    // auto-repeat would look like a release to callers which compare against
    // glfw.PRESS
    if(action == GLFW_REPEAT) return;

    if(!push_callback("Key")) return;

    lua_pushinteger(L, key);
    lua_pushinteger(L, action);
    if(lua_pcall(L, 2, 0, 0) != LUA_OK)
        lua_pop(L, 1);
}

//************************************************************************
//****                    Initialization and windows                  ****
//************************************************************************

static int glfw_Init(lua_State *L)
{
    callback_state = L;
    lua_pushboolean(L, glfwInit() == GLFW_TRUE);
    return 1;
}

static int glfw_Terminate(lua_State *L)
{
    (void)L;
    window = NULL;
    glfwTerminate();
    return 0;
}

static int glfw_OpenWindow(lua_State *L)
{
    int width = (int)luaL_checkinteger(L, 1);
    int height = (int)luaL_checkinteger(L, 2);
    const char *title = luaL_optstring(L, 3, "dokidoki");
    int fullscreen = lua_toboolean(L, 4);

    if(window != NULL)
        return luaL_error(L, "a window is already open");

    glfwWindowHint(GLFW_RED_BITS, 8);
    glfwWindowHint(GLFW_GREEN_BITS, 8);
    glfwWindowHint(GLFW_BLUE_BITS, 8);
    glfwWindowHint(GLFW_ALPHA_BITS, 8);
    glfwWindowHint(GLFW_DEPTH_BITS, 24);
    glfwWindowHint(GLFW_STENCIL_BITS, 0);

    window = glfwCreateWindow(width, height, title,
                              fullscreen ? glfwGetPrimaryMonitor() : NULL,
                              NULL);
    if(window == NULL)
        return luaL_error(L, "window creation failed");

    glfwMakeContextCurrent(window);

    glfwSetWindowCloseCallback(window, windowclosefun);
    glfwSetWindowSizeCallback(window, windowsizefun);
    glfwSetKeyCallback(window, keyfun);

    return 0;
}

static int glfw_CloseWindow(lua_State *L)
{
    (void)L;
    if(window != NULL)
    {
        glfwDestroyWindow(window);
        window = NULL;
    }
    return 0;
}

static int glfw_SetWindowTitle(lua_State *L)
{
    glfwSetWindowTitle(check_window(L), luaL_checkstring(L, 1));
    return 0;
}

static int glfw_SetWindowSize(lua_State *L)
{
    int width = (int)luaL_checkinteger(L, 1);
    int height = (int)luaL_checkinteger(L, 2);
    glfwSetWindowSize(check_window(L), width, height);
    return 0;
}

static int glfw_GetWindowSize(lua_State *L)
{
    int width, height;
    glfwGetWindowSize(check_window(L), &width, &height);
    lua_pushinteger(L, width);
    lua_pushinteger(L, height);
    return 2;
}

static int glfw_SwapBuffers(lua_State *L)
{
    glfwSwapBuffers(check_window(L));
    glfwPollEvents();
    return 0;
}

static int glfw_PollEvents(lua_State *L)
{
    (void)L;
    glfwPollEvents();
    return 0;
}

static int glfw_GetDesktopMode(lua_State *L)
{
    GLFWmonitor *monitor = glfwGetPrimaryMonitor();
    if(monitor == NULL)
        return luaL_error(L, "no monitor found");
    push_video_mode(L, glfwGetVideoMode(monitor));
    return 1;
}

static int glfw_GetVideoModes(lua_State *L)
{
    GLFWmonitor *monitor = glfwGetPrimaryMonitor();
    int count = 0;
    const GLFWvidmode *modes;
    int i;

    if(monitor == NULL)
        return luaL_error(L, "no monitor found");

    modes = glfwGetVideoModes(monitor, &count);
    lua_createtable(L, count, 0);
    for(i = 0; i < count; i++)
    {
        push_video_mode(L, &modes[i]);
        lua_rawseti(L, -2, i + 1);
    }
    return 1;
}

//************************************************************************
//****                       Input and timing                         ****
//************************************************************************

static int glfw_GetKey(lua_State *L)
{
    int key = (int)luaL_checkinteger(L, 1);
    lua_pushinteger(L, glfwGetKey(check_window(L), key));
    return 1;
}

static int glfw_GetJoystickButtons(lua_State *L)
{
    int joystick = (int)luaL_checkinteger(L, 1);
    int wanted = (int)luaL_optinteger(L, 2, -1);
    int count = 0;
    const unsigned char *buttons = glfwGetJoystickButtons(joystick, &count);
    int i;

    if(wanted >= 0 && wanted < count) count = wanted;

    lua_createtable(L, count, 0);
    for(i = 0; i < count; i++)
    {
        lua_pushinteger(L, buttons[i]);
        lua_rawseti(L, -2, i + 1);
    }
    return 1;
}

static int glfw_GetTime(lua_State *L)
{
    lua_pushnumber(L, glfwGetTime());
    return 1;
}

static int glfw_SetTime(lua_State *L)
{
    glfwSetTime(luaL_checknumber(L, 1));
    return 0;
}

// GLFW 3 dropped glfwSleep
static int glfw_Sleep(lua_State *L)
{
    lua_Number seconds = luaL_checknumber(L, 1);
    struct timespec request;

    if(seconds <= 0) return 0;

    request.tv_sec = (time_t)seconds;
    request.tv_nsec = (long)((seconds - (lua_Number)request.tv_sec) * 1e9);
    while(nanosleep(&request, &request) == -1)
        ;

    return 0;
}

//************************************************************************
//****                        Callback setters                         ****
//************************************************************************

static int glfw_SetWindowCloseCallback(lua_State *L)
{
    lua_settop(L, 1);
    set_callback(L, "WindowClose");
    return 0;
}

static int glfw_SetWindowSizeCallback(lua_State *L)
{
    lua_settop(L, 1);
    set_callback(L, "WindowSize");
    return 0;
}

static int glfw_SetKeyCallback(lua_State *L)
{
    lua_settop(L, 1);
    set_callback(L, "Key");
    return 0;
}

//************************************************************************
//****                       Module definition                        ****
//************************************************************************

static const luaL_Reg glfwlib[] = {
    { "Init", glfw_Init },
    { "Terminate", glfw_Terminate },
    { "OpenWindow", glfw_OpenWindow },
    { "CloseWindow", glfw_CloseWindow },
    { "SetWindowTitle", glfw_SetWindowTitle },
    { "SetWindowSize", glfw_SetWindowSize },
    { "GetWindowSize", glfw_GetWindowSize },
    { "SwapBuffers", glfw_SwapBuffers },
    { "PollEvents", glfw_PollEvents },
    { "GetDesktopMode", glfw_GetDesktopMode },
    { "GetVideoModes", glfw_GetVideoModes },
    { "GetKey", glfw_GetKey },
    { "GetJoystickButtons", glfw_GetJoystickButtons },
    { "GetTime", glfw_GetTime },
    { "SetTime", glfw_SetTime },
    { "Sleep", glfw_Sleep },
    { "SetWindowCloseCallback", glfw_SetWindowCloseCallback },
    { "SetWindowSizeCallback", glfw_SetWindowSizeCallback },
    { "SetKeyCallback", glfw_SetKeyCallback },
    { NULL, NULL }
};

struct lua_constant
{
    const char *name;
    int value;
};

static const struct lua_constant glfw_constants[] = {
    { "FALSE", GLFW_FALSE },
    { "TRUE", GLFW_TRUE },
    { "RELEASE", GLFW_RELEASE },
    { "PRESS", GLFW_PRESS },
    { "KEY_ESC", GLFW_KEY_ESCAPE },
    { "KEY_SPACE", GLFW_KEY_SPACE },
    { "KEY_ENTER", GLFW_KEY_ENTER },
    { "JOYSTICK_1", GLFW_JOYSTICK_1 },
    { "JOYSTICK_2", GLFW_JOYSTICK_2 },
    { "JOYSTICK_3", GLFW_JOYSTICK_3 },
    { "JOYSTICK_4", GLFW_JOYSTICK_4 },
    { NULL, 0 }
};

int luaopen_glfw(lua_State *L)
{
    const struct lua_constant *constant;

    luaL_newlib(L, glfwlib);

    for(constant = glfw_constants; constant->name; constant++)
    {
        lua_pushinteger(L, constant->value);
        lua_setfield(L, -2, constant->name);
    }

    // remember the state used to call back into lua
    callback_state = L;

    return 1;
}
