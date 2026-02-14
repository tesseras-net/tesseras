#!/bin/sh
set -eu

DEST="/var/www/htdocs/tesseras.net/book/"
SERVER="${1:-tesseras-website}"
STAGEDIR="_site"

cd "$(dirname "$0")"

# build both languages
mdbook build en
mdbook build pt-br

# assemble staging directory
rm -rf "$STAGEDIR"
mkdir -p "$STAGEDIR"
cp -a en/book "$STAGEDIR/en"
cp -a pt-br/book "$STAGEDIR/pt-br"

# pre-compress static assets for httpd(8)
find "$STAGEDIR" -type f \( \
    -name '*.html' -o \
    -name '*.css' -o \
    -name '*.js' -o \
    -name '*.svg' -o \
    -name '*.json' -o \
    -name '*.txt' \
    \) -exec gzip -fk9 {} +

# sync — prefer openrsync, fall back to rsync
if command -v openrsync >/dev/null 2>&1; then
    RSYNC=openrsync
else
    RSYNC=rsync
fi

"$RSYNC" -av --delete "$STAGEDIR/" "${SERVER}:${DEST}"

# clean up
rm -rf "$STAGEDIR"
