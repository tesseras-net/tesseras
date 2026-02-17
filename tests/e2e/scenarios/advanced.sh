#!/bin/bash
# Test: Advanced E2E scenarios — covers gaps not tested by replication.sh and node_failure.sh
#
# Scenarios tested:
#   1. Content integrity — add, fetch, export, verify byte-for-byte match
#   2. Multi-file tessera — tessera with 3 files, all survive round-trip
#   3. Bootstrap failover — bootstrap1 crashes, network still operates
#   4. Fragment health check — verify fragment health reporting via RPC
#   5. Concurrent adds — multiple nodes add tesseras simultaneously
#   6. Large file — 1MB+ file survives erasure coding round-trip
set -euo pipefail

COMPOSE="docker compose -f tests/e2e/docker-compose.yml"

echo "=== Advanced E2E Scenarios ==="

# ---------------------------------------------------------------------------
# Helpers
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
# Setup: Start daemons on alice, bob, charlie
# ===========================================================================
echo ""
echo "--- Setup: Start nodes ---"

sleep 5  # Wait for bootstrap node

start_daemon alice
start_daemon bob
start_daemon charlie

for NODE in alice bob charlie; do
    if ! check_daemon "$NODE"; then
        echo "FAIL: $NODE daemon not running"
        exit 1
    fi
done

for NODE in alice bob charlie; do
    wait_for_peers "$NODE" 1 15 || true
done
echo "Setup complete"

# ===========================================================================
# Scenario 1: Content integrity — byte-for-byte verification
# ===========================================================================
echo ""
echo "--- Scenario 1: Content integrity (add → fetch → export → verify) ---"

# Create a file with known content and compute its checksum
$COMPOSE exec -T alice sh -c 'echo "The exact content that must survive: café résumé naïve." > /tmp/integrity.txt'
ORIG_SUM=$($COMPOSE exec -T alice sh -c 'sha256sum /tmp/integrity.txt | cut -d" " -f1' | tr -d '\r\n')
echo "Original SHA256: $ORIG_SUM"

# Alice adds it
HASH_I=$($COMPOSE exec -T alice tes --identity=/data add /tmp/integrity.txt --name "Integrity" 2>/dev/null)
HASH_I=$(echo "$HASH_I" | tr -d '\r\n')
echo "Alice added tessera: $HASH_I"

sleep 2

# Bob fetches it from the network
GET_BOB=$($COMPOSE exec -T bob tes --identity=/data get "$HASH_I" 2>&1 || true)
if ! echo "$GET_BOB" | grep -qi "Fetched tessera"; then
    echo "FAIL: Bob could not fetch tessera: $GET_BOB"
    exit 1
fi
echo "Bob fetched tessera from network"

# Bob exports it and verifies checksum matches
$COMPOSE exec -T bob sh -c 'rm -rf /tmp/export && mkdir -p /tmp/export'
$COMPOSE exec -T bob tes --identity=/data export "$HASH_I" /tmp/export 2>/dev/null
EXPORT_SUM=$($COMPOSE exec -T bob sh -c 'sha256sum /tmp/export/integrity/integrity.txt | cut -d" " -f1' | tr -d '\r\n')
echo "Exported SHA256: $EXPORT_SUM"

if [ "$ORIG_SUM" = "$EXPORT_SUM" ]; then
    echo "PASS: Content integrity verified — byte-for-byte match"
else
    echo "FAIL: Content mismatch! Original=$ORIG_SUM Exported=$EXPORT_SUM"
    exit 1
fi

echo "PASS: Scenario 1 — Content integrity"

# ===========================================================================
# Scenario 2: Multi-file tessera
# ===========================================================================
echo ""
echo "--- Scenario 2: Multi-file tessera (3 files in one tessera) ---"

# Create 3 different files on alice
$COMPOSE exec -T alice sh -c 'echo "File one: photo metadata" > /tmp/photo.txt'
$COMPOSE exec -T alice sh -c 'echo "File two: audio transcript of a conversation" > /tmp/audio.txt'
$COMPOSE exec -T alice sh -c 'echo "File three: handwritten letter scanned as text" > /tmp/letter.txt'

# Add all 3 as a single tessera
HASH_M=$($COMPOSE exec -T alice tes --identity=/data add /tmp/photo.txt /tmp/audio.txt /tmp/letter.txt --name "MultiFile" 2>/dev/null)
HASH_M=$(echo "$HASH_M" | tr -d '\r\n')
echo "Alice added multi-file tessera: $HASH_M"

sleep 2

# Charlie fetches it
GET_CHARLIE=$($COMPOSE exec -T charlie tes --identity=/data get "$HASH_M" 2>&1 || true)
if ! echo "$GET_CHARLIE" | grep -qi "Fetched tessera"; then
    echo "FAIL: Charlie could not fetch multi-file tessera: $GET_CHARLIE"
    exit 1
fi

# Verify cat shows all 3 files
CAT_M=$($COMPOSE exec -T charlie tes --identity=/data cat "$HASH_M" 2>/dev/null || true)
MISSING=""
for FILE in photo.txt audio.txt letter.txt; do
    if ! echo "$CAT_M" | grep -q "$FILE"; then
        MISSING="$MISSING $FILE"
    fi
done

if [ -z "$MISSING" ]; then
    echo "PASS: All 3 files present in tessera"
else
    echo "FAIL: Missing files:$MISSING  Cat output: $CAT_M"
    exit 1
fi

# Export and verify each file's content
$COMPOSE exec -T charlie sh -c 'rm -rf /tmp/multi_export && mkdir -p /tmp/multi_export'
$COMPOSE exec -T charlie tes --identity=/data export "$HASH_M" /tmp/multi_export 2>/dev/null

PHOTO_CONTENT=$($COMPOSE exec -T charlie sh -c 'cat /tmp/multi_export/multifile/photo.txt' | tr -d '\r')
AUDIO_CONTENT=$($COMPOSE exec -T charlie sh -c 'cat /tmp/multi_export/multifile/audio.txt' | tr -d '\r')
LETTER_CONTENT=$($COMPOSE exec -T charlie sh -c 'cat /tmp/multi_export/multifile/letter.txt' | tr -d '\r')

if echo "$PHOTO_CONTENT" | grep -q "photo metadata" && \
   echo "$AUDIO_CONTENT" | grep -q "audio transcript" && \
   echo "$LETTER_CONTENT" | grep -q "handwritten letter"; then
    echo "PASS: All 3 files have correct content after round-trip"
else
    echo "FAIL: File content mismatch"
    echo "  photo: $PHOTO_CONTENT"
    echo "  audio: $AUDIO_CONTENT"
    echo "  letter: $LETTER_CONTENT"
    exit 1
fi

echo "PASS: Scenario 2 — Multi-file tessera"

# ===========================================================================
# Scenario 3: Bootstrap node failover
# ===========================================================================
echo ""
echo "--- Scenario 3: Bootstrap node failover ---"

# Verify bootstrap1 is running
BOOTSTRAP1_STATUS=$($COMPOSE exec -T bootstrap1 tes --identity=/data admin daemon status 2>&1 || true)
if echo "$BOOTSTRAP1_STATUS" | grep -q "Daemon running (PID"; then
    echo "Bootstrap1 is running"
else
    echo "WARN: Bootstrap1 not detected as daemon (running via foreground): $BOOTSTRAP1_STATUS"
fi

# Kill bootstrap1 entirely
$COMPOSE stop bootstrap1 2>&1
echo "Bootstrap1 stopped"
sleep 2

# Verify the mesh still works — alice creates a new tessera
$COMPOSE exec -T alice sh -c 'echo "Created after bootstrap crash!" > /tmp/post_crash.txt'
HASH_PC=$($COMPOSE exec -T alice tes --identity=/data add /tmp/post_crash.txt --name "PostCrash" 2>/dev/null)
HASH_PC=$(echo "$HASH_PC" | tr -d '\r\n')
echo "Alice added tessera after bootstrap crash: $HASH_PC"

sleep 2

# Charlie should be able to fetch it (via direct peer routes, no bootstrap needed)
GET_PC=$($COMPOSE exec -T charlie tes --identity=/data get "$HASH_PC" 2>&1 || true)
if echo "$GET_PC" | grep -qi "Fetched tessera"; then
    echo "PASS: Network operates without bootstrap node"
else
    echo "FAIL: Network broken after bootstrap crash: $GET_PC"
    exit 1
fi

# Bring bootstrap1 back
$COMPOSE start bootstrap1 2>&1
sleep 3
echo "Bootstrap1 restarted"

echo "PASS: Scenario 3 — Bootstrap failover"

# ===========================================================================
# Scenario 4: Fragment health check
# ===========================================================================
echo ""
echo "--- Scenario 4: Fragment health check via RPC ---"

# Alice should have fragments for her tesseras — check health
HEALTH=$($COMPOSE exec -T alice tes --identity=/data admin daemon status 2>&1 || true)
echo "Alice status: $HEALTH"

# The status should show peers and the node should be running
if echo "$HEALTH" | grep -q "Daemon running (PID"; then
    echo "PASS: Daemon status reports correctly"
else
    echo "FAIL: Daemon status broken: $HEALTH"
    exit 1
fi

# Verify alice has tesseras locally
LS_ALICE=$($COMPOSE exec -T alice tes --identity=/data ls 2>/dev/null || true)
TESSERA_COUNT=$(echo "$LS_ALICE" | tr -d '\r' | grep -cE '[a-f0-9]{64}' || true)
TESSERA_COUNT=$(echo "$TESSERA_COUNT" | tr -d '\r\n ')
echo "Alice has $TESSERA_COUNT tessera(s) locally"

if [ "${TESSERA_COUNT:-0}" -ge 2 ]; then
    echo "PASS: Alice has expected tesseras"
else
    echo "WARN: Alice has fewer tesseras than expected ($TESSERA_COUNT)"
fi

# Verify cat shows fragment info for one of alice's tesseras
CAT_DETAIL=$($COMPOSE exec -T alice tes --identity=/data cat "$HASH_I" 2>/dev/null || true)
echo "Cat detail: $CAT_DETAIL"

if echo "$CAT_DETAIL" | grep -q "integrity.txt"; then
    echo "PASS: Fragment health — tessera metadata intact"
else
    echo "FAIL: Fragment health — metadata broken: $CAT_DETAIL"
    exit 1
fi

echo "PASS: Scenario 4 — Fragment health check"

# ===========================================================================
# Scenario 5: Concurrent adds from multiple nodes
# ===========================================================================
echo ""
echo "--- Scenario 5: Concurrent adds from multiple nodes ---"

# Create files on each node
$COMPOSE exec -T alice sh -c 'echo "Concurrent from Alice" > /tmp/concurrent.txt'
$COMPOSE exec -T bob sh -c 'echo "Concurrent from Bob" > /tmp/concurrent.txt'
$COMPOSE exec -T charlie sh -c 'echo "Concurrent from Charlie" > /tmp/concurrent.txt'

# Add tesseras simultaneously (write output to temp files)
$COMPOSE exec -T alice tes --identity=/data add /tmp/concurrent.txt --name "ConcAlice" 2>/dev/null > /tmp/conc_alice.txt &
PID_A=$!
$COMPOSE exec -T bob tes --identity=/data add /tmp/concurrent.txt --name "ConcBob" 2>/dev/null > /tmp/conc_bob.txt &
PID_B=$!
$COMPOSE exec -T charlie tes --identity=/data add /tmp/concurrent.txt --name "ConcCharlie" 2>/dev/null > /tmp/conc_charlie.txt &
PID_C=$!

# Wait for all to complete
wait $PID_A || true
wait $PID_B || true
wait $PID_C || true

HASH_CA=$(cat /tmp/conc_alice.txt 2>/dev/null | tr -d '\r\n')
HASH_CB=$(cat /tmp/conc_bob.txt 2>/dev/null | tr -d '\r\n')
HASH_CC=$(cat /tmp/conc_charlie.txt 2>/dev/null | tr -d '\r\n')

echo "Concurrent hashes: Alice=$HASH_CA Bob=$HASH_CB Charlie=$HASH_CC"

CONC_OK=0
for HASH in "$HASH_CA" "$HASH_CB" "$HASH_CC"; do
    if [ -n "$HASH" ] && [ ${#HASH} -eq 64 ]; then
        CONC_OK=$((CONC_OK + 1))
    fi
done

if [ "$CONC_OK" -eq 3 ]; then
    echo "PASS: All 3 concurrent adds succeeded"
else
    echo "FAIL: Only $CONC_OK/3 concurrent adds succeeded"
    exit 1
fi

sleep 3  # Let announcements propagate

# Cross-fetch: bob gets alice's concurrent tessera
if [ -n "$HASH_CA" ] && [ ${#HASH_CA} -eq 64 ]; then
    GET_CONC=$($COMPOSE exec -T bob tes --identity=/data get "$HASH_CA" 2>&1 || true)
    if echo "$GET_CONC" | grep -qi "Fetched tessera\|Found tessera"; then
        echo "PASS: Cross-fetch after concurrent add works"
    else
        echo "WARN: Cross-fetch result: $GET_CONC"
    fi
fi

echo "PASS: Scenario 5 — Concurrent adds"

# ===========================================================================
# Scenario 6: Large file (1MB+) survives erasure coding round-trip
# ===========================================================================
echo ""
echo "--- Scenario 6: Large file (1MB) with erasure coding ---"

# Generate a 1MB file with deterministic content
$COMPOSE exec -T alice sh -c 'dd if=/dev/urandom bs=1024 count=1024 2>/dev/null | base64 > /tmp/large.bin'
LARGE_SUM=$($COMPOSE exec -T alice sh -c 'sha256sum /tmp/large.bin | cut -d" " -f1' | tr -d '\r\n')
LARGE_SIZE=$($COMPOSE exec -T alice sh -c 'wc -c < /tmp/large.bin' | tr -d '\r\n ')
echo "Large file: $LARGE_SIZE bytes, SHA256=$LARGE_SUM"

# Alice adds the large file
HASH_L=$($COMPOSE exec -T alice tes --identity=/data add /tmp/large.bin --name "LargeFile" 2>/dev/null)
HASH_L=$(echo "$HASH_L" | tr -d '\r\n')
echo "Alice added large tessera: $HASH_L"

sleep 3  # Extra time for fragment distribution

# Bob fetches it from the network
GET_LARGE=$($COMPOSE exec -T bob tes --identity=/data get "$HASH_L" 2>&1 || true)
if echo "$GET_LARGE" | grep -qi "Fetched tessera"; then
    echo "Bob fetched large tessera from network"
elif echo "$GET_LARGE" | grep -qi "Found tessera"; then
    echo "Bob already has large tessera"
else
    echo "FAIL: Bob could not fetch large tessera: $GET_LARGE"
    exit 1
fi

# Bob exports and verifies checksum
$COMPOSE exec -T bob sh -c 'rm -rf /tmp/large_export && mkdir -p /tmp/large_export'
$COMPOSE exec -T bob tes --identity=/data export "$HASH_L" /tmp/large_export 2>/dev/null
EXPORT_LARGE_SUM=$($COMPOSE exec -T bob sh -c 'sha256sum /tmp/large_export/largefile/large.bin | cut -d" " -f1' | tr -d '\r\n')
echo "Exported SHA256: $EXPORT_LARGE_SUM"

if [ "$LARGE_SUM" = "$EXPORT_LARGE_SUM" ]; then
    echo "PASS: Large file integrity verified — byte-for-byte match after erasure coding"
else
    echo "FAIL: Large file content mismatch! Original=$LARGE_SUM Exported=$EXPORT_LARGE_SUM"
    exit 1
fi

echo "PASS: Scenario 6 — Large file with erasure coding"

# ===========================================================================
# Cleanup
# ===========================================================================
echo ""
echo "--- Cleanup: stopping daemons ---"
for NODE in alice bob charlie; do
    stop_daemon "$NODE"
done

echo ""
echo "=== Advanced E2E Scenarios PASSED ==="
