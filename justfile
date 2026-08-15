set shell := ["bash", "-euo", "pipefail", "-c"]

default: check

# Play it
run:
    cargo run --release --bin pax

# Run the Rust game with no window and print a summary
smoke frames='12000':
    cargo run --release --bin headless -- {{frames}}

# Smoke-test the original Lua with no window
headless frames='1200':
    lua5.4 test/headless.lua {{frames}}

# Record the golden state trace the Rust port is checked against
trace frames='12000':
    mkdir -p traces
    lua5.4 test/trace.lua {{frames}} --out traces/golden.txt

# Check that the trace is reproducible run to run
trace-verify frames='2000':
    mkdir -p traces
    lua5.4 test/trace.lua {{frames}} --out traces/a.txt
    lua5.4 test/trace.lua {{frames}} --out traces/b.txt
    cmp traces/a.txt traces/b.txt && echo "trace is deterministic"
    rm -f traces/a.txt traces/b.txt

# First frame at which two traces disagree, with context
# (diff exits 1 when they differ, which is the interesting case, hence `|| true`)
trace-diff a b:
    diff <(grep -v '^#' {{a}}) <(grep -v '^#' {{b}}) | head -20 || true

# Dump every field of every actor for a frame range, to localise a divergence
trace-dump first last frames='12000':
    lua5.4 test/trace.lua {{frames}} --out /dev/null --dump {{first}}:{{last}}

# Regenerate the Lua PRNG vectors hardcoded in src/rng.rs
rng-vectors:
    lua5.4 test/rng_vectors.lua

# Regenerate the collision differential-test vectors
collision-vectors:
    mkdir -p traces
    lua5.4 test/collision_vectors.lua > traces/collision.txt

# Rust: format, lint, test
check:
    cargo fmt --check
    cargo clippy --all --benches --tests --examples --all-features
    cargo test
