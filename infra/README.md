# Infrastructure

OpenTofu (Terraform-compatible) configuration for tesseras bootstrap nodes.

## Prerequisites

- [OpenTofu](https://opentofu.org/) >= 1.6
- Hetzner Cloud account + API token
- Cloudflare account managing `tesseras.net` + API token with DNS edit
- SSH key pair

## Quick Start

```bash
# 1. Copy and fill in secrets
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars with your tokens and SSH key

# 2. Initialize providers
tofu init

# 3. Review what will be created
tofu plan

# 4. Provision infrastructure
tofu apply
```

## Deploy the Daemon

After provisioning, deploy the tesd .deb package:

```bash
# From the project root:
just deploy                                    # deploys to bootstrap1.tesseras.net
just deploy host="bootstrap2.tesseras.net"     # deploy to a specific host
```

This builds a static MUSL binary, packages it as .deb, uploads via scp, and installs with dpkg.

## What Gets Created

| Resource | Description |
|----------|-------------|
| Hetzner VPS (cx23) | Debian 13, Falkenstein DE |
| Hetzner SSH key | Your public key for access |
| Hetzner firewall | UDP 4433, TCP 22, TCP 9190 (internal) |
| Hetzner rDNS | bootstrap1.tesseras.net |
| Cloudflare A record | bootstrap1.tesseras.net -> IPv4 |
| Cloudflare AAAA record | bootstrap1.tesseras.net -> IPv6 |
| Cloudflare SRV record | _tesseras._udp.tesseras.net -> bootstrap1:4433 |

## Security Hardening

The `scripts/provision.sh` cloud-init script applies:

- **SSH**: key-only auth, no password login
- **fail2ban**: bans after 3 failed SSH attempts (1h)
- **nftables**: default-drop firewall (22/tcp, 4433/udp, 9190/tcp internal)
- **unattended-upgrades**: automatic daily security patches
- **sysctl**: SYN cookies, no ICMP redirects, no forwarding, restricted BPF/dmesg

The systemd unit adds process-level hardening (NoNewPrivileges, ProtectSystem=strict, etc.).

## Files

| File | Purpose |
|------|---------|
| `main.tf` | Provider configuration (Hetzner + Cloudflare) |
| `variables.tf` | Input variables |
| `bootstrap.tf` | VPS resources (boot1 active, boot2 commented out) |
| `dns.tf` | DNS records (forward + reverse) |
| `firewall.tf` | Cloud firewall rules |
| `outputs.tf` | IP and DNS outputs |
| `terraform.tfvars.example` | Template for secrets |
| `scripts/provision.sh` | Cloud-init provisioning + hardening |

## Adding a Second Node

1. Uncomment `boot2` in `bootstrap.tf`
2. Add DNS records for `bootstrap2` in `dns.tf`
3. Add a second SRV record
4. `tofu apply`
5. `just deploy host="bootstrap2.tesseras.net"`
