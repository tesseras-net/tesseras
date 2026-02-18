#!/bin/sh
# E2E test entrypoint: configures the node before starting tes.
# Environment variables:
#   TES_IDENTITY     — data directory path (required)
#   TES_LISTEN       — listen address (e.g., 10.10.1.10:4433)
#   TES_BOOTSTRAP    — bootstrap address (e.g., 10.10.0.10:4433)
set -e

DATA_DIR="${TES_IDENTITY:-/data}"

# Ensure identity exists
tes --identity="$DATA_DIR" admin id >/dev/null 2>&1 || true

# Configure listen address if set
if [ -n "${TES_LISTEN:-}" ]; then
    tes --identity="$DATA_DIR" admin bootstrap ls >/dev/null 2>&1 || true
    # Write config directly — simpler than multiple CLI calls
    cat > "$DATA_DIR/config.toml" <<EOF
listen = "${TES_LISTEN}"
bootstrap = [$([ -n "${TES_BOOTSTRAP:-}" ] && echo "\"${TES_BOOTSTRAP}\"" || echo "")]
bootstrap_dns = ""
data_shards = 3
parity_shards = 2
stun_servers = []
EOF
fi

exec "$@"
