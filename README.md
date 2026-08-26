# Pax Britannica

One-button real-time strategy. The original was made in 2010 for GAMMA4, a
one-button game competition.

This is a vibe-coded port of Pax Britannica to Lua 5.4 and GLFW 3, made with an
LLM coding agent. It has not been reviewed line by line by a human. The original
codebase, which this came from, is at
[henkboom/pax-britannica](https://github.com/henkboom/pax-britannica).

## Setup

The game can be played with one player versus the computer, or up to four
players head-to-head. Press `A` on the title screen to join the game. Other
players can join as long as the 5-second countdown doesn't run out. The `a`,
`f`, `h` and `l` keys are mapped to players 1-4. Alternately, you can play with
Xbox controllers.

## Controls

The game is controlled entirely with one button, the `A` button on the Xbox 360
controller.

Holding down the button spins the needle on the radial menu in the middle of the
player's factory ship. The needle will only travel as far as the player's
current resources allow. Resources (gold? seaweed? who knows!) accumulate over
time.

Releasing the button creates a ship that corresponds to the quadrant that the
needle is pointing at.

| Quadrant | Unit | Behaviour |
| --- | --- | --- |
| 1 | Fighter | Small, fast and cheap. Great at chasing down Bombers. |
| 2 | Bomber | Shoots slow projectiles that do massive damage. |
| 3 | Frigate | A great hulk of a ship that fires volleys of heat-seeking torpedoes. |
| 4 | Upgrade | Improve your factory ship to accumulate resources more quickly. |

```
                     __.......__
                _.-''    |      '-..
             ,-'         |          '-.
           ,'            |             '.
         ,'              |               '\
        /                |                 `
       /                 |                  `.
      /        4         |        1          \
     |                   |                    |
     |                   |                    |
    |                    |                     |
    |--------------------|---------------------|
    '.                   |                    .'
     |                   |                    |
      |                  |                   .'
       \       3         |        2          /
        \                |                 ,'
         `               |                /
          '.             |              ,'
            '-.          |           _,'
               '-._      |       _,-'
                   '`--......---'
```

Ships you spawn fight automatically using the latest in artificial
aquatelligence technology.

The player who keeps their factory ship alive wins.

## Building

See [compiling.txt](compiling.txt) for the full instructions. On Debian or
Ubuntu:

```bash
sudo apt install pkg-config liblua5.4-dev libglfw3-dev libgl-dev \
    libglu1-mesa-dev libasound2-dev
make linux
./pax-britannica --windowed
```

Useful flags: `--windowed` opens a 1024x768 window instead of going fullscreen,
`--stderr` logs to the terminal instead of a logfile, `--no-music` skips loading
the soundtrack, and `--debug` enables the debug keys.

The game logic also runs without a window, which needs nothing but the `lua5.4`
interpreter:

```bash
lua5.4 test/headless.lua
```

### The Rust port

A Rust rewrite is in progress alongside the Lua; see [PORTING.md](PORTING.md).
All of the game logic is ported. The renderer, on macroquad, is written but not
yet verified on a screen.

```bash
cargo run --release --bin pax               # the game, keyboard only for now
cargo run --release --bin headless -- 12000 # the game with no window, and a summary
cargo test
```

It also builds for the browser:

```bash
rustup target add wasm32-unknown-unknown
just serve                                  # build into web/dist and serve it
```

`just wasm` builds `web/dist` without serving it: the `.wasm`, miniquad's JS
glue, `web/index.html` and symlinks to `sprites/` and `audio/`, which the game
fetches by the same relative paths it reads on a desktop. Click the canvas
before playing, both to give it keyboard focus and to let the music start.

## Credits

Game design and programming by Henk Boom, Renaud Bédard and Matthew Gallant.
Artwork by Daniel Burton. Music by Ben Abraham.

## License

MIT, see [license.txt](license.txt). The vendored C code in `dokidoki-support`
carries its own notices in
[dokidoki-support/LICENSE](dokidoki-support/LICENSE): `memarray` is by Varol
Kaptan, `stb_image` and `stb_vorbis` are public domain by Sean Barrett, and
`luaglfw.h` is zlib-licensed by Camilla Berglund.
