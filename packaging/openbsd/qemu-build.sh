#!/bin/sh
# Cross-build tesseras for OpenBSD using QEMU
#
# Prerequisites:
#   - QEMU installed: pacman -S qemu-full (Arch) or apt install qemu-system-x86 (Debian)
#   - An OpenBSD QEMU image with Rust toolchain and rsync
#   - SSH access configured (key-based, port forwarded to host)
#
# Setup (one-time):
#   1. Download OpenBSD install ISO: https://cdn.openbsd.org/pub/OpenBSD/7.7/amd64/install77.iso
#   2. Create disk image: qemu-img create -f qcow2 openbsd.qcow2 20G
#   3. Install OpenBSD in QEMU (use this script with --start-vm to boot)
#   4. Inside OpenBSD: pkg_add rust rsync
#   5. Snapshot the image for fast reuse
#
# Usage:
#   # Start VM and build (default image: ~/vms/openbsd77/openbsd77.qcow2)
#   ./qemu-build.sh --start-vm
#
#   # Start VM with custom image path
#   OPENBSD_IMAGE=/path/to/openbsd.qcow2 ./qemu-build.sh --start-vm
#
#   # Build only (VM already running or using remote host)
#   OPENBSD_HOST=m0x.example.com ./qemu-build.sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
OPENBSD_HOST="${OPENBSD_HOST:-127.0.0.1}"
OPENBSD_PORT="${OPENBSD_PORT:-7722}"
OPENBSD_USER="${OPENBSD_USER:-builder}"
OPENBSD_IMAGE="${OPENBSD_IMAGE:-$HOME/vms/openbsd77/openbsd77.qcow2}"
REMOTE_DIR="/home/${OPENBSD_USER}/tesseras-build"
SSH_OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR"

# QEMU settings
QEMU_MEM="${QEMU_MEM:-6G}"
QEMU_SMP="${QEMU_SMP:-8}"

START_VM=false
STOP_VM=false
for arg in "$@"; do
    case "$arg" in
        --start-vm) START_VM=true ;;
        --stop-vm)  STOP_VM=true ;;
    esac
done

if [ "$OPENBSD_PORT" != "22" ]; then
    SSH_OPTS="$SSH_OPTS -p $OPENBSD_PORT"
    SCP_OPTS="-P $OPENBSD_PORT"
else
    SCP_OPTS=""
fi

SSH_CMD="ssh $SSH_OPTS ${OPENBSD_USER}@${OPENBSD_HOST}"
SCP_CMD="scp $SSH_OPTS $SCP_OPTS"

# Start QEMU VM if requested
if [ "$START_VM" = true ]; then
    if ! [ -f "$OPENBSD_IMAGE" ]; then
        echo "Error: OpenBSD image not found: $OPENBSD_IMAGE"
        exit 1
    fi
    echo "==> Starting OpenBSD VM ($QEMU_MEM RAM, $QEMU_SMP CPUs, KVM)..."
    qemu-system-x86_64 \
        -m "$QEMU_MEM" -smp "$QEMU_SMP" -enable-kvm \
        -drive "file=$OPENBSD_IMAGE,format=qcow2" \
        -device virtio-net-pci,netdev=net0 \
        -netdev "user,id=net0,hostfwd=tcp::${OPENBSD_PORT}-:22" \
        -display none -daemonize

    echo "==> Waiting for VM to boot..."
    for i in $(seq 1 30); do
        if $SSH_CMD "true" 2>/dev/null; then
            break
        fi
        sleep 2
    done

    if ! $SSH_CMD "true" 2>/dev/null; then
        echo "Error: VM did not become reachable via SSH after 60s"
        exit 1
    fi

    # Enable SMT for better build performance
    $SSH_CMD "doas sysctl hw.smt=1" 2>/dev/null || true
    echo "==> VM ready"
fi

echo "==> Cross-building tesseras for OpenBSD via ${OPENBSD_HOST}:${OPENBSD_PORT}"

# 1. Sync source to OpenBSD machine
echo "==> Syncing source..."
$SSH_CMD "rm -rf $REMOTE_DIR && mkdir -p $REMOTE_DIR"
rsync -az --delete \
    -e "ssh $SSH_OPTS" \
    --exclude 'target/' \
    --exclude '.git/' \
    --exclude '.claude/' \
    --exclude 'docs/plans/' \
    --exclude 'apps/' \
    --exclude 'packaging/windows/' \
    --exclude 'infra/.terraform/' \
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

# Stop VM if requested
if [ "$STOP_VM" = true ]; then
    echo "==> Shutting down VM..."
    $SSH_CMD "doas shutdown -p now" 2>/dev/null || true
fi

echo "==> Done. Install on OpenBSD with: doas pkg_add -D unsigned tesseras-${VERSION}.tgz"
