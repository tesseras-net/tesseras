#!/usr/bin/env bash
# Cloud-init provisioning script for tesseras bootstrap nodes.
# Templated by OpenTofu — variables: node_name, bootstrap_peers, daemon_version
#
# This script runs once at VPS creation. The tesseras-daemon .deb package
# handles user creation, config installation, and systemd unit via maintainer
# scripts. This script just prepares the system.
set -euo pipefail

NODE_NAME="${node_name}"
DAEMON_VERSION="${daemon_version}"

export DEBIAN_FRONTEND=noninteractive

# System updates
apt-get update -y
apt-get upgrade -y
apt-get install -y --no-install-recommends curl ca-certificates

echo "Provisioning complete for $NODE_NAME (daemon $DAEMON_VERSION)"
echo "Deploy the .deb package: scp tesseras-daemon_*.deb root@<host>:/tmp/ && dpkg -i /tmp/tesseras-daemon_*.deb"
