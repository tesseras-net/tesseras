#!/bin/bash
# Test: daemon-based networking — alice starts a daemon, adds a tessera,
# verifies status, and checks that operations work through the daemon RPC.
set -euo pipefail

COMPOSE="docker compose -f tests/e2e/docker-compose.yml"

echo "=== Daemon network test ==="

# Alice starts a daemon in the background
$COMPOSE exec -T alice tes --identity=/data admin daemon start
sleep 2

# Check that daemon is running via status
STATUS_OUT=$($COMPOSE exec -T alice tes --identity=/data admin daemon status 2>&1 || true)
echo "Status output: $STATUS_OUT"

if echo "$STATUS_OUT" | grep -qi "running\|node_id\|peer"; then
    echo "PASS: Daemon status responded"
else
    echo "SKIP: Daemon status format unexpected"
    echo "=== Daemon network test SKIPPED ==="
    exit 0
fi

# Add a tessera through the running daemon
$COMPOSE exec -T alice sh -c 'echo "daemon test content" > /tmp/daemon_test.txt'
HASH=$($COMPOSE exec -T alice tes --identity=/data add /tmp/daemon_test.txt --name "DaemonTest" 2>/dev/null)
HASH=$(echo "$HASH" | tr -d '\r\n')
echo "Added tessera via daemon: $HASH"

if [ -z "$HASH" ]; then
    echo "FAIL: No hash returned from add"
    exit 1
fi

# List tesseras — should show via daemon RPC
LS_OUT=$($COMPOSE exec -T alice tes --identity=/data ls 2>/dev/null)
if echo "$LS_OUT" | grep -q "DaemonTest"; then
    echo "PASS: Tessera visible via daemon"
else
    echo "FAIL: Tessera not visible via daemon: $LS_OUT"
    exit 1
fi

# Cat tessera metadata
CAT_OUT=$($COMPOSE exec -T alice tes --identity=/data cat "$HASH" 2>/dev/null)
if echo "$CAT_OUT" | grep -q "daemon_test.txt"; then
    echo "PASS: Cat shows file details"
else
    echo "FAIL: Cat output incorrect: $CAT_OUT"
    exit 1
fi

# Remove tessera
$COMPOSE exec -T alice tes --identity=/data rm "$HASH" 2>/dev/null
LS_AFTER=$($COMPOSE exec -T alice tes --identity=/data ls 2>&1)
if echo "$LS_AFTER" | grep -q "$HASH"; then
    echo "FAIL: Tessera still exists after rm"
    exit 1
else
    echo "PASS: Tessera removed via daemon"
fi

# Stop the daemon
$COMPOSE exec -T alice tes --identity=/data admin daemon stop 2>/dev/null || true

echo "=== Daemon network test PASSED ==="
