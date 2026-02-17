#!/bin/bash
# Test various file sizes: 1B, 1KB, 1MB
set -euo pipefail

echo "=== File sizes test ==="

for SIZE_DESC in "1B:1" "1KB:1024" "1MB:1048576"; do
    NAME=$(echo "$SIZE_DESC" | cut -d: -f1)
    SIZE=$(echo "$SIZE_DESC" | cut -d: -f2)

    echo "Testing $NAME file..."

    # Create file with specific size
    docker compose -f tests/e2e/docker-compose.yml exec -T alice \
        sh -c "dd if=/dev/urandom of=/tmp/test_${NAME}.bin bs=1 count=${SIZE} 2>/dev/null"

    # Add
    HASH=$(docker compose -f tests/e2e/docker-compose.yml exec -T alice \
        tes --identity=/data add "/tmp/test_${NAME}.bin" --name "Size ${NAME}" 2>/dev/null)
    HASH=$(echo "$HASH" | tr -d '\r\n')
    echo "  Added $NAME: $HASH"

    # Export and verify size
    docker compose -f tests/e2e/docker-compose.yml exec -T alice \
        tes --identity=/data export "$HASH" /tmp/export_sizes 2>/dev/null

    ACTUAL_SIZE=$(docker compose -f tests/e2e/docker-compose.yml exec -T alice \
        sh -c "wc -c < /tmp/export_sizes/size-${NAME,,}/test_${NAME}.bin" 2>/dev/null)
    ACTUAL_SIZE=$(echo "$ACTUAL_SIZE" | tr -d ' \r\n')

    if [ "$ACTUAL_SIZE" = "$SIZE" ]; then
        echo "  PASS: $NAME file size correct ($ACTUAL_SIZE bytes)"
    else
        echo "  FAIL: $NAME expected $SIZE bytes, got $ACTUAL_SIZE"
        exit 1
    fi

    # Cleanup
    docker compose -f tests/e2e/docker-compose.yml exec -T alice \
        tes --identity=/data rm "$HASH" 2>/dev/null
done

echo "=== File sizes test PASSED ==="
