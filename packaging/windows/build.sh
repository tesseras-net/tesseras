#!/bin/sh
# Build tesseras for Windows via SSH to a Windows machine
#
# Prerequisites:
#   - Windows 10 machine with SSH server enabled (OpenSSH)
#   - Rust toolchain installed: rustup (https://rustup.rs)
#   - Inno Setup installed (for installer): https://jrsoftware.org/isinfo.php
#
# Usage:
#   WIN_HOST=192.168.1.x WIN_USER=user ./build.sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
WIN_HOST="${WIN_HOST:?Set WIN_HOST to your Windows machine IP/hostname}"
WIN_USER="${WIN_USER:-$USER}"
WIN_PORT="${WIN_PORT:-22}"
REMOTE_DIR="C:\\tesseras"
SSH_OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR"

if [ "$WIN_PORT" != "22" ]; then
    SSH_OPTS="$SSH_OPTS -p $WIN_PORT"
fi

SSH_CMD="ssh $SSH_OPTS ${WIN_USER}@${WIN_HOST}"

echo "==> Building tesseras for Windows via ${WIN_HOST}"

# 1. Sync source to Windows machine
echo "==> Syncing source..."
rsync -az --delete \
    -e "ssh $SSH_OPTS" \
    --exclude 'target/' \
    --exclude '.git/' \
    --exclude '.claude/' \
    --exclude 'docs/plans/' \
    "$PROJECT_ROOT/" \
    "${WIN_USER}@${WIN_HOST}:${REMOTE_DIR}/"

# 2. Build on Windows
echo "==> Building on Windows..."
$SSH_CMD "cd ${REMOTE_DIR} && cargo build --release -p tes"

# 3. Create installer if Inno Setup is available
echo "==> Creating installer..."
$SSH_CMD "if exist \"C:\\Program Files (x86)\\Inno Setup 6\\ISCC.exe\" ( \
    \"C:\\Program Files (x86)\\Inno Setup 6\\ISCC.exe\" ${REMOTE_DIR}\\packaging\\windows\\tesseras-installer.iss \
) else ( echo Inno Setup not found - skipping installer )" || true

# 4. Fetch artifacts
OUTDIR="$PROJECT_ROOT/target/packages"
mkdir -p "$OUTDIR"

# Binary
scp $SSH_OPTS "${WIN_USER}@${WIN_HOST}:${REMOTE_DIR}/target/release/tes.exe" \
    "$OUTDIR/tes-windows-amd64.exe"

# Installer (if it was created)
scp $SSH_OPTS "${WIN_USER}@${WIN_HOST}:C:/tesseras-installer/tesseras-setup-*.exe" \
    "$OUTDIR/" 2>/dev/null || echo "  (no installer found, binary-only build)"

echo "==> Done. Artifacts in $OUTDIR/"
