#!/bin/sh
# Cross-build tesseras for OpenBSD using QEMU
#
# Prerequisites:
#   - QEMU installed: pacman -S qemu-full (Arch) or apt install qemu-system-x86 (Debian)
#   - An OpenBSD QEMU image with Rust toolchain
#   - SSH access configured (key-based, port forwarded to host)
#
# Setup (one-time):
#   1. Download OpenBSD install ISO: https://cdn.openbsd.org/pub/OpenBSD/7.6/amd64/install76.iso
#   2. Create disk image: qemu-img create -f qcow2 openbsd.qcow2 20G
#   3. Install OpenBSD in QEMU:
#      qemu-system-x86_64 -m 2G -smp 2 -hda openbsd.qcow2 \
#        -cdrom install76.iso -boot d -net nic -net user,hostfwd=tcp::2222-:22
#   4. Inside OpenBSD, install Rust: pkg_add rust
#   5. Snapshot the image for fast reuse
#
# Usage:
#   OPENBSD_HOST=localhost OPENBSD_PORT=2222 ./qemu-build.sh
#   or with a remote OpenBSD machine:
#   OPENBSD_HOST=m0x.example.com ./qemu-build.sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
OPENBSD_HOST="${OPENBSD_HOST:-localhost}"
OPENBSD_PORT="${OPENBSD_PORT:-2222}"
OPENBSD_USER="${OPENBSD_USER:-root}"
REMOTE_DIR="/tmp/tesseras-build"
SSH_OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR"

if [ "$OPENBSD_PORT" != "22" ]; then
    SSH_OPTS="$SSH_OPTS -p $OPENBSD_PORT"
    SCP_OPTS="-P $OPENBSD_PORT"
else
    SCP_OPTS=""
fi

SSH_CMD="ssh $SSH_OPTS ${OPENBSD_USER}@${OPENBSD_HOST}"
SCP_CMD="scp $SSH_OPTS $SCP_OPTS"

echo "==> Cross-building tesseras for OpenBSD via ${OPENBSD_HOST}:${OPENBSD_PORT}"

# 1. Sync source to OpenBSD machine (exclude target/ and .git/)
echo "==> Syncing source..."
$SSH_CMD "rm -rf $REMOTE_DIR && mkdir -p $REMOTE_DIR"
rsync -az --delete \
    -e "ssh $SSH_OPTS" \
    --exclude 'target/' \
    --exclude '.git/' \
    --exclude '.claude/' \
    --exclude 'docs/plans/' \
    "$PROJECT_ROOT/" \
    "${OPENBSD_USER}@${OPENBSD_HOST}:${REMOTE_DIR}/"

# 2. Build on OpenBSD
echo "==> Building on OpenBSD..."
$SSH_CMD "cd $REMOTE_DIR && cargo build --release -p tes"

# 3. Run the package creation script
echo "==> Creating OpenBSD package..."
$SSH_CMD "cd $REMOTE_DIR && sh packaging/openbsd/create-package.sh"

# 4. Fetch the package back
OUTDIR="$PROJECT_ROOT/target/packages"
mkdir -p "$OUTDIR"
VERSION=$($SSH_CMD "sed -n 's/^version = \"\(.*\)\"/\1/p' $REMOTE_DIR/tes/Cargo.toml | head -1")
$SCP_CMD "${OPENBSD_USER}@${OPENBSD_HOST}:${REMOTE_DIR}/target/packages/tesseras-${VERSION}.tgz" \
    "$OUTDIR/tesseras-${VERSION}-openbsd.tgz"

echo "==> OpenBSD package: $OUTDIR/tesseras-${VERSION}-openbsd.tgz"

# 5. Optionally fetch the binary for inspection
$SCP_CMD "${OPENBSD_USER}@${OPENBSD_HOST}:${REMOTE_DIR}/target/release/tes" \
    "$OUTDIR/tes-openbsd-amd64" 2>/dev/null || true

echo "==> Done. Install on OpenBSD with: doas pkg_add tesseras-${VERSION}-openbsd.tgz"
