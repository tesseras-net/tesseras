#!/bin/sh
# Build and package tesseras for OpenBSD
# Run this inside an OpenBSD machine (native or QEMU) with Rust toolchain installed
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$PROJECT_ROOT/tes/Cargo.toml" | head -1)
PKG_NAME="tesseras-${VERSION}"
STAGING="$PROJECT_ROOT/target/openbsd-staging"
DESTDIR="$STAGING/fake"
OUTDIR="$PROJECT_ROOT/target/packages"

echo "==> Building tesseras $VERSION for OpenBSD"

# 1. Build release binary
cd "$PROJECT_ROOT"
cargo build --release -p tes

# 2. Create staging directory with OpenBSD layout
rm -rf "$STAGING"
mkdir -p "$DESTDIR/usr/local/bin"
mkdir -p "$DESTDIR/etc/tesseras"
mkdir -p "$DESTDIR/etc/rc.d"

# Binary
install -m 0755 target/release/tes "$DESTDIR/usr/local/bin/tes"

# Config — adapt Debian default for OpenBSD paths
# Named .dist so @sample creates config.toml from it on install
sed 's|/run/tesseras/|/var/run/tesseras/|' \
    "$SCRIPT_DIR/../debian/tesd.default.toml" \
    > "$DESTDIR/etc/tesseras/config.toml.dist"

# rc.d script
install -m 0555 "$SCRIPT_DIR/tesd.rc" "$DESTDIR/etc/rc.d/tesd"

# 3. Generate packing list with user/group creation and lifecycle hooks
cat > "$STAGING/packing-list" <<PLIST
@conflict tesseras-*
@pkgpath sysutils/tesseras
@comment P2P network for preserving human memories
@newgroup _tesseras:899
@newuser _tesseras:899:_tesseras::Tesseras daemon:/var/lib/tesseras:/sbin/nologin
@rcscript etc/rc.d/tesd
@exec install -d -o _tesseras -g _tesseras -m 0750 /var/lib/tesseras
@exec install -d -o root -g wheel -m 0755 /etc/tesseras
@unexec-delete rcctl stop tesd 2>/dev/null || true
@unexec-delete rcctl disable tesd 2>/dev/null || true
usr/local/bin/tes
etc/tesseras/
etc/tesseras/config.toml.dist
@sample etc/tesseras/config.toml
PLIST

# 4. Build the package
mkdir -p "$OUTDIR"
cd "$DESTDIR"
pkg_create \
    -A "$(uname -m)" \
    -d "$SCRIPT_DIR/+DESC" \
    -D COMMENT="P2P memory preservation daemon" \
    -D FULLPKGPATH=sysutils/tesseras \
    -D PORTSDIR=. \
    -D FTP=https://tesseras.net \
    -D MAINTAINER="Ivan Carvalho <ivan@tesseras.net>" \
    -f "$STAGING/packing-list" \
    -B "$DESTDIR" \
    -p / \
    "$OUTDIR/${PKG_NAME}.tgz"

echo "==> Package created: $OUTDIR/${PKG_NAME}.tgz"
echo "==> Install with: doas pkg_add $OUTDIR/${PKG_NAME}.tgz"
