# Pax Britannica

One-button real-time strategy. The original was made in 2010 for GAMMA4, a
one-button game competition.

This is a vibe-coded port of Pax Britannica to Rust and macroquad, made with an
LLM coding agent. It has not been reviewed line by line by a human. The original
codebase, Lua on a hand-rolled engine, is at
[henkboom/pax-britannica](https://github.com/henkboom/pax-britannica); the port
was driven against it frame by frame, as [PORTING.md](PORTING.md) describes.

## Setup

The game can be played with one player versus the computer, or up to four
players head-to-head. Press `A` on the title screen to join the game. Other
players can join as long as the 5-second countdown doesn't run out. The `a`,
`f`, `h` and `l` keys are mapped to players 1-4. Alternately, you can play with
Xbox controllers.

On a touchscreen the screen is split down the middle: tapping the left half
joins player 1, the right half player 2, and each thumb then holds the half it
joined with. One player alone gets a CPU opponent, and the whole screen becomes
theirs.

`F11` toggles fullscreen on the desktop. In the browser, use the button in the
corner: it takes the whole page fullscreen and asks for landscape, and tapping
it again is the way back out on a phone.

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

```bash
just run                                    # or: cargo run --release --bin pax
cargo run --release --bin headless -- 12000 # the game with no window, and a summary
just check                                  # fmt, clippy, tests
```

It also builds for the browser:

```bash
rustup target add wasm32-unknown-unknown
just serve                                  # build into web/dist and serve it
```

`just wasm` builds `web/dist` without serving it: the `.wasm`, miniquad's JS
glue, `web/index.html` and symlinks to `sprites/` and `audio/`, which the game
fetches by the same relative paths it reads on a desktop. The mouse counts as a
touch, so the split-screen controls can be tried without a phone. Click the
canvas before playing, both to give it keyboard focus and to let the music
start.

### Hosting it on Cloudflare Pages

The build is static files, so Pages serves it as-is, `.wasm` included. One-time
setup, with the account id from the Cloudflare dashboard:

```bash
export CLOUDFLARE_ACCOUNT_ID=...
npx wrangler@4 login
npx wrangler@4 pages project create pax-britannica --production-branch main
```

Then, for each release:

```bash
just deploy
```

`just bundle` runs [web/bundle.sh](web/bundle.sh), which lays out `web/upload`:
the payload under `/v/<hash>/`, hashed over its own contents, and an `index.html`
at the root carrying `<base href="/v/<hash>/">`. The game asks for `pax.wasm`,
`sprites/*.png` and `audio/music.ogg` by relative path, so the base tag moves the
whole set to the new prefix at once and [web/_headers](web/_headers) can mark it
`immutable`. Only `index.html` is ever revalidated. The deploy prints a
`*.pages.dev` URL.

The wasm is a megabyte and the game blocks on 6.4MB of sprites and music before
its first frame, so `web/index.html` opens with the title screen rebuilt in HTML
— the sea gradient from `render.rs`, the sprites at the coordinates `splash.rs`
and `game_flow.rs` place them — and a progress bar over it. The bar counts bytes
against `manifest.json`, which [web/manifest.sh](web/manifest.sh) writes at build
time; the page downloads everything itself, then hands the wasm to miniquad and
lets the game refetch the rest from cache.

The favicon is player one's factory ship, `sprites/factory_p1.png` scaled to a
square and flattened onto the sea colour of `sprites/background.png`, since a
faviconless page has nothing but a transparent hull to show. `web/favicon.png`
rides along under the hashed prefix; `web/favicon.ico` sits at the root for the
clients that ask for it before reading any HTML.

`just preview` serves `web/upload` locally, which is the hashed layout rather
than the flat one `just serve` gives you. `_headers` is Pages' own file and does
nothing under a local server, so caching only takes effect once deployed.

To reach it at a subdomain of a zone in the same account, add the hostname under
the project's *Custom domains*; Cloudflare writes the proxied CNAME itself. A
zone elsewhere needs that CNAME to `<project>.pages.dev` created by hand.

## Credits

Game design and programming by Henk Boom, Renaud Bédard and Matthew Gallant.
Artwork by Daniel Burton. Music by Ben Abraham.

## License

MIT, see [license.txt](license.txt).
