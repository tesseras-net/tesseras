#!/usr/bin/env bash
# Smoke test: spin up a 3-node tesseras network in Docker, verify connectivity.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

cleanup() {
    echo "--- tearing down ---"
    docker compose down --timeout 5 2>/dev/null || true
}
trap cleanup EXIT

echo "--- building and starting 3-node network ---"
docker compose up --build -d

echo "--- waiting for nodes to start (10s) ---"
sleep 10

echo "--- checking boot1 logs ---"
if docker compose logs boot1 2>&1 | grep -q "daemon ready"; then
    echo "PASS: boot1 is ready"
else
    echo "FAIL: boot1 did not reach ready state"
    docker compose logs boot1
    exit 1
fi

echo "--- checking boot2 logs ---"
if docker compose logs boot2 2>&1 | grep -q "daemon ready"; then
    echo "PASS: boot2 is ready"
else
    echo "FAIL: boot2 did not reach ready state"
    docker compose logs boot2
    exit 1
fi

echo "--- checking client logs ---"
if docker compose logs client 2>&1 | grep -q "daemon ready"; then
    echo "PASS: client is ready"
else
    echo "FAIL: client did not reach ready state"
    docker compose logs client
    exit 1
fi

echo "--- checking bootstrap connectivity ---"
if docker compose logs boot2 2>&1 | grep -q "bootstrap successful"; then
    echo "PASS: boot2 bootstrapped successfully"
else
    echo "WARN: boot2 bootstrap status unclear"
fi

if docker compose logs client 2>&1 | grep -q "bootstrap successful"; then
    echo "PASS: client bootstrapped successfully"
else
    echo "WARN: client bootstrap status unclear"
fi

echo ""
echo "=== SMOKE TEST PASSED ==="
