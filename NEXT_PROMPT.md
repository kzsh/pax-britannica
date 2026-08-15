# Handoff prompt — next session

Paste everything below the line as the opening message.

---

Continue the Lua → Rust port of Pax Britannica. Read `PORTING.md` first — start
with the "Goal — changed" section, which supersedes anything later in that
document that still smells of frame-perfect parity.

## The short version

The bit-exact differential-trace goal is dead. The target is a playable Rust Pax
Britannica whose mechanics match the original: same units, same costs, same AI
behaviour, same one-button dial, same feel. Port by reading the Lua and
transliterating it; do not chase digits.

## Where things stand

Phases 0–3 are done: all of the game logic is in Rust, 187 tests pass, fmt and
clippy are clean. `cargo run --release --bin headless -- 60000` plays a full
game with scripted input and restarts six times, so the mechanics work end to
end.

Phase 4 — `src/render.rs` and `src/bin/pax.rs`, on macroquad — is **written and
has never been run**. The session that wrote it had no display and no libGL, so
it is compiler-checked and nothing more.

Everything except phase 0 is **uncommitted working tree**. Treat `git status` and
`git diff` as read-only context; do not `git checkout`, `git stash`, or otherwise
write with git.

## Your task

1. **Run it.** `cargo run --release --bin pax`. Press `A` (or `F`, `H`, `L`) to
   join, wait out the countdown, hold the button to build.
2. **Fix what is wrong on screen.** PORTING.md's "next concrete step" section
   lists the four most likely failures in order — vertical flip, pie-slice
   winding, the health bar's manual texture wrap, blending. Compare against the
   original if it still builds, or against `screenshot_*.png` in the repo root.
3. Then, if the game looks right: gamepads (needs `gilrs` — ask before adding),
   and phase 5, deleting the Lua and C trees.

## Environment

- Rust lives at `~/.cargo/bin`, not on `PATH`: `export PATH="$HOME/.cargo/bin:$PATH"`, or let direnv run `.envrc`.
- `just` may not be installed. The `justfile` recipes are correct but unverified; run the commands directly.
- `lua5.4` was on `PATH` in earlier sessions and is not any more. If you want the original as a reference, install it; `test/headless.lua` and `test/trace.lua` still work.

```bash
cargo run --release --bin pax
cargo run --release --bin headless -- 12000
cargo test
cargo clippy --all --benches --tests --examples --all-features
```

## Still worth knowing

Behaviour, not parity, so these still apply:

- **Iteration order is spawn order**, and draw order is z-order. `src/world.rs`
  gets this right; don't refactor it into an unordered ECS. The renderer walks
  the same order for the same reason.
- **Deletion is deferred to end of update; everything else is immediate.**
- **`production::draw` and `factory_damage::draw` are gameplay, not pixels** —
  they carry state and take a random draw. They run in `the_game::step`; the
  renderer only reads what they leave behind. Keep that split or `headless`
  stops being a faithful driver.
- Lua's `%` is floor modulo and `//` is floor division; `rem_euclid` exists.
- `f64` everywhere in gameplay maths, not `f32`. The renderer casts to `f32` at
  the boundary and nowhere earlier.
- The deliberate original quirks listed at the end of PORTING.md's phase 3
  section are gameplay, not bugs. Leave them alone.

## Definition of done

`cargo fmt`, clippy clean, `cargo test`. Update `PORTING.md`'s status table and
next-step section. Keep it honest — it is the handoff.

House style: no `unwrap`/panic outside tests, `crate::` over `super::`, no
breadcrumb comments, no new dependencies without asking.
