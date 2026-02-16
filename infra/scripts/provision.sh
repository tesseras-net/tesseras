#!/usr/bin/env bash
# Cloud-init provisioning script for tesseras bootstrap nodes.
# Templated by OpenTofu — variables: node_name, bootstrap_peers, daemon_version
#
# This script runs once at VPS creation. The tesseras-daemon .deb package
# handles user creation, config installation, and systemd unit via maintainer
# scripts. This script prepares the system and applies security hardening.
set -euo pipefail

NODE_NAME="${node_name}"
DAEMON_VERSION="${daemon_version}"

export DEBIAN_FRONTEND=noninteractive

# ── System updates ──────────────────────────────────────────────────────────
apt-get update -y
apt-get upgrade -y
apt-get install -y --no-install-recommends \
    curl ca-certificates nftables unattended-upgrades apt-listchanges fail2ban

# ── SSH hardening ───────────────────────────────────────────────────────────
sed -i 's/^#\?PermitRootLogin.*/PermitRootLogin prohibit-password/' /etc/ssh/sshd_config
sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config
sed -i 's/^#\?KbdInteractiveAuthentication.*/KbdInteractiveAuthentication no/' /etc/ssh/sshd_config
systemctl restart sshd

# ── Automatic security updates ──────────────────────────────────────────────
cat > /etc/apt/apt.conf.d/50unattended-upgrades <<'EOF'
Unattended-Upgrade::Origins-Pattern {
    "origin=Debian,codename=${distro_codename},label=Debian-Security";
    "origin=Debian,codename=${distro_codename}-security,label=Debian-Security";
};
Unattended-Upgrade::AutoFixInterruptedDpkg "true";
Unattended-Upgrade::Remove-Unused-Dependencies "true";
Unattended-Upgrade::Automatic-Reboot "false";
EOF
cat > /etc/apt/apt.conf.d/20auto-upgrades <<'EOF'
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
APT::Periodic::AutocleanInterval "7";
EOF

# ── fail2ban ────────────────────────────────────────────────────────────────
cat > /etc/fail2ban/jail.local <<'EOF'
[sshd]
enabled = true
mode = aggressive
maxretry = 3
bantime = 3600
findtime = 600
EOF
systemctl enable --now fail2ban

# ── nftables firewall ──────────────────────────────────────────────────────
cat > /etc/nftables.conf <<'EOF'
#!/usr/sbin/nft -f
flush ruleset

table inet filter {
    chain input {
        type filter hook input priority 0; policy drop;

        # Established/related
        ct state established,related accept

        # Loopback
        iif lo accept

        # ICMP/ICMPv6 (ping, neighbor discovery)
        ip protocol icmp accept
        ip6 nexthdr icmpv6 accept

        # SSH
        tcp dport 22 accept

        # QUIC (tesseras DHT + data)
        udp dport 4433 accept

        # Prometheus metrics (internal only)
        tcp dport 9190 ip saddr 10.0.0.0/8 accept

        # Drop everything else (logged)
        log prefix "nftables-drop: " limit rate 5/minute counter drop
    }

    chain forward {
        type filter hook forward priority 0; policy drop;
    }

    chain output {
        type filter hook output priority 0; policy accept;
    }
}
EOF
systemctl enable --now nftables

# ── Kernel hardening ────────────────────────────────────────────────────────
cat > /etc/sysctl.d/90-tesseras-hardening.conf <<'EOF'
# Ignore ICMP redirects
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0
net.ipv6.conf.default.accept_redirects = 0

# Don't send ICMP redirects
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.default.send_redirects = 0

# Ignore source-routed packets
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.conf.default.accept_source_route = 0
net.ipv6.conf.all.accept_source_route = 0
net.ipv6.conf.default.accept_source_route = 0

# SYN flood protection
net.ipv4.tcp_syncookies = 1

# Log suspicious packets
net.ipv4.conf.all.log_martians = 1
net.ipv4.conf.default.log_martians = 1

# Disable IP forwarding
net.ipv4.ip_forward = 0
net.ipv6.conf.all.forwarding = 0

# Harden BPF
kernel.unprivileged_bpf_disabled = 1

# Restrict dmesg
kernel.dmesg_restrict = 1

# Restrict kernel pointers
kernel.kptr_restrict = 2
EOF
sysctl --system > /dev/null 2>&1

echo "Provisioning complete for $NODE_NAME (daemon $DAEMON_VERSION)"
echo "Deploy the .deb package: scp tesseras-daemon_*.deb root@<host>:/tmp/ && dpkg -i /tmp/tesseras-daemon_*.deb"
