#!/usr/bin/env bash
# Cloud-init provisioning script for tesseras bootstrap nodes.
# Templated by OpenTofu — variables: node_name, bootstrap_peers, daemon_version
set -euo pipefail

NODE_NAME="${node_name}"
BOOTSTRAP_PEERS="${bootstrap_peers}"
DAEMON_VERSION="${daemon_version}"

export DEBIAN_FRONTEND=noninteractive

# System updates
apt-get update -y
apt-get upgrade -y
apt-get install -y --no-install-recommends curl ca-certificates

# Create tesseras user
useradd --system --create-home --shell /usr/sbin/nologin tesseras

# Download daemon binary (placeholder — replace with actual release URL)
# For now, assume the binary is deployed via CI/CD or manual upload.
INSTALL_DIR="/usr/local/bin"
DATA_DIR="/var/lib/tesseras"
CONFIG_DIR="/etc/tesseras"

mkdir -p "$DATA_DIR" "$CONFIG_DIR"
chown tesseras:tesseras "$DATA_DIR"

# Write config
cat > "$CONFIG_DIR/config.toml" <<TOML
[node]
data_dir = "$DATA_DIR"
listen_addr = "0.0.0.0:4433"

[dht]
k = 20
alpha = 3
bucket_refresh_interval_secs = 3600
republish_interval_secs = 3600
pointer_ttl_secs = 86400
max_stored_pointers = 100000
ping_failure_threshold = 3

[bootstrap]
dns_domain = "_tesseras._udp.tesseras.net"
hardcoded = ["$BOOTSTRAP_PEERS"]

[network]
enable_mdns = false

[observability]
metrics_addr = "127.0.0.1:9190"
log_format = "json"
TOML

# Systemd service
cat > /etc/systemd/system/tesseras-daemon.service <<EOF
[Unit]
Description=Tesseras P2P Daemon ($NODE_NAME)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=tesseras
Group=tesseras
ExecStart=$INSTALL_DIR/tesseras-daemon --config $CONFIG_DIR/config.toml
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

# Security hardening
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$DATA_DIR
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable tesseras-daemon

echo "Provisioning complete for $NODE_NAME (daemon $DAEMON_VERSION)"
echo "Upload tesseras-daemon binary to $INSTALL_DIR and run: systemctl start tesseras-daemon"
