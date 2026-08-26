#!/usr/bin/env bash
# Everything the game fetches at startup, with byte counts, so the loading
# screen's progress bar can be exact. Sizes are uncompressed, which is what a
# fetch reader counts; Content-Length would be the transfer-encoded size.
set -euo pipefail
cd "$(dirname "$0")/.."

out=web/dist/manifest.json

{
    echo '{'
    printf '  "wasm": %s,\n' "$(stat -Lc %s web/dist/pax.wasm)"
    echo '  "assets": ['
    # -L: web/dist/sprites and web/dist/audio are symlinks into the repo.
    find -L web/dist/sprites web/dist/audio -type f | sort |
        while read -r file; do
            printf '    {"path": "%s", "bytes": %s},\n' \
                "${file#web/dist/}" "$(stat -Lc %s "$file")"
        done | sed '$ s/,$//'
    echo '  ]'
    echo '}'
} >"$out"

echo "$out: $(grep -c '"path"' "$out") assets"
