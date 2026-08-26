set shell := ["bash", "-euo", "pipefail", "-c"]

default: check

# Play it
run:
    cargo run --release --bin pax

# Build the browser version into web/dist
wasm:
    cargo build --release --target wasm32-unknown-unknown --bin pax
    mkdir -p web/dist
    cp target/wasm32-unknown-unknown/release/pax.wasm web/dist/pax.wasm
    cp web/index.html web/favicon.png web/dist/
    # miniquad's JS glue, which the wasm imports; upstream ships no crates.io copy
    [[ -f web/dist/mq_js_bundle.js ]] || \
        curl -sSfL -o web/dist/mq_js_bundle.js \
            https://not-fl3.github.io/miniquad-samples/mq_js_bundle.js
    # load_texture and load_sound fetch these paths relative to the page
    ln -sfn ../../sprites web/dist/sprites
    ln -sfn ../../audio web/dist/audio

# Serve the browser version at http://localhost:8000
serve port='8000': wasm
    python3 -m http.server {{port}} --directory web/dist

# Lay out web/upload for deployment, under a content-hashed prefix
bundle: wasm
    web/bundle.sh

# Serve exactly what gets deployed, at http://localhost:8000
preview port='8000': bundle
    python3 -m http.server {{port}} --directory web/upload

# Publish to Cloudflare Pages (needs CLOUDFLARE_ACCOUNT_ID, wrangler login)
deploy project='pax-britannica' branch='main': bundle
    npx --yes wrangler@4 pages deploy web/upload \
        --project-name {{project}} --branch {{branch}} --commit-dirty=true

# Run the Rust game with no window and print a summary
smoke frames='12000':
    cargo run --release --bin headless -- {{frames}}

# Rust: format, lint, test
check: web-check
    cargo fmt --check
    cargo clippy --all --benches --tests --examples --all-features
    cargo test

# index.html and miniquad's bundle share one global scope; check they can
web-check:
    node test/web_globals.mjs
