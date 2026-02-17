#!/bin/bash
# Basic test: add public file on alice, verify it exists locally
set -euo pipefail

echo "=== Basic: Add and list tessera ==="

# Create a test file on alice
docker compose -f tests/e2e/docker-compose.yml exec -T alice \
    sh -c 'echo "Hello from Alice" > /tmp/test.txt'

# Add the file
HASH=$(docker compose -f tests/e2e/docker-compose.yml exec -T alice \
    tes --identity=/data add /tmp/test.txt --name "Alice Test" 2>/dev/null)
HASH=$(echo "$HASH" | tr -d '\r\n')
echo "Added tessera: $HASH"

# Verify it appears in ls
LS_OUT=$(docker compose -f tests/e2e/docker-compose.yml exec -T alice \
    tes --identity=/data ls 2>/dev/null)
echo "Listing: $LS_OUT"

if echo "$LS_OUT" | grep -q "Alice Test"; then
    echo "PASS: Tessera listed correctly"
else
    echo "FAIL: Tessera not found in listing"
    exit 1
fi

# Cat the tessera
CAT_OUT=$(docker compose -f tests/e2e/docker-compose.yml exec -T alice \
    tes --identity=/data cat "$HASH" 2>/dev/null)
echo "Cat output: $CAT_OUT"

if echo "$CAT_OUT" | grep -q "test.txt"; then
    echo "PASS: Cat shows file details"
else
    echo "FAIL: Cat output incorrect"
    exit 1
fi

# Export
docker compose -f tests/e2e/docker-compose.yml exec -T alice \
    tes --identity=/data export "$HASH" /tmp/export 2>/dev/null

EXPORTED=$(docker compose -f tests/e2e/docker-compose.yml exec -T alice \
    cat /tmp/export/alice-test/test.txt 2>/dev/null)
EXPORTED=$(echo "$EXPORTED" | tr -d '\r\n')

if [ "$EXPORTED" = "Hello from Alice" ]; then
    echo "PASS: Export content matches"
else
    echo "FAIL: Export content mismatch: '$EXPORTED'"
    exit 1
fi

# Remove
docker compose -f tests/e2e/docker-compose.yml exec -T alice \
    tes --identity=/data rm "$HASH" 2>/dev/null

LS_AFTER=$(docker compose -f tests/e2e/docker-compose.yml exec -T alice \
    tes --identity=/data ls 2>&1)

if echo "$LS_AFTER" | grep -q "No tesseras found"; then
    echo "PASS: Tessera removed successfully"
else
    echo "FAIL: Tessera still exists after rm"
    exit 1
fi

echo "=== Basic test PASSED ==="
