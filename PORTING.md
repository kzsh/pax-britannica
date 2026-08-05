# Pax Britannica → Rust port plan

## Where things stand

Phases 0, 1 and 2 are done. Phase 3 is two-thirds done: clusters 1–4 of 6.

**146 tests pass, `cargo clippy --all --tests` is clean.**

Phase 0 is committed (`438796e Commit phase 0`: `Cargo.toml`, `src/rng.rs`, `src/lib.rs`, the `test/` harness and `traces/golden.txt`). **Everything from phases 1–3 is uncommitted working tree** — all of `src/scripts/`, `src/v2.rs`, `src/collision.rs`, `src/world.rs`, `src/game.rs`, `src/blueprints.rs`, `src/particles.rs`, `src/log.rs`, `src/targeting.rs`, `src/resources.rs`, `src/constants.rs`, `tests/`, `traces/collision.txt`, and the two newer generator scripts in `test/`.

| Area | State |
| --- | --- |
| Golden trace oracle | Done. `traces/golden.txt`, 12,000 frames, reproducible |
| `rng`, `v2`, `collision` | Done, differentially tested against Lua |
| `world` (actor model, phases) | Done, mutation-tested |
| `ship`, `bullet`, `targeting`, `log`, `particles`, collision resolution | Done |
| Fighter / bomber / frigate AI + shooting, `heatseeking_ai`, `blueprints` | Done |
| `resources`, `production`, `factory_ai`, `factory_damage`, both producers | Done, checked against the interpreter |
| `game_flow`, `countdown`, `selector`, `splash`, `fade` | **Not started** |
| `debris`, `fish`, `background_fx` | **Not started** |
| Rust headless runner + trace writer | **Not started** — nothing has been compared to the oracle yet |
| Phase 4 (macroquad platform) | Not started |

There is **no Rust binary yet**; the crate is a library plus tests. `cargo test` is the only thing to run. Rust lives at `~/.cargo/bin` and is not on `PATH`.

### Environment

- `lua5.4` (5.4.7) on `PATH`. Rust 1.97.1 lives at `~/.cargo/bin`; there is an `.envrc` (direnv) that sets it up, but in a shell where direnv has not run, `export PATH="$HOME/.cargo/bin:$PATH"` first.
- `just` is **not installed**, so the `justfile` recipes are written but unverified. Run the commands directly.

### Commands

```bash
cargo test                                    # 119 tests
cargo clippy --all --benches --tests --examples --all-features
lua5.4 test/headless.lua 12000                # Lua smoke test
lua5.4 test/trace.lua 12000 --out traces/golden.txt          # regenerate oracle (~100s)
lua5.4 test/trace.lua 12000 --out /dev/null --dump 500:505   # full state for a frame range
lua5.4 test/particle_draws.lua                # RNG draw counts per particle effect
```

### The next concrete step

Port cluster 5: `game_flow` and the scene state machine (`countdown`, `selector`, `splash`, `fade`). It is the last thing between here and the first trace comparison — factories can build ships now, but nothing yet starts a match.

Then build `src/bin/trace.rs`: mirror `test/harness.lua`'s input schedule exactly (keys `A`/`F`/`H`/`L`, players 1 and 2 join on frames 2–3, then `frame % (120 * i * i)` hold patterns), emit the same one-line-per-frame format as `test/trace.lua`, and diff against `traces/golden.txt`. Expect the first comparison to diverge early; that is the tool working.

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

### Phase 1 — Foundations — **DONE**

**Shipped:**

- `src/v2.rs` — `V2`, `Copy`, `f64`. Operator impls rather than `v2.add`-style calls. `V2::random` draws **angle first**; Lua does not specify argument evaluation order, but PUC-Rio evaluates left to right and that is what produced the golden trace, so it was verified against the interpreter rather than assumed.
- `src/collision.rs` — `Polygon` + `collide`, transliterated from `collision.c` / `test/stubs/collision/native.lua`.
- `tests/collision_differential.rs` + `traces/collision.txt` — 3,000 cases over the game's real shapes plus triangles, a pentagon and a clockwise-wound polygon, compared **bit-exactly**. 39% collide, so the fine phase is genuinely exercised.

**`base.lua` needs no port.** `irandomize` is the only RNG-consuming helper and nothing calls it; the rest (`ifilter`, `ireverse`, `imap`, …) are Rust iterator methods.

**Mutation-checked, with a surprise.** Of the three quirks flagged as "load-bearing" while reading the C, only one actually is: normalising the separating axis fails the differential test on case 8. The `halfwidth_along_axis` floor at `0` and the zero-correction-means-separated guard are both *unreachable* for these shapes — every polygon is built around its own centroid and so contains the origin, and no edge is degenerate. Both are kept for fidelity, and the doc comment now says which is which. Worth remembering the general lesson for phase 3: a subtlety spotted while reading is a hypothesis, and mutation is how you find out.

### Risk to settle before phase 3: transcendental functions

Gameplay calls `sin`, `cos`, `atan2` and `sqrt` constantly. `sqrt` is IEEE-exact and safe. The other three are **not** guaranteed to agree bit-for-bit between implementations. On this machine both Lua and Rust call glibc's libm, so they should match — but that is a property of the box, not of the port, and it means the golden trace is only portable across machines to the extent libm is. If phase 3 shows drift that tracks trig usage, the fix is to pin both sides to one implementation (e.g. the `libm` crate) rather than to loosen the comparison.

### Phase 2 — The framework — **DONE**

`src/world.rs` implements the actor model. The three semantics that gameplay depends on are all in place, and the last two were **mutation-tested** — the framework was deliberately broken to confirm the tests catch it:

- **Spawn-order iteration.** A plain append-only `Vec`.
- **An actor spawned mid-phase runs in that phase.** The phase loop re-reads `order.len()` every iteration. Swapping in a snapshot length fails `an_actor_spawned_mid_phase_runs_in_that_phase`.
- **Death is deferred but checked per script.** An actor killed by its own script skips its remaining scripts. Hoisting the check to per-actor fails `a_script_killing_its_actor_stops_that_actors_later_scripts`.

Dead actors keep their data forever and `ActorId` is never reused, because the Lua holds direct table references and stale reads of `target.dead` must stay valid. Memory grows with total spawns; measure before optimising.

`src/game.rs` holds the singleton services (`rng`, `collision`, `particles`, `log`) as plain fields.

**Design decision:** the owning `player` is hoisted onto `Actor` rather than living on the `ship`/`bullet` scripts as it does in Lua, which removes the `self.ship and self.ship.player or self.bullet.player` fallback. Every collidable has exactly one.

### Phase 3 — Game logic, script by script — **IN PROGRESS (3 of 6 clusters)**

1. ~~`transform`, `sprite` (data only), `collision` + `targeting`~~ **done**
2. ~~`ship`, `bullet`, `heatseeking_ai`, `particles`, `log`, collision resolution~~ **done**
3. ~~`fighter_*`, `bomber_*`, `frigate_*` AI and shooting, `blueprints`~~ **done**
4. ~~`resources`, `production`, `player_production`, `easy_enemy_production`, `factory_ai`, `factory_damage`~~ **done**
5. `game_flow`, `countdown`, `selector`, `splash`, `fade` — the scene state machine — **next**
6. Cosmetics: `debris`, `fish`, `background_fx`

**Two things cluster 4 turned up, both about draws in the *draw* phase.** The
port had assumed `draw` was inert until phase 4; it is not:

- `scripts/factory_damage.lua` takes **one unconditional draw per factory per
  drawn frame** for its flicker opacity. It is otherwise pure cosmetics, and it
  is the sort of file you would defer to phase 4 without thinking. `draw()` in
  `src/scripts/factory_damage.rs` exists solely to take that draw.
- `scripts/production.lua`'s needle and texture scroller are chunk-level locals
  mutated in `draw`, so they are per-frame state the trace captures even though
  nothing renders yet. `production::draw` runs the state machine and leaves the
  GL for phase 4.

The general rule this suggests for clusters 5 and 6: **read every `draw` for RNG
use and for writes to chunk-level locals before writing it off as cosmetic.**

Also ported here: `components/the_one_button.lua`, as `src/the_one_button.rs`.
Polling is split from latching — the driver writes `keys`, and the
`update_setup` phase latches it — so the component needs no window. Cluster 5's
`selector` wants `pressed`, which is already there.

**Validation:** the Rust headless trace matches `traces/golden.txt` frame for frame. This is the gate for the whole project — treat a divergence at frame N as a bug to be localised, not a tolerance to be widened.

Expect the state machine in `scripts/game_flow.lua` and the radial-menu maths in `scripts/production.lua` (210 lines, the densest file) to be where the divergences cluster.

#### Techniques that have earned their keep

**Measure draw counts against the interpreter, don't derive them.** `test/particle_draws.lua` loads the real `components/particles.lua` under a counting RNG and prints the exact number of draws per effect (`explode_big` = 1280, `_mid` = 380, `_small` = 150, `_tiny` = 68, `laser_hit` = 20, `add_bubble` = 2). Those measured numbers are the assertions in `src/particles.rs`. Do the same for anything else whose draw count is non-obvious.

**Lua's `or` short-circuits, so RNG draws are conditional.** Every AI has a line like `if not target or target.dead or math.random() < 0.005 then`. The draw only happens when the earlier tests fail. Each ported AI has a test asserting its exact per-update draw count — fighter with no target: 0; with a live target: 1; frigate acquiring a target: 2 (the fuzz offset). This is the single easiest way to desynchronise the whole run.

**Mutate to check a test can fail.** Phase 1 found that two of three "load-bearing" quirks were unreachable. A subtlety spotted while reading is a hypothesis.

#### Original quirks copied deliberately (do not "fix")

- `explode_tiny` adds its flash to the *small* explosion emitter.
- The inner spark loops in `explode_*` always divide by 20 regardless of their own bound, so smaller explosions throw *slower* sparks, not fewer.
- `bomber_shooting.lua` has the ship-velocity term commented out: bombs do not inherit the bomber's motion.
- `heatseeking_ai.lua`'s `predict()` guards on the missile's own velocity but divides by the *relative* velocity, yielding infinite or negative times that `max(0, ..)` flattens.
- **`player_production` and `easy_enemy_production` run *after* `production` in the blueprint**, so the dial always acts on the previous frame's button state. One frame of input lag, in the original, and it moves every build by a frame.
- The needle's bounce never settles: it gains 0.002 before losing 52.5%, so its velocity converges on −0.000644, not 0, and the fall-back arm runs forever. Harmless — the needle is pinned at zero — but "at rest" is not a state you can test for. Pinned by `the_settled_needle_keeps_bouncing_infinitesimally_forever`.
- `scripts/production.lua`'s four debug-key spawns are **not ported**: `components/debug_keys.lua` gates them on a `--debug` flag nothing passes, so they are dead in every build, including the traced one.
- **`components/targeting.lua` never checks `dead`, and the tag index is only culled at end of update.** So on the frame a target dies, the AI retargets onto the same corpse and only picks a live enemy the frame after. Pinned by `a_fighter_retargets_onto_a_corpse_for_one_frame_then_moves_on`. Expect this to look like a bug when a divergence lands near a kill.

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

## File map

Lua source on the left, its Rust counterpart on the right. Absent means not yet ported.

| Lua | Rust |
| --- | --- |
| `dokidoki/game.lua` | `src/world.rs`, `src/game.rs` |
| `dokidoki/v2.lua` | `src/v2.rs` |
| `dokidoki/collision.lua`, `dokidoki-support/collision.c` | `src/collision.rs` |
| Lua 5.4 `lmathlib.c` | `src/rng.rs` |
| `blueprints.lua` | `src/blueprints.rs` |
| `components/constants.lua` | `src/constants.rs` |
| `components/targeting.lua` | `src/targeting.rs` |
| `components/log.lua` | `src/log.rs` |
| `components/the_one_button.lua` | `src/the_one_button.rs` |
| `components/particles.lua`, `particles.c` | `src/particles.rs` |
| `components/resources.lua` | `src/resources.rs` (names only; loading is phase 4) |
| `components/collision.lua`, `scripts/collision.lua` | `src/scripts/collision.rs` |
| `dokidoki/scripts/transform.lua` | `src/scripts/transform.rs` |
| `dokidoki/scripts/sprite.lua` | `src/scripts/sprite.rs` (data only) |
| `scripts/ship.lua` | `src/scripts/ship.rs` |
| `scripts/bullet.lua` | `src/scripts/bullet.rs` |
| `scripts/{fighter,frigate}_shooting.lua` | `src/scripts/shooting.rs` (shared `Weapon`) + the two callers |
| `scripts/*_ai.lua` | `src/scripts/*_ai.rs` |
| `scripts/resources.lua` | `src/scripts/resources.rs` |
| `scripts/production.lua` | `src/scripts/production.rs` (dial state; GL in phase 4) |
| `scripts/player_production.lua` | `src/scripts/player_production.rs` |
| `scripts/easy_enemy_production.lua` | `src/scripts/easy_enemy_production.rs` |
| `scripts/factory_damage.lua` | `src/scripts/factory_damage.rs` |
| `dokidoki/base.lua` | none needed — Rust iterators |
| `dokidoki/kernel.lua`, `graphics.lua`, `default_font.lua` | phase 4 |

Test-only Lua tooling: `test/harness.lua` (shared driver), `test/trace.lua` (oracle),
`test/rng_vectors.lua`, `test/collision_vectors.lua`, `test/particle_draws.lua`
(all three generate expectations for Rust tests from the real interpreter).
