#!/bin/bash
set -e

COMPOSE="docker compose -f tests/e2e/docker-compose.yml"

echo "Building and starting containers..."
$COMPOSE up -d --build
sleep 5  # wait for containers to start

FAILED=0

for scenario in tests/e2e/scenarios/*.sh; do
    echo ""
    echo "=== Running $(basename "$scenario") ==="
    if bash "$scenario"; then
        echo "=== $(basename "$scenario") PASSED ==="
    else
        echo "=== $(basename "$scenario") FAILED ==="
        FAILED=1
    fi
done

echo ""
echo "Stopping containers..."
$COMPOSE down -v

if [ "$FAILED" -eq 0 ]; then
    echo ""
    echo "All E2E tests passed."
else
    echo ""
    echo "Some E2E tests FAILED."
    exit 1
fi
