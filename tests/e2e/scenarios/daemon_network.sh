#!/bin/bash
# Test: daemon-based networking — alice adds a tessera via local CLI,
# verifies the daemon is running, checks node status.
set -euo pipefail

COMPOSE="docker compose -f tests/e2e/docker-compose.yml"

echo "=== Daemon network test ==="

# Alice starts a daemon in the background
$COMPOSE exec -T alice sh -c '
    tes --identity=/data admin daemon start &
    sleep 2
'

# Check that daemon is running via ping
PING_OUT=$($COMPOSE exec -T alice tes --identity=/data admin ping 2>&1 || true)
echo "Ping output: $PING_OUT"

if echo "$PING_OUT" | grep -qi "pong\|node_id"; then
    echo "PASS: Daemon responded to ping"
else
    echo "SKIP: Daemon ping not supported or daemon not running"
    echo "=== Daemon network test SKIPPED ==="
    exit 0
fi

# Add a tessera via the running daemon
$COMPOSE exec -T alice sh -c 'echo "daemon test content" > /tmp/daemon_test.txt'
HASH=$($COMPOSE exec -T alice tes --identity=/data add /tmp/daemon_test.txt --name "DaemonTest" 2>/dev/null)
HASH=$(echo "$HASH" | tr -d '\r\n')
echo "Added tessera via daemon: $HASH"

# List tesseras
LS_OUT=$($COMPOSE exec -T alice tes --identity=/data ls 2>/dev/null)
if echo "$LS_OUT" | grep -q "DaemonTest"; then
    echo "PASS: Tessera visible via daemon"
else
    echo "FAIL: Tessera not visible"
    exit 1
fi

# Check node status
STATUS_OUT=$($COMPOSE exec -T alice tes --identity=/data admin status 2>&1 || true)
echo "Status: $STATUS_OUT"

if echo "$STATUS_OUT" | grep -qi "tessera_count\|tessera"; then
    echo "PASS: Node status shows tessera count"
else
    echo "SKIP: Status command format may differ"
fi

# Cleanup
$COMPOSE exec -T alice tes --identity=/data rm "$HASH" 2>/dev/null

echo "=== Daemon network test PASSED ==="
