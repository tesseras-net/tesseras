#!/bin/sh
set -eu

DEST="/var/www/htdocs/tesseras.net/"
SERVER="${1:-tesseras-website}"
BUILDDIR="public"

cd "$(dirname "$0")"

# build
zola build

# pre-compress static assets for httpd(8)
find "$BUILDDIR" -type f \( \
    -name '*.html' -o \
    -name '*.css' -o \
    -name '*.js' -o \
    -name '*.xml' -o \
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

"$RSYNC" -av --delete --exclude book "$BUILDDIR/" "${SERVER}:${DEST}"
