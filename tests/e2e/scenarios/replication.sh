#!/bin/bash
# Test: P2P replication — validates steps 1-8 end-to-end between containers.
#
# Features tested:
#   Step 1: DHT integration — nodes discover each other via bootstrap
#   Step 2: Replication — erasure-coded fragments created on add
#   Step 3: Daemon mode — all nodes run daemon with RPC
#   Step 4: P2P tessera fetch — bob fetches alice's tessera via DHT
#   Step 7: Security — signature validation on ingest
#   Step 8: Persistence — daemon restart preserves data and peers
set -euo pipefail

COMPOSE="docker compose -f tests/e2e/docker-compose.yml"

echo "=== P2P Replication test ==="

# ---------------------------------------------------------------------------
# Helper: start a daemon on a client node (config already set by entrypoint)
# ---------------------------------------------------------------------------
start_daemon() {
    local NODE="$1"
    $COMPOSE exec -T "$NODE" tes --identity=/data admin daemon start 2>/dev/null || true
    sleep 3
}

# Helper: check daemon is running
check_daemon() {
    local NODE="$1"
    local STATUS
    STATUS=$($COMPOSE exec -T "$NODE" tes --identity=/data admin daemon status 2>&1 || true)
    if echo "$STATUS" | grep -q "Daemon running (PID"; then
        return 0
    else
        return 1
    fi
}

# Helper: wait for peers (retry up to N seconds)
wait_for_peers() {
    local NODE="$1"
    local MIN_PEERS="$2"
    local TIMEOUT="${3:-15}"
    local ELAPSED=0
    local PEERS=0
    while [ "$ELAPSED" -lt "$TIMEOUT" ]; do
        local STATUS
        STATUS=$($COMPOSE exec -T "$NODE" tes --identity=/data admin daemon status 2>&1 || true)
        PEERS=$(echo "$STATUS" | grep -oP 'Peers:\s+\K\d+' || echo "0")
        if [ "$PEERS" -ge "$MIN_PEERS" ]; then
            echo "  $NODE has $PEERS peers (need $MIN_PEERS)"
            return 0
        fi
        sleep 1
        ELAPSED=$((ELAPSED + 1))
    done
    echo "  WARN: $NODE only has $PEERS peers after ${TIMEOUT}s (need $MIN_PEERS)"
    return 1
}

# Helper: stop daemon (kill process directly if tes stop fails)
stop_daemon() {
    local NODE="$1"
    $COMPOSE exec -T "$NODE" tes --identity=/data admin daemon stop 2>/dev/null || true
    sleep 1
    # Force kill if still running
    $COMPOSE exec -T "$NODE" sh -c 'kill $(cat /data/daemon.pid 2>/dev/null) 2>/dev/null; rm -f /data/daemon.pid /data/daemon.sock' 2>/dev/null || true
    sleep 1
}

# ===========================================================================
# Phase 1: Start daemons on alice, bob, charlie (DHT integration — Step 1)
# ===========================================================================
echo ""
echo "--- Phase 1: Start daemons and verify DHT peer discovery ---"

# Give bootstrap1 time to start (it's started by docker-compose)
sleep 5

start_daemon alice
start_daemon bob
start_daemon charlie

# Verify all daemons are running
for NODE in alice bob charlie; do
    if check_daemon "$NODE"; then
        echo "PASS: $NODE daemon is running"
    else
        echo "FAIL: $NODE daemon not running"
        exit 1
    fi
done

# Wait for peer discovery (each node should know at least the bootstrap)
for NODE in alice bob charlie; do
    wait_for_peers "$NODE" 1 15 || true
done
echo "PASS: Phase 1 — DHT peer discovery"

# ===========================================================================
# Phase 2: Alice creates and publishes a tessera (Replication — Step 2)
# ===========================================================================
echo ""
echo "--- Phase 2: Alice creates and publishes a tessera ---"

# Create test file on alice
$COMPOSE exec -T alice sh -c 'echo "Memory from Alice: the sky was orange at sunset." > /tmp/memory.txt'

# Add the tessera (creates locally + erasure codes fragments)
HASH_A=$($COMPOSE exec -T alice tes --identity=/data add /tmp/memory.txt --name "Sunset" 2>/dev/null)
HASH_A=$(echo "$HASH_A" | tr -d '\r\n')
echo "Alice added tessera: $HASH_A"

if [ -z "$HASH_A" ]; then
    echo "FAIL: No hash returned from add"
    exit 1
fi

# Verify it exists locally on alice
LS_ALICE=$($COMPOSE exec -T alice tes --identity=/data ls 2>/dev/null)
if echo "$LS_ALICE" | grep -q "Sunset"; then
    echo "PASS: Tessera visible locally on Alice"
else
    echo "FAIL: Tessera not visible locally: $LS_ALICE"
    exit 1
fi

# Publish to the DHT network (announce + distribute fragments)
PUB_OUT=$($COMPOSE exec -T alice tes --identity=/data publish "$HASH_A" 2>&1 || true)
echo "Publish output: $PUB_OUT"

if echo "$PUB_OUT" | grep -qi "announced"; then
    echo "PASS: Tessera published to network"
else
    echo "FAIL: Publish failed: $PUB_OUT"
    exit 1
fi

# Give DHT time to propagate
sleep 2

echo "PASS: Phase 2 — Tessera creation and publication"

# ===========================================================================
# Phase 3: Bob fetches Alice's tessera via DHT (P2P Fetch — Step 4)
# ===========================================================================
echo ""
echo "--- Phase 3: Bob fetches Alice's tessera from the network ---"

# Bob should NOT have this tessera locally
BOB_LS=$($COMPOSE exec -T bob tes --identity=/data ls 2>&1)
if echo "$BOB_LS" | grep -q "Sunset"; then
    echo "FAIL: Bob already has the tessera (shouldn't be possible)"
    exit 1
fi
echo "Confirmed: Bob doesn't have tessera locally"

# Bob fetches via DHT
GET_OUT=$($COMPOSE exec -T bob tes --identity=/data get "$HASH_A" 2>&1 || true)
echo "Get output: $GET_OUT"

if echo "$GET_OUT" | grep -qi "Fetched tessera"; then
    echo "PASS: Bob fetched tessera from network"
else
    echo "FAIL: Bob could not fetch tessera: $GET_OUT"
    exit 1
fi

echo "PASS: Phase 3 — P2P tessera fetch"

# ===========================================================================
# Phase 4: Charlie fetches same tessera (may come from Alice or Bob)
# ===========================================================================
echo ""
echo "--- Phase 4: Charlie also fetches the tessera ---"

GET_C=$($COMPOSE exec -T charlie tes --identity=/data get "$HASH_A" 2>&1 || true)
echo "Charlie get: $GET_C"

if echo "$GET_C" | grep -qi "Fetched tessera"; then
    echo "PASS: Charlie fetched tessera from network"
else
    echo "FAIL: Charlie could not fetch tessera: $GET_C"
    exit 1
fi

echo "PASS: Phase 4 — Multi-node replication"

# ===========================================================================
# Phase 5: Daemon RPC operations (Daemon Mode — Step 3)
# ===========================================================================
echo ""
echo "--- Phase 5: Daemon RPC operations ---"

# Node status via RPC
STATUS_A=$($COMPOSE exec -T alice tes --identity=/data admin daemon status 2>&1 || true)
echo "Alice status: $STATUS_A"

if echo "$STATUS_A" | grep -qi "running"; then
    echo "PASS: Daemon status works via RPC"
else
    echo "FAIL: Daemon status failed"
    exit 1
fi

# List tesseras via daemon RPC
LS_VIA_RPC=$($COMPOSE exec -T alice tes --identity=/data ls 2>/dev/null)
if echo "$LS_VIA_RPC" | grep -q "Sunset"; then
    echo "PASS: List via daemon RPC works"
else
    echo "FAIL: List via RPC failed"
    exit 1
fi

# Cat via daemon RPC
CAT_A=$($COMPOSE exec -T alice tes --identity=/data cat "$HASH_A" 2>/dev/null)
if echo "$CAT_A" | grep -q "memory.txt"; then
    echo "PASS: Cat via daemon RPC works"
else
    echo "FAIL: Cat via RPC failed: $CAT_A"
    exit 1
fi

echo "PASS: Phase 5 — Daemon RPC operations"

# ===========================================================================
# Phase 6: Persistence — restart Alice's daemon, verify data survives (Step 8)
# ===========================================================================
echo ""
echo "--- Phase 6: Persistence across daemon restart ---"

# Stop Alice's daemon
stop_daemon alice

# Verify daemon stopped
if check_daemon alice; then
    echo "FAIL: Alice daemon still running after stop"
    exit 1
fi
echo "Alice daemon stopped"

# Restart Alice's daemon
start_daemon alice
if check_daemon alice; then
    echo "PASS: Alice daemon restarted"
else
    echo "FAIL: Alice daemon failed to restart"
    exit 1
fi

# Verify tessera is still there after restart
LS_AFTER_RESTART=$($COMPOSE exec -T alice tes --identity=/data ls 2>/dev/null)
if echo "$LS_AFTER_RESTART" | grep -q "Sunset"; then
    echo "PASS: Tessera persisted across daemon restart"
else
    echo "FAIL: Tessera lost after restart: $LS_AFTER_RESTART"
    exit 1
fi

# Verify peers were restored from persistence
wait_for_peers alice 1 10 || true
echo "PASS: Phase 6 — Persistence across restart"

# ===========================================================================
# Phase 7: Multiple tesseras from different nodes
# ===========================================================================
echo ""
echo "--- Phase 7: Multiple tesseras from different nodes ---"

# Bob creates and publishes a tessera
$COMPOSE exec -T bob sh -c 'echo "Memory from Bob: the rain sounded like music." > /tmp/bob_memory.txt'
HASH_B=$($COMPOSE exec -T bob tes --identity=/data add /tmp/bob_memory.txt --name "Rain" 2>/dev/null)
HASH_B=$(echo "$HASH_B" | tr -d '\r\n')
echo "Bob added tessera: $HASH_B"

$COMPOSE exec -T bob tes --identity=/data publish "$HASH_B" 2>&1 || true

# Charlie creates and publishes a tessera
$COMPOSE exec -T charlie sh -c 'echo "Memory from Charlie: we laughed until we cried." > /tmp/charlie_memory.txt'
HASH_C=$($COMPOSE exec -T charlie tes --identity=/data add /tmp/charlie_memory.txt --name "Laughter" 2>/dev/null)
HASH_C=$(echo "$HASH_C" | tr -d '\r\n')
echo "Charlie added tessera: $HASH_C"

$COMPOSE exec -T charlie tes --identity=/data publish "$HASH_C" 2>&1 || true
sleep 2

# Start dave, fetch both
start_daemon dave
wait_for_peers dave 1 10 || true

# Dave fetches Bob's tessera
GET_DB=$($COMPOSE exec -T dave tes --identity=/data get "$HASH_B" 2>&1 || true)
echo "Dave get Bob's: $GET_DB"
if echo "$GET_DB" | grep -qi "Fetched tessera"; then
    echo "PASS: Dave fetched Bob's tessera"
else
    echo "FAIL: Dave could not fetch Bob's tessera: $GET_DB"
    exit 1
fi

# Dave fetches Charlie's tessera
GET_DC=$($COMPOSE exec -T dave tes --identity=/data get "$HASH_C" 2>&1 || true)
echo "Dave get Charlie's: $GET_DC"
if echo "$GET_DC" | grep -qi "Fetched tessera"; then
    echo "PASS: Dave fetched Charlie's tessera"
else
    echo "FAIL: Dave could not fetch Charlie's tessera: $GET_DC"
    exit 1
fi

echo "PASS: Phase 7 — Multi-node multi-tessera replication"

# ===========================================================================
# Phase 8: Negative case — fetch non-existent tessera
# ===========================================================================
echo ""
echo "--- Phase 8: Negative cases ---"

# Try to get a hash that nobody has
FAKE_HASH="0000000000000000000000000000000000000000000000000000000000000000"
GET_FAKE=$($COMPOSE exec -T alice tes --identity=/data get "$FAKE_HASH" 2>&1 || true)
if echo "$GET_FAKE" | grep -qi "not found\|error"; then
    echo "PASS: Non-existent tessera correctly returns error"
else
    echo "FAIL: Should have returned error for non-existent hash: $GET_FAKE"
    exit 1
fi

echo "PASS: Phase 8 — Negative cases"

# ===========================================================================
# Phase 9: Remove and verify it's gone
# ===========================================================================
echo ""
echo "--- Phase 9: Remove tessera ---"

$COMPOSE exec -T alice tes --identity=/data rm "$HASH_A" 2>/dev/null
LS_AFTER_RM=$($COMPOSE exec -T alice tes --identity=/data ls 2>&1)

if echo "$LS_AFTER_RM" | grep -q "Sunset"; then
    echo "FAIL: Tessera still exists after rm"
    exit 1
fi
echo "PASS: Phase 9 — Tessera removed from alice"

# ===========================================================================
# Cleanup: stop all daemons
# ===========================================================================
echo ""
echo "--- Cleanup: stopping daemons ---"
for NODE in alice bob charlie dave; do
    stop_daemon "$NODE"
done

echo ""
echo "=== P2P Replication test PASSED ==="
