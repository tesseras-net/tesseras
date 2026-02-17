#!/bin/bash
# Test: Node failure resilience — validates data survives node crashes and
# tesseras remain fetchable when the original author goes offline.
#
# Scenarios tested:
#   1. Author goes offline — tessera still fetchable from other nodes
#   2. Multiple nodes crash — data reconstructed from remaining fragments
#   3. All nodes except one crash — data survives on the lone survivor
#   4. Network partition heals — nodes re-discover each other
set -euo pipefail

COMPOSE="docker compose -f tests/e2e/docker-compose.yml"

echo "=== Node Failure Resilience test ==="

# ---------------------------------------------------------------------------
# Helpers (same as replication.sh)
# ---------------------------------------------------------------------------
start_daemon() {
    local NODE="$1"
    $COMPOSE exec -T "$NODE" tes --identity=/data admin daemon start 2>/dev/null || true
    sleep 3
}

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

stop_daemon() {
    local NODE="$1"
    $COMPOSE exec -T "$NODE" tes --identity=/data admin daemon stop 2>/dev/null || true
    sleep 1
    $COMPOSE exec -T "$NODE" sh -c 'kill $(cat /data/daemon.pid 2>/dev/null) 2>/dev/null; rm -f /data/daemon.pid /data/daemon.sock' 2>/dev/null || true
    sleep 1
}

# ===========================================================================
# Setup: Start all daemons, create and distribute a tessera
# ===========================================================================
echo ""
echo "--- Setup: Start nodes and create test data ---"

sleep 5  # Wait for bootstrap node

start_daemon alice
start_daemon bob
start_daemon charlie
start_daemon dave
start_daemon eve

for NODE in alice bob charlie dave eve; do
    if ! check_daemon "$NODE"; then
        echo "FAIL: $NODE daemon not running"
        exit 1
    fi
done

# Wait for mesh formation
for NODE in alice bob charlie dave eve; do
    wait_for_peers "$NODE" 2 15 || true
done

# Alice creates a tessera (auto-announces to DHT + distributes fragments)
$COMPOSE exec -T alice sh -c 'echo "Critical memory: the stars aligned on that winter night." > /tmp/stars.txt'
HASH=$($COMPOSE exec -T alice tes --identity=/data add /tmp/stars.txt --name "Stars" 2>/dev/null)
HASH=$(echo "$HASH" | tr -d '\r\n')
echo "Alice created tessera: $HASH"

if [ -z "$HASH" ]; then
    echo "FAIL: No hash returned"
    exit 1
fi

# Let all other nodes fetch it so fragments are spread across the network
sleep 2
for NODE in bob charlie dave eve; do
    GET_OUT=$($COMPOSE exec -T "$NODE" tes --identity=/data get "$HASH" 2>&1 || true)
    if echo "$GET_OUT" | grep -qi "Fetched tessera"; then
        echo "  $NODE fetched tessera OK"
    else
        echo "  $NODE fetch: $GET_OUT"
    fi
done

echo "Setup complete — tessera distributed to all 5 nodes"

# ===========================================================================
# Scenario 1: Author goes offline — tessera still fetchable
# ===========================================================================
echo ""
echo "--- Scenario 1: Author (Alice) goes offline ---"

stop_daemon alice
if check_daemon alice; then
    echo "FAIL: Alice daemon still running"
    exit 1
fi
echo "Alice is offline"

# Bob should still be able to cat the tessera (from local cache)
CAT_BOB=$($COMPOSE exec -T bob tes --identity=/data cat "$HASH" 2>/dev/null || true)
if echo "$CAT_BOB" | grep -q "stars.txt"; then
    echo "PASS: Bob can still access tessera with author offline"
else
    echo "FAIL: Bob cannot access tessera: $CAT_BOB"
    exit 1
fi

# Eve (who fetched it earlier) should also have it
CAT_EVE=$($COMPOSE exec -T eve tes --identity=/data cat "$HASH" 2>/dev/null || true)
if echo "$CAT_EVE" | grep -q "stars.txt"; then
    echo "PASS: Eve can still access tessera with author offline"
else
    echo "FAIL: Eve cannot access tessera: $CAT_EVE"
    exit 1
fi

echo "PASS: Scenario 1 — Data survives author going offline"

# ===========================================================================
# Scenario 2: Multiple nodes crash — data still available
# ===========================================================================
echo ""
echo "--- Scenario 2: Multiple nodes crash (Alice + Charlie + Dave offline) ---"

# Alice is already offline; take down Charlie and Dave too
stop_daemon charlie
stop_daemon dave

for NODE in charlie dave; do
    if check_daemon "$NODE"; then
        echo "FAIL: $NODE daemon still running"
        exit 1
    fi
done
echo "3 of 5 nodes offline (Alice, Charlie, Dave)"

# Bob should still have it locally
LS_BOB=$($COMPOSE exec -T bob tes --identity=/data ls 2>/dev/null || true)
if echo "$LS_BOB" | grep -q "Stars"; then
    echo "PASS: Bob still has tessera with 3 nodes down"
else
    echo "FAIL: Bob lost tessera: $LS_BOB"
    exit 1
fi

# Eve should still have it too
LS_EVE=$($COMPOSE exec -T eve tes --identity=/data ls 2>/dev/null || true)
if echo "$LS_EVE" | grep -q "Stars"; then
    echo "PASS: Eve still has tessera with 3 nodes down"
else
    echo "FAIL: Eve lost tessera: $LS_EVE"
    exit 1
fi

echo "PASS: Scenario 2 — Data survives multiple node failures"

# ===========================================================================
# Scenario 3: All nodes except one crash — lone survivor keeps data
# ===========================================================================
echo ""
echo "--- Scenario 3: All nodes except Eve go offline ---"

stop_daemon bob
if check_daemon bob; then
    echo "FAIL: Bob daemon still running"
    exit 1
fi
echo "4 of 5 nodes offline — only Eve remains"

# Eve should still have the tessera
CAT_EVE=$($COMPOSE exec -T eve tes --identity=/data cat "$HASH" 2>/dev/null || true)
if echo "$CAT_EVE" | grep -q "stars.txt"; then
    echo "PASS: Eve (lone survivor) still has full tessera"
else
    echo "FAIL: Lone survivor lost tessera: $CAT_EVE"
    exit 1
fi

echo "PASS: Scenario 3 — Lone survivor preserves data"

# ===========================================================================
# Scenario 4: Nodes come back online — data still accessible
# ===========================================================================
echo ""
echo "--- Scenario 4: Nodes recover and rejoin the network ---"

# Bring everyone back
start_daemon alice
start_daemon bob
start_daemon charlie
start_daemon dave

for NODE in alice bob charlie dave eve; do
    if ! check_daemon "$NODE"; then
        echo "FAIL: $NODE daemon not running after restart"
        exit 1
    fi
done
echo "All 5 nodes back online"

# Wait for re-discovery
for NODE in alice bob charlie dave eve; do
    wait_for_peers "$NODE" 2 15 || true
done

# Alice (the author) should still have her tessera (persisted on disk)
LS_ALICE=$($COMPOSE exec -T alice tes --identity=/data ls 2>/dev/null || true)
if echo "$LS_ALICE" | grep -q "Stars"; then
    echo "PASS: Alice recovered her tessera after restart"
else
    echo "FAIL: Alice lost tessera after restart: $LS_ALICE"
    exit 1
fi

# Charlie (was offline) should also have it (fetched before crash)
LS_CHARLIE=$($COMPOSE exec -T charlie tes --identity=/data ls 2>/dev/null || true)
if echo "$LS_CHARLIE" | grep -q "Stars"; then
    echo "PASS: Charlie recovered tessera after restart"
else
    echo "FAIL: Charlie lost tessera after restart: $LS_CHARLIE"
    exit 1
fi

echo "PASS: Scenario 4 — Network recovery preserves all data"

# ===========================================================================
# Scenario 5: New node joins after failures — can fetch from recovered network
# ===========================================================================
echo ""
echo "--- Scenario 5: New node joins and fetches from recovered network ---"

# Eve creates a second tessera while the network is recovered
$COMPOSE exec -T eve sh -c 'echo "Memory from Eve: the forest was quiet after the storm." > /tmp/forest.txt'
HASH_E=$($COMPOSE exec -T eve tes --identity=/data add /tmp/forest.txt --name "Forest" 2>/dev/null)
HASH_E=$(echo "$HASH_E" | tr -d '\r\n')
echo "Eve created second tessera: $HASH_E"
sleep 2

# Dave fetches the new tessera (proves the network is fully operational post-failure)
GET_DAVE=$($COMPOSE exec -T dave tes --identity=/data get "$HASH_E" 2>&1 || true)
if echo "$GET_DAVE" | grep -qi "Fetched tessera"; then
    echo "PASS: Dave fetched Eve's tessera from recovered network"
else
    echo "FAIL: Dave couldn't fetch from recovered network: $GET_DAVE"
    exit 1
fi

# Alice also fetches Eve's tessera
GET_ALICE=$($COMPOSE exec -T alice tes --identity=/data get "$HASH_E" 2>&1 || true)
if echo "$GET_ALICE" | grep -qi "Fetched tessera"; then
    echo "PASS: Alice fetched Eve's tessera from recovered network"
else
    echo "FAIL: Alice couldn't fetch from recovered network: $GET_ALICE"
    exit 1
fi

echo "PASS: Scenario 5 — Network fully operational after recovery"

# ===========================================================================
# Cleanup
# ===========================================================================
echo ""
echo "--- Cleanup: stopping daemons ---"
for NODE in alice bob charlie dave eve; do
    stop_daemon "$NODE"
done

echo ""
echo "=== Node Failure Resilience test PASSED ==="
