#!/bin/sh
# Submit a dependency check job to SourceHut builds.
# Intended to run via cron on the VPS.
#
# Requirements: curl, jq
# Token: /etc/tesseras/srht-token (chmod 600, scope: builds.sr.ht)
#
# Crontab example (weekly, Monday 08:00 UTC):
#   0 8 * * 1  /usr/local/bin/srht-deps-check.sh
set -eu

SRHT_TOKEN_FILE="${SRHT_TOKEN_FILE:-/etc/tesseras/srht-token}"
SRHT_API="https://builds.sr.ht/query"

if [ ! -f "$SRHT_TOKEN_FILE" ]; then
    echo "error: token file not found: $SRHT_TOKEN_FILE" >&2
    exit 1
fi

TOKEN=$(cat "$SRHT_TOKEN_FILE")

MANIFEST=$(cat <<'MANIFEST'
image: archlinux
packages:
  - rustup
sources:
  - https://git.sr.ht/~ijanc/tesseras
tasks:
  - setup: |
      rustup default stable
      cargo install cargo-audit cargo-outdated cargo-deny
  - audit: |
      cd tesseras
      cargo audit
  - outdated: |
      cd tesseras
      cargo outdated --root-deps-only
  - deny: |
      cd tesseras
      cargo deny check advisories
triggers:
  - action: email
    condition: failure
    to: murilo@ijanc.org
MANIFEST
)

MANIFEST_JSON=$(printf '%s' "$MANIFEST" | jq -Rs .)

QUERY=$(cat <<EOF
mutation {
  submit(
    manifest: ${MANIFEST_JSON},
    tags: ["deps-check"],
    note: "Scheduled dependency check",
    visibility: PRIVATE
  ) {
    id
    status
  }
}
EOF
)

RESPONSE=$(curl -sf -X POST "$SRHT_API" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d "$(printf '%s' "$QUERY" | jq -Rs '{query: .}')")

echo "$RESPONSE" | jq .
