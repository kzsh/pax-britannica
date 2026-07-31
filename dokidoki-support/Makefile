NAME=dokidoki-support

#### General Settings

TARGET=$(NAME)

# override this if your lua 5.4 pkg-config file goes by another name, e.g.
# `make linux LUA_PKG=lua-5.4` or `make linux LUA_PKG=lua`
LUA_PKG?=lua5.4
LUA_CFLAGS?=$(shell pkg-config --cflags $(LUA_PKG) 2>/dev/null)
LUA_LDFLAGS?=$(shell pkg-config --libs $(LUA_PKG) 2>/dev/null)

CFLAGS=-Wall -O2 -DMEMARRAY_USE_OPENGL \
	   $(LUA_CFLAGS) $(PLATFORM_CFLAGS) $(EXTRA_CFLAGS)
LDFLAGS=$(LUA_LDFLAGS) -lglfw \
		$(PLATFORM_LDFLAGS) $(EXTRA_LDFLAGS)
CC=gcc

#### Platform Settings

linux:
	$(MAKE) $(TARGET) PLATFORM=linux \
		PLATFORM_CFLAGS="-DDOKIDOKI_LINUX -pthread" \
		PLATFORM_LDFLAGS="-Wl,-E -pthread -lGL -lGLU -lX11 -lXrandr -lm -lasound"

macosx:
	$(MAKE) $(TARGET) PLATFORM=macosx \
		PLATFORM_CFLAGS="-DDOKIDOKI_MACOSX -I/System/Library/Frameworks/CoreFoundation.framework/Headers -I/opt/local/include" \
		PLATFORM_LDFLAGS="-lportaudio -L/opt/local/lib -framework AGL -framework OpenGL -framework Carbon"

mingw:
	$(MAKE) $(TARGET) PLATFORM=mingw \
		PLATFORM_CFLAGS="-DDOKIDOKI_MINGW" \
		PLATFORM_LDFLAGS="-lportaudio -lopengl32 -lglu32 -lole32 -lwinmm"

clean:
	rm -f *.o $(NAME)

#### Actual Building

OBJS= collision.o \
	  gl.o \
	  glu.o \
	  log.o \
	  lua_stb_image.o \
	  luaglfw.o \
	  memarray.o \
	  minlua.o \
	  mixer.o \
	  stb_vorbis.o \
	  $(EXTRA_OBJECTS)

# a missing lua is otherwise a confusing "lua.h: No such file or directory"
.PHONY: lua-check
lua-check:
	@test -n "$(strip $(LUA_LDFLAGS))" || { \
		echo "couldn't find lua 5.4 via pkg-config ($(LUA_PKG))"; \
		echo "on debian/ubuntu: apt install pkg-config liblua5.4-dev"; \
		echo "or pass the flags yourself, e.g."; \
		echo "    make linux LUA_CFLAGS=-I/usr/local/include LUA_LDFLAGS=-llua"; \
		exit 1; }

$(TARGET): lua-check $(OBJS)
	$(CC) -o $@ $(filter %.o,$^) $(LDFLAGS)

log.o: log.h
lua_stb_image.o: stb_image.c
minlua.o: log.h
mixer.o: stb_vorbis.h
