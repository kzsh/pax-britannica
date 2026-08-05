# Handoff prompt — next session

Paste everything below the line as the opening message.

---

Continue the Lua → Rust port of Pax Britannica. **Read `PORTING.md` first, end to
end** — it is the source of truth for the plan, the architecture, the phase
status and the list of deliberately-copied quirks. This prompt only covers what
that document does not: how to work on it, and where the traps are.

## Where things stand

Phases 0–2 done, phase 3 at 4 of 6 clusters. Cluster 4 (`resources`,
`production`, both producers, `factory_ai`, `factory_damage`,
`components/the_one_button.lua`) landed last session. 146 tests pass, `cargo fmt
--check` and clippy are clean.

**Your task is cluster 5: `game_flow`, `countdown`, `selector`, `splash`,
`fade` — the scene state machine.** After that, cluster 6 is cosmetics
(`debris`, `fish`, `background_fx`), and then the thing everything has been
building towards: `src/bin/trace.rs`, and the first comparison against
`traces/golden.txt`. Do not skip ahead to the trace runner — until the game can
start, there is no run to trace.

Everything from phases 1–3 is **uncommitted working tree**. Only phase 0 is
committed. Do not `git checkout`, `git stash`, or otherwise write with git; treat
`git status`/`git diff` as read-only context. Another agent or the user may have
committed in the meantime.

## Environment

- Rust lives at `~/.cargo/bin`, not on `PATH`. `export PATH="$HOME/.cargo/bin:$PATH"` first, or let direnv run `.envrc`.
- `just` is **not installed**. The `justfile` recipes are correct but unverified; run the commands directly.
- `lua5.4` is on `PATH` and is the oracle. Use it.
- `python3` is broken in this sandbox (no stdlib). Use Lua, shell, or Rust for scratch work.

```bash
cargo test
cargo clippy --all --benches --tests --examples --all-features
lua5.4 test/headless.lua 1200                 # smoke test
lua5.4 test/trace.lua 12000 --out /dev/null --dump 500:505   # full state, frame range
```

## How to work on this

The port's whole value is bit-exact parity with the Lua. Four rules, each of
which has already caught a real bug:

1. **Measure against the interpreter; do not derive.** When a ported script's
   numbers or draw counts are non-obvious, get them out of `lua5.4` and paste
   them into the test as literals. See
   `easy_enemy_production::tests::creation_matches_the_lua_interpreter_bit_for_bit`
   for the pattern, and `test/particle_draws.lua` for the generator form.
2. **Mutation-test the subtle assertions.** Deliberately break the thing the
   test claims to pin and confirm the test fails, then restore. A subtlety
   spotted while reading is a hypothesis. Phase 1 found two of three "load-
   bearing" quirks were unreachable this way.
3. **Count RNG draws, and read `draw` methods as carefully as `update`.** The
   shared `math.random` stream is gameplay state; getting the *number* of draws
   wrong desynchronises everything downstream, and it shows up in the trace a
   frame or two later rather than immediately. Cluster 4 found two draw-phase
   traps — see the note in PORTING.md's phase 3 section. Watch for Lua's `or`
   and `and` short-circuiting, which makes draws conditional.
4. **Chunk-level `local`s are per-instance state, not scratch.** `test/trace.lua`
   walks script upvalues precisely because several scripts keep their real state
   there — `local state = 'init'` in `scripts/game_flow.lua` *is* the scene state
   machine, and it is in the golden trace. Anything mutated in a `draw` counts
   too.

## Specific to cluster 5

- `scripts/game_flow.lua` and `the_game.lua`'s init together decide **actor spawn order**, which is iteration order and therefore z-order and AI targeting order. Get the order of `game.init_component` and the first `game.actors.new` calls literally right.
- The scene machine works through **callbacks** (`fade` calls back on completion, `selector` calls the countdown's reset, the countdown calls `start_game`). Rust has no closures-over-scene-state to lean on here; a small explicit enum of pending actions is likely to translate more honestly than boxed callbacks. Decide deliberately and write down why.
- **Restarting builds a whole new game object.** `kernel.switch_scene(the_game.make())` — the world, the components and the log are all replaced, and the RNG is *not* reseeded (`test/harness.lua` neuters `math.randomseed`). The golden trace covers one scene switch, so this path is exercised.
- `test/harness.lua` is the authority on the input schedule the trace was made with. `src/the_one_button.rs` already has `held`/`pressed`/`released`; the driver writes `keys` and the `update_setup` phase latches.

## Definition of done for the cluster

- Every ported script has tests, including a draw-count assertion where any RNG is involved.
- `cargo fmt`, then clippy clean, then `cargo test`.
- `PORTING.md` updated: the status table, the cluster list, the next concrete step, the file map, and any new quirk worth pinning. Keep it honest — it is the handoff.

Follow the house style in `~/.claude/CLAUDE.md`: no `unwrap`/panic outside tests,
`crate::` over `super::`, no breadcrumb comments, no new dependencies without
asking.
