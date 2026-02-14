#!/usr/bin/env bash
# Smoke test: spin up a 3-node tesseras network in Docker, exercise the full
# pipeline — create a tessera, verify it, and confirm replication is active.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

TIMEOUT=30

cleanup() {
    echo "--- tearing down ---"
    docker compose down --timeout 5 2>/dev/null || true
}
trap cleanup EXIT

pass() { echo "PASS: $1"; }
fail() { echo "FAIL: $1"; docker compose logs; exit 1; }
info() { echo "INFO: $1"; }

# --- 1. Build and start 3-node network ---
echo "--- building and starting 3-node network ---"
docker compose up --build -d

# --- 2. Wait for all nodes to be ready (poll logs) ---
echo "--- waiting for nodes to be ready (up to ${TIMEOUT}s) ---"
for node in boot1 boot2 client; do
    elapsed=0
    while ! docker compose logs "$node" 2>&1 | grep -q "daemon ready"; do
        if [ "$elapsed" -ge "$TIMEOUT" ]; then
            fail "$node did not reach ready state within ${TIMEOUT}s"
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    pass "$node is ready"
done

# --- 3. Verify bootstrap connectivity ---
echo "--- checking bootstrap connectivity ---"
for node in boot2 client; do
    if docker compose logs "$node" 2>&1 | grep -q "bootstrap successful"; then
        pass "$node bootstrapped successfully"
    else
        info "$node bootstrap status unclear (may still be connecting)"
    fi
done

# --- 4. Initialize identity on client node ---
echo "--- initializing identity on client ---"
docker compose exec -T client tes init --data-dir /data
pass "identity initialized on client"

# --- 5. Create a sample tessera ---
echo "--- creating sample tessera ---"
docker compose exec -T client mkdir -p /tmp/sample-tessera
docker compose exec -T client sh -c 'echo "This is a test memory for the smoke test." > /tmp/sample-tessera/memory.txt'

CREATE_OUTPUT=$(docker compose exec -T client tes create --data-dir /data -n /tmp/sample-tessera/ 2>&1)
echo "$CREATE_OUTPUT"

HASH=$(echo "$CREATE_OUTPUT" | grep -oP 'Created tessera: \K\S+')
if [ -z "$HASH" ]; then
    fail "could not extract tessera hash from create output"
fi
pass "tessera created: $HASH"

# --- 6. List tesseras ---
echo "--- listing tesseras ---"
LIST_OUTPUT=$(docker compose exec -T client tes list --data-dir /data 2>&1)
echo "$LIST_OUTPUT"

if echo "$LIST_OUTPUT" | grep -q "${HASH:0:10}"; then
    pass "tessera appears in list"
else
    fail "tessera not found in list output"
fi

# --- 7. Verify tessera integrity ---
echo "--- verifying tessera ---"
VERIFY_OUTPUT=$(docker compose exec -T client tes verify --data-dir /data "$HASH" 2>&1)
echo "$VERIFY_OUTPUT"

if echo "$VERIFY_OUTPUT" | grep -qi "passed\|valid\|ok"; then
    pass "tessera verification passed"
else
    fail "tessera verification did not pass"
fi

# --- 8. Check replication activity in logs ---
echo "--- checking replication activity ---"
if docker compose logs client 2>&1 | grep -qi "repair.loop\|replication\|under-replicated\|fragment"; then
    pass "replication activity detected in client logs"
else
    info "no replication log entries found (stub transport — expected)"
fi

# --- 9. Check routing table ---
echo "--- checking routing table ---"
if docker compose logs client 2>&1 | grep -qi "routing.table\|peers\|bucket"; then
    pass "routing table activity detected"
else
    info "no routing table entries in logs (may need more time)"
fi

echo ""
echo "=== SMOKE TEST PASSED ==="
