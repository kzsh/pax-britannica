#!/usr/bin/env bash
# Lay out web/dist for upload: everything under a content-hashed prefix, with
# an unhashed index.html pointing at it.
set -euo pipefail
cd "$(dirname "$0")/.."

src=web/dist
out=web/upload

# One hash over the whole payload, so a change to any file moves every URL.
# index.html is excluded: it is never cached, and it carries the hash.
hash=$(
    find -L "$src" -type f ! -name index.html -print0 |
        sort -z | xargs -0 sha256sum | sha256sum | cut -c1-16
)

rm -rf "$out"
mkdir -p "$out/v/$hash"
# -L because sprites/ and audio/ in web/dist are symlinks and wrangler
# uploads real files only.
cp -RL "$src"/. "$out/v/$hash/"
rm "$out/v/$hash/index.html"

# Every relative URL the page and the game reach for -- mq_js_bundle.js,
# pax.wasm, sprites/*.png, audio/music.ogg, the latter two fetched by
# XMLHttpRequest from miniquad's fs_load_file -- resolves against <base>.
sed "s|<head>|<head>\n    <base href=\"/v/$hash/\" />|" "$src/index.html" >"$out/index.html"
cp web/_headers "$out/_headers"

echo "web/upload: /v/$hash"
