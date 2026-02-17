#!/usr/bin/env bash
# End-to-end test against the live tesseras network.
#
# Prerequisites:
#   - tesd running locally (connected to bootstrap nodes)
#   - tes CLI installed and identity initialized
#
# Usage:
#   ./tests/e2e/run.sh                      # full suite
#   ./tests/e2e/run.sh --local-only         # skip network tests (no daemon needed)
#   ./tests/e2e/run.sh --fetch-hash <HASH>  # fetch a specific tessera from the network
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# --- Configuration ---
DATA_DIR="${TESSERAS_DATA_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/tesseras}"
SOCKET="${TESSERAS_SOCKET:-}"  # empty = auto-detect
LOCAL_ONLY=false
FETCH_HASH=""
CREATED_HASH=""
TMPDIR_E2E=""

# --- Parse args ---
while [[ $# -gt 0 ]]; do
    case "$1" in
        --local-only)    LOCAL_ONLY=true; shift ;;
        --fetch-hash)    FETCH_HASH="$2"; shift 2 ;;
        --data-dir)      DATA_DIR="$2"; shift 2 ;;
        --socket)        SOCKET="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: $0 [--local-only] [--fetch-hash HASH] [--data-dir PATH] [--socket PATH]"
            exit 0 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

SOCKET_ARGS=()
if [[ -n "$SOCKET" ]]; then
    SOCKET_ARGS=(--socket "$SOCKET")
fi

# --- Helpers ---
PASS=0
FAIL=0
SKIP=0
TOTAL=0

cleanup() {
    if [[ -n "$TMPDIR_E2E" && -d "$TMPDIR_E2E" ]]; then
        rm -rf "$TMPDIR_E2E"
    fi
    echo ""
    echo "========================================"
    echo "  Results: $PASS passed, $FAIL failed, $SKIP skipped (of $TOTAL)"
    echo "========================================"
    if [[ $FAIL -gt 0 ]]; then
        exit 1
    fi
}
trap cleanup EXIT

pass() {
    PASS=$((PASS + 1))
    TOTAL=$((TOTAL + 1))
    echo "  PASS  $1"
}

fail() {
    FAIL=$((FAIL + 1))
    TOTAL=$((TOTAL + 1))
    echo "  FAIL  $1"
    if [[ -n "${2:-}" ]]; then
        echo "        $2"
    fi
}

skip() {
    SKIP=$((SKIP + 1))
    TOTAL=$((TOTAL + 1))
    echo "  SKIP  $1"
}

section() {
    echo ""
    echo "--- $1 ---"
}

# --- Create temp dir for test content ---
TMPDIR_E2E="$(mktemp -d /tmp/tesseras-e2e.XXXXXX)"

# ==========================================================================
# Phase 1: Pre-flight checks
# ==========================================================================
section "Pre-flight checks"

# Check tes CLI is available
if command -v tes &>/dev/null; then
    TES_VERSION=$(tes --version 2>&1 || true)
    pass "tes CLI found: $TES_VERSION"
else
    fail "tes CLI not found in PATH"
    exit 1
fi

# Check data dir exists and has identity
if [[ -f "$DATA_DIR/identity/node.ed25519.pub" ]] || [[ -f "$DATA_DIR/identity/signing.ed25519.pub" ]]; then
    pass "identity exists at $DATA_DIR"
else
    fail "no identity at $DATA_DIR — run 'tes identity init' first"
    exit 1
fi

# Check database exists
if [[ -f "$DATA_DIR/db/tesseras.db" ]]; then
    pass "database exists"
else
    fail "database not found at $DATA_DIR/db/tesseras.db"
    exit 1
fi

# Check daemon (unless local-only)
DAEMON_OK=false
if [[ "$LOCAL_ONLY" == "false" ]]; then
    if tes net peers "${SOCKET_ARGS[@]}" &>/dev/null; then
        DAEMON_OK=true
        pass "daemon is reachable"
    else
        fail "daemon not reachable — network tests will be skipped" \
             "Start tesd or use --local-only"
        LOCAL_ONLY=true
    fi
else
    skip "daemon check (--local-only)"
fi

# ==========================================================================
# Phase 2: Tessera creation
# ==========================================================================
section "Tessera creation"

# Create sample content directory
mkdir -p "$TMPDIR_E2E/sample"
echo "This is an end-to-end test memory created at $(date -Iseconds)." > "$TMPDIR_E2E/sample/memory.txt"

# Create a minimal JPEG (smallest valid JFIF — 107 bytes)
printf '\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00' > "$TMPDIR_E2E/sample/test.jpg"
printf '\xff\xdb\x00\x43\x00\x08\x06\x06\x07\x06\x05\x08\x07\x07\x07\x09\x09' >> "$TMPDIR_E2E/sample/test.jpg"
printf '\x08\x0a\x0c\x14\x0d\x0c\x0b\x0b\x0c\x19\x12\x13\x0f\x14\x1d\x1a' >> "$TMPDIR_E2E/sample/test.jpg"
printf '\x1f\x1e\x1d\x1a\x1c\x1c\x20\x24\x2e\x27\x20\x22\x2c\x23\x1c\x1c' >> "$TMPDIR_E2E/sample/test.jpg"
printf '\x28\x37\x29\x2c\x30\x31\x34\x34\x34\x1f\x27\x39\x3d\x38\x32\x3c' >> "$TMPDIR_E2E/sample/test.jpg"
printf '\x2e\x33\x34\x32' >> "$TMPDIR_E2E/sample/test.jpg"
printf '\xff\xc0\x00\x0b\x08\x00\x01\x00\x01\x01\x01\x11\x00' >> "$TMPDIR_E2E/sample/test.jpg"
printf '\xff\xc4\x00\x1f\x00\x00\x01\x05\x01\x01\x01\x01\x01\x01\x00\x00' >> "$TMPDIR_E2E/sample/test.jpg"
printf '\x00\x00\x00\x00\x00\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b' >> "$TMPDIR_E2E/sample/test.jpg"
printf '\xff\xda\x00\x08\x01\x01\x00\x00\x3f\x00\x7b\x40' >> "$TMPDIR_E2E/sample/test.jpg"
printf '\xff\xd9' >> "$TMPDIR_E2E/sample/test.jpg"

# Test: dry run
DRY_OUTPUT=$(tes create "$TMPDIR_E2E/sample" --dry-run --data-dir "$DATA_DIR" -n 2>&1 || true)
if echo "$DRY_OUTPUT" | grep -qi "dry run\|would be included"; then
    pass "create --dry-run works"
else
    fail "create --dry-run unexpected output" "$DRY_OUTPUT"
fi

# Test: actual create
CREATE_OUTPUT=$(tes create "$TMPDIR_E2E/sample" --data-dir "$DATA_DIR" -n \
    --no-publish --visibility public --tags "e2e,test" --language "en" 2>&1)
# Try both old format ("Created tessera: HASH") and new format ("Hash:  HASH")
CREATED_HASH=$(echo "$CREATE_OUTPUT" | grep -oP 'Created tessera: \K\S+' || true)
if [[ -z "$CREATED_HASH" ]]; then
    CREATED_HASH=$(echo "$CREATE_OUTPUT" | grep -oP 'Hash:\s+\K\S+' || true)
fi

if [[ -n "$CREATED_HASH" ]]; then
    pass "tessera created: $CREATED_HASH"
else
    fail "could not create tessera" "$CREATE_OUTPUT"
    exit 1
fi

# ==========================================================================
# Phase 3: Local operations
# ==========================================================================
section "Local operations"

# Test: list
LIST_OUTPUT=$(tes list --data-dir "$DATA_DIR" 2>&1)
if echo "$LIST_OUTPUT" | grep -q "${CREATED_HASH:0:10}"; then
    pass "tessera appears in list"
else
    fail "tessera not found in list" "$LIST_OUTPUT"
fi

# Test: show
SHOW_OUTPUT=$(tes show "$CREATED_HASH" --data-dir "$DATA_DIR" 2>&1)
if echo "$SHOW_OUTPUT" | grep -qi "public\|memory\|file"; then
    pass "show displays tessera details"
else
    fail "show output unexpected" "$SHOW_OUTPUT"
fi

# Test: show --json
JSON_OUTPUT=$(tes show "$CREATED_HASH" --json --data-dir "$DATA_DIR" 2>&1)
if echo "$JSON_OUTPUT" | grep -q '"hash"'; then
    pass "show --json returns valid JSON"
else
    fail "show --json unexpected output" "$JSON_OUTPUT"
fi

# Test: verify
VERIFY_OUTPUT=$(tes verify "$CREATED_HASH" --data-dir "$DATA_DIR" 2>&1)
VERIFY_EXIT=$?
if [[ $VERIFY_EXIT -eq 0 ]] && echo "$VERIFY_OUTPUT" | grep -qi "pass\|valid\|ok"; then
    pass "verify passed (exit=$VERIFY_EXIT)"
else
    fail "verify failed (exit=$VERIFY_EXIT)" "$VERIFY_OUTPUT"
fi

# Test: export
EXPORT_DEST="$TMPDIR_E2E/export"
mkdir -p "$EXPORT_DEST"
EXPORT_OUTPUT=$(tes export "$CREATED_HASH" "$EXPORT_DEST" --data-dir "$DATA_DIR" 2>&1)
if [[ -d "$EXPORT_DEST" ]] && ls "$EXPORT_DEST"/*/ &>/dev/null 2>&1; then
    pass "export created directory"
else
    # Check if files exist directly in EXPORT_DEST
    if [[ -n "$(ls -A "$EXPORT_DEST" 2>/dev/null)" ]]; then
        pass "export created files"
    else
        fail "export produced no output" "$EXPORT_OUTPUT"
    fi
fi

# Test: prefix resolution (use first 8 chars)
PREFIX="${CREATED_HASH:0:8}"
PREFIX_OUTPUT=$(tes show "$PREFIX" --data-dir "$DATA_DIR" 2>&1)
if echo "$PREFIX_OUTPUT" | grep -qi "public\|memory\|file"; then
    pass "hash prefix resolution works ($PREFIX)"
else
    fail "prefix resolution failed for $PREFIX" "$PREFIX_OUTPUT"
fi

# ==========================================================================
# Phase 4: Network operations (requires daemon)
# ==========================================================================
section "Network operations"

if [[ "$LOCAL_ONLY" == "true" ]]; then
    skip "net peers (local-only mode)"
    skip "net publish (local-only mode)"
    skip "net status (local-only mode)"
    skip "net fetch (local-only mode)"
else
    # Test: peers
    PEERS_OUTPUT=$(tes net peers "${SOCKET_ARGS[@]}" 2>&1)
    PEER_COUNT=$(echo "$PEERS_OUTPUT" | grep -oP '\d+ peer' | grep -oP '\d+' || echo "0")
    if [[ "$PEER_COUNT" -gt 0 ]]; then
        pass "peers: $PEER_COUNT peer(s) in routing table"
    else
        if echo "$PEERS_OUTPUT" | grep -qi "no peers\|0 peer"; then
            fail "no peers in routing table — DHT may not have bootstrapped"
        else
            pass "peers command ran (output: $(echo "$PEERS_OUTPUT" | head -3))"
        fi
    fi

    # Test: publish
    PUBLISH_OUTPUT=$(tes net publish "$CREATED_HASH" --data-dir "$DATA_DIR" "${SOCKET_ARGS[@]}" 2>&1)
    if echo "$PUBLISH_OUTPUT" | grep -qi "publish\|fragment\|distribut"; then
        pass "publish succeeded"
        echo "        $PUBLISH_OUTPUT"
    else
        fail "publish unexpected output" "$PUBLISH_OUTPUT"
    fi

    # Wait a moment for distribution to start
    sleep 2

    # Test: status
    STATUS_OUTPUT=$(tes net status "$CREATED_HASH" --data-dir "$DATA_DIR" "${SOCKET_ARGS[@]}" 2>&1)
    if echo "$STATUS_OUTPUT" | grep -qi "fragment\|replic\|healthy\|publish\|local\|state"; then
        pass "status reports replication state"
        echo "        $(echo "$STATUS_OUTPUT" | head -5)"
    else
        fail "status unexpected output" "$STATUS_OUTPUT"
    fi

    # Test: fetch (from network — use a specific hash if provided)
    if [[ -n "$FETCH_HASH" ]]; then
        FETCH_OUTPUT=$(tes net fetch "$FETCH_HASH" --data-dir "$DATA_DIR" "${SOCKET_ARGS[@]}" 2>&1)
        if echo "$FETCH_OUTPUT" | grep -qi "fetch\|memor\|byte"; then
            pass "fetch from network succeeded for $FETCH_HASH"
        else
            fail "fetch failed for $FETCH_HASH" "$FETCH_OUTPUT"
        fi
    else
        skip "net fetch (no --fetch-hash provided; publish from another node first)"
    fi
fi

# ==========================================================================
# Phase 5: Simplified create flow (auto-init, auto-publish)
# ==========================================================================
section "Simplified create flow"

SIMPLE_DIR="$(mktemp -d /tmp/tesseras-e2e-simple.XXXXXX)"
SIMPLE_DATA="$SIMPLE_DIR/data"
SIMPLE_CONTENT="$SIMPLE_DIR/content"
mkdir -p "$SIMPLE_CONTENT"
echo "Simplified flow test at $(date -Iseconds)" > "$SIMPLE_CONTENT/note.txt"

# Test: create with auto-init (fresh data dir, no prior init)
SIMPLE_OUTPUT=$(tes create "$SIMPLE_CONTENT" -n --data-dir "$SIMPLE_DATA" --no-publish 2>&1)
if echo "$SIMPLE_OUTPUT" | grep -qi "preserved\|created\|Hash:"; then
    pass "simplified create auto-initializes"
else
    fail "simplified create failed" "$SIMPLE_OUTPUT"
fi

# Verify identity was created
if [[ -f "$SIMPLE_DATA/identity/signing.ed25519.pub" ]] || [[ -f "$SIMPLE_DATA/identity/node.ed25519.pub" ]]; then
    pass "auto-init created identity"
else
    fail "auto-init did not create identity"
fi

# Verify database was created
if [[ -f "$SIMPLE_DATA/db/tesseras.db" ]]; then
    pass "auto-init created database"
else
    fail "auto-init did not create database"
fi

rm -rf "$SIMPLE_DIR"

# ==========================================================================
# Phase 6: Peer stability test
# ==========================================================================
section "Peer stability"

if [[ "$LOCAL_ONLY" == "false" && "$DAEMON_OK" == "true" ]]; then
    PEER_COUNTS=()
    for i in $(seq 1 10); do
        COUNT=$(tes net peers "${SOCKET_ARGS[@]}" 2>/dev/null | grep -oP '\d+ peer' | grep -oP '\d+' || echo "0")
        PEER_COUNTS+=($COUNT)
        sleep 6
    done

    # Calculate min/max/variance
    MIN=${PEER_COUNTS[0]}
    MAX=${PEER_COUNTS[0]}
    ZEROS=0
    for c in "${PEER_COUNTS[@]}"; do
        ((c < MIN)) && MIN=$c
        ((c > MAX)) && MAX=$c
        ((c == 0)) && ZEROS=$((ZEROS + 1))
    done
    VARIANCE=$((MAX - MIN))

    if ((ZEROS > 2)); then
        fail "peer count dropped to 0 too often ($ZEROS times in 10 checks)"
    elif ((VARIANCE > 3)); then
        fail "peer count oscillation: min=$MIN max=$MAX variance=$VARIANCE (counts: ${PEER_COUNTS[*]})"
    else
        pass "peer stability OK: min=$MIN max=$MAX variance=$VARIANCE (counts: ${PEER_COUNTS[*]})"
    fi
else
    skip "peer stability (local-only or no daemon)"
fi

# ==========================================================================
# Phase 7: Cross-node test instructions
# ==========================================================================
section "Cross-node verification"

if [[ "$LOCAL_ONLY" == "false" && -n "$CREATED_HASH" ]]; then
    echo ""
    echo "  To complete the cross-node test, run on another node (m0x, hetzner, etc.):"
    echo ""
    echo "    tes net fetch $CREATED_HASH"
    echo "    tes verify $CREATED_HASH"
    echo "    tes show $CREATED_HASH"
    echo ""
    echo "  Then fetch something back from that node:"
    echo ""
    echo "    # On the remote node, create + publish a tessera"
    echo "    # Then on this machine:"
    echo "    ./tests/e2e/run.sh --fetch-hash <REMOTE_HASH>"
    echo ""
fi
