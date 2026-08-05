# Pax Britannica → Rust port plan

## What's actually here

| Layer | Files | LoC | Fate |
| --- | --- | --- | --- |
| Game logic (Lua) | `blueprints.lua`, `the_game.lua`, `scripts/*`, `components/*` | ~1,600 | **Hand-port.** This is the game. |
| dokidoki engine (Lua) | `dokidoki/*.lua` | ~1,000 | **Hand-port**, shrinks a lot. |
| Real C | `particles.c`, `collision.c`, `mixer.c`, `log.c` | ~1,200 | **Hand-port** to safe Rust. |
| Binding glue C | `gl.c`, `glu.c`, `luaglfw.c`, `memarray.c`, `minlua.c`, `lua_stb_image.c`, `extra_loaders.h`, `module_to_c.lua` | ~9,000 | **Delete.** Pure Lua↔C plumbing that Rust doesn't need. |
| Vendored libs | `stb_image.c`, `stb_vorbis.c` | ~9,300 | **Delete**, replace with `image` + `lewton`/`symphonia`. |

So ~16k of the 21k lines vanish. The port is really ~2,600 lines of logic plus a new platform layer.

## The one hard problem

dokidoki's actor model is metatable magic that has no Rust equivalent:

- A **script** is a Lua function whose `_ENV` is swapped to a per-actor table. Global assignments inside it (`function update() ... end`, `velocity = v2.zero`) become that actor's fields, chaining to `_G` on miss.
- An **actor** is a table of script-envs keyed by script name, so `self.ship.velocity` and `other.transform.pos` reach across scripts freely, read *and* write, with no declared interface.
- A **blueprint** is a list of scripts plus default field values.
- A **component** (`game.resources`, `game.targeting`, `game.log`) is the same trick applied to a process-wide singleton.

Two properties of `dokidoki/game.lua:270-305` that a naive ECS port will silently break:

1. **Iteration order is spawn order.** `scripts_by_method[m]` is appended to at spawn and culled with an order-preserving `ifilter`. Update order feeds AI targeting and collision resolution; draw order *is* the z-order. Archetype-based ECS iteration (hecs, bevy_ecs) does not give you this.
2. **Deletion is deferred, mutation is not.** `dead`/`paused`/`hidden` are checked per script call inside the frame; culling happens once at end of update. But damage, spawns and field writes land immediately mid-frame.

Getting either wrong produces a game that runs and looks right and is subtly, unfixably not the same game.

## Recommended target architecture

**Port for parity first, idiomatize second.** Concretely:

```rust
// Spawn-ordered arena. Index = stable ActorId. Mirrors Lua exactly.
struct World {
    actors: Vec<Actor>,        // never reordered; compacted at end of update
    by_tag: HashMap<Tag, Vec<ActorId>>,
}

struct Actor {
    blueprint: BlueprintId,
    dead: bool, paused: bool, hidden: bool,
    transform: Option<Transform>,
    ship: Option<Ship>,
    collision: Option<Collision>,
    fighter_ai: Option<FighterAi>,
    // ...one Option per script, ~25 of them
}
```

Yes, 25 `Option` fields is unfashionable. It is also a direct, checkable translation of the Lua table, it preserves spawn order for free, and it makes `actor.ship.velocity` a field access instead of an ECS query dance. Once the headless differential test is green, refactoring to `hecs` (with an explicit ordered `Vec<Entity>` per phase) is a mechanical follow-up you can do with confidence — or skip, because at ~200 actors the flat arena is already faster than the Lua original by a wide margin.

- dokidoki **components** (`resources`, `targeting`, `constants`, `log`, `the_one_button`, …) → fields on a `Game` struct passed as `&mut` alongside the world. No globals, no `lazy_static`.
- **Update/draw phases** (`update_setup`, `update`, `collision_registry`, `collision_check`, `update_cleanup`; `draw_setup`, `draw`, `draw_foreground`, `fade_draw`) → an explicit ordered list of system fns. This is the schedule; keep it literal and readable.
- **Borrow conflicts** (ship A reading ship B's transform while writing its own) → resolve with index-based access and `split_at_mut`, or by copying the small `Copy` structs (`Transform`, `Ship` velocity/facing are a handful of `f64`s). Avoid a deferred command queue for damage — Lua applies it immediately and the queue would change behaviour.
- **`dokidoki/private/will.lua`** (GC finalizers for textures) → `Drop`. Deletes itself.

## Platform stack — decided: `macroquad`

Phases 0–3 below are stack-agnostic; this only affects phase 4.

The existing renderer is immediate-mode `glBegin/glVertex2d` quads with `glPushMatrix` transforms and a letterbox viewport — macroquad's API is nearly a 1:1 target, and it bundles windowing, input, gamepads, textures, and audio in one dependency. Fast to reach parity, and small enough not to dominate a 2010 one-button game.

Rejected: **`bevy`** — its scheduler and change-detection fight the spawn-ordered lockstep model above, and it's a large dependency here. **Hand-rolled `winit` + `wgpu` + `gilrs` + `kira`** — most control, most work; only worth it if the renderer is meant to be a first-class part of the exercise.

macroquad covers PNG decode, Ogg playback and gamepads itself, so no separate `image`/`lewton`/`gilrs` unless its versions prove inadequate. Two things to verify early, since they'd be the reasons to reach for a direct dependency: that its Ogg support streams/loops the 5MB `audio/music.ogg` cleanly, and that its gamepad support sees four simultaneous Xbox pads.

Note the textures are power-of-two-padded with per-sprite `GL_NEAREST`/`GL_LINEAR` and `GL_REPEAT` tweaks (`components/resources.lua:101-137`) — the PoT padding is a 1998-era constraint you can drop, but the filtering and the health-bar `GL_REPEAT` tiling are load-bearing for how the game looks.

## Phases

### Phase 0 — Golden reference — **DONE**

`test/headless.lua` already runs the real game logic against stubs with a fixed seed. Extend it to dump a per-frame trace: actor count by tag, and each actor's `pos`/`facing`/`velocity`/`hit_points` hashed to one line per frame. That file is the oracle for the entire port.

Requires porting Lua 5.4's `math.random` bit-exactly — it's xoshiro256\*\* plus Lua's specific integer-projection for `random(m,n)`, roughly 40 lines of Rust. Do it. Without it you get no differential testing and every gameplay divergence becomes an argument instead of a test failure.

**Shipped:**

- `test/harness.lua` — the input schedule and scene driver, shared by the smoke test and the trace so the two can't drift apart. Hooks `game.actors.new` to record every actor in creation order, and resets that list on scene switch.
- `test/trace.lua` — per live actor in creation order, every number/bool/string/`v2` field on every script, numbers at `%.17g`. Also walks each script method's **upvalues** via `debug.getupvalue`: several scripts keep their real state in chunk-level locals (`local state = 'init'` in `scripts/game_flow.lua` *is* the scene state machine) which are otherwise invisible from outside. One line per frame: FNV-1a-64 hash, live count, tag counts. `--dump FIRST:LAST` prints full per-field state to localise a divergence once the hash says where it is.
- `traces/golden.txt` — 12,000 frames, 1 scene switch, 477K. Byte-identical across repeated runs, verified at 2,000 and at full length.
- `src/rng.rs` — Lua 5.4's `math.random`, bit-exact: xoshiro256\*\*, the 16-draw seeding ritual, the 53-bit float conversion, and the rejection sampling in `project()`. Tested against vectors taken from the `lua5.4` binary itself (`test/rng_vectors.lua`), including the full-range case where the interval width overflows `i64`.
- `test/headless.lua` — rewritten onto the harness; output verified identical to the pre-refactor version at 1,200 and 12,000 frames.

**Note on key ordering:** Lua 5.4 randomises its string hash seed per process, so `pairs()` order over string keys differs between runs. The trace sorts keys for this reason. It also means `components/log.lua`'s `print_stats` output is unordered — cosmetic, but don't mistake it for a divergence.

**Sequencing note:** the trace does not capture the RNG's internal state. A port that diverges only in *how many* draws it takes will show up as a state divergence a frame or two later rather than immediately. If that proves fiddly to debug in phase 3, add a draw counter to both sides.

**Trap found while building this:** `components/particles.lua` looks purely cosmetic, but `explode_big` and friends pull *hundreds* of draws off the shared `math.random` stream per explosion (each `v2.random()` is two draws). Particle emission is therefore gameplay-relevant: the Rust port must reproduce the number and order of those draws exactly, or every frame after the first explosion diverges. The particles themselves are excluded from the trace — they're driven by `test/stubs/particles.lua`, not the real `particles.c`, so their contents aren't authoritative — but the RNG consumption is load-bearing. Port `components/particles.lua` in emission-faithful form early, not last.

### Phase 1 — Foundations

`v2` (`dokidoki/v2.lua`), the bits of `base.lua` that survive, xoshiro256\*\*, and `collision.c` → safe Rust SAT for convex polys. `collision.c` is self-contained and unit-testable against the C version directly; do that.

**Validation:** unit tests, including property tests that the Rust SAT agrees with recorded C outputs.

### Phase 2 — The framework

`World`/`Actor`/blueprints/phase schedule, plus the `Game` singleton struct. No rendering, no window — a `run_headless(frames)` entry point that mirrors `test/headless.lua`'s driver, including its scripted key presses.

**Validation:** it compiles and runs 12,000 frames of nothing.

### Phase 3 — Game logic, script by script

Port in dependency order, checking the trace after each cluster:

1. `transform`, `sprite` (data only, no draw yet), `collision` component + `targeting`
2. `ship`, `bullet`, `heatseeking_ai`
3. `fighter_*`, `bomber_*`, `frigate_*` AI and shooting
4. `resources`, `production`, `player_production`, `easy_enemy_production`, `factory_ai`, `factory_damage`
5. `game_flow`, `countdown`, `selector`, `splash`, `fade` — the scene state machine
6. Cosmetics: `debris`, `fish`, `background_fx` — and `components/particles.lua`, which despite being cosmetic must land in step 2 alongside `ship`, because of the RNG-stream issue noted in phase 0. Emission only; drawing waits for phase 4.

**Validation:** the Rust headless trace matches `traces/golden_12000.txt` frame for frame. This is the gate for the whole project — treat a divergence at frame N as a bug to be localized, not a tolerance to be widened.

Expect the state machine in `scripts/game_flow.lua` and the radial-menu maths in `scripts/production.lua` (210 lines, the densest file) to be where the divergences cluster.

### Phase 4 — Platform

Window, letterboxed 4:3 viewport, sprite/quad renderer, text (`default_font.lua` is an embedded bitmap font), keyboard + gamepad input, Ogg music. Port `particles.c` — it's a flat SoA particle buffer already, so it maps to a `Vec<Particle>` plus one batched draw call, and it should end up *simpler* than the C.

Keep the fixed-60Hz accumulator with `max_frameskip = 6` from `dokidoki/kernel.lua:268-291`. The game's physics constants assume a 1/60 tick; do not switch to variable dt.

**Validation:** play it. Side-by-side screenshots against the Lua build at matched frame counts.

### Phase 5 — Cut over

Delete `dokidoki/`, `dokidoki-support/`, `*.c`, `Makefile`, `compiling.txt`, `extra_loaders.h`. Keep `sprites/`, `audio/`, `media/`, `license.txt`, and the third-party notices for anything still vendored (probably nothing). Rewrite `README.md` build instructions. Add a `justfile`. Keep the headless differential runner as a permanent regression test.

## Risks, in order of how much they'll cost you

1. **Silent gameplay divergence** — mitigated entirely by phase 0. If you skip phase 0 to move faster, this becomes the project.
2. **Float determinism** — Lua uses f64 throughout; use `f64` in Rust, not `f32`, in all gameplay maths. Watch for `x^0.5` vs `sqrt`, and for Lua's `//` (floor div) and `%` (floor mod, *not* Rust's truncating `%`). This one bites reliably.
3. **Order-of-iteration** — covered above; the flat arena makes it a non-issue by construction.
4. **Borrow checker vs. mutual ship interaction** — real friction in phase 3.2, solved with indices. Budget for one refactor here.
5. **Scope creep into "make it a proper ECS/bevy game"** — the port and the redesign are two projects. Ship the port.

## Rough sizing

Phases 0–2 are a few days. Phase 3 is the bulk — 1,600 lines of dense, untyped, mutually-referential Lua, and every line needs its implicit types recovered. Phase 4 is a day or two on macroquad. Call it 2–4 weeks of focused work, and the single biggest lever on that number is whether phase 0 exists.
