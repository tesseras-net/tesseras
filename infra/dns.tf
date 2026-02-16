# Reverse DNS (Hetzner)
resource "hcloud_rdns" "boot1_v4" {
  server_id  = hcloud_server.boot1.id
  ip_address = hcloud_server.boot1.ipv4_address
  dns_ptr    = "bootstrap1.tesseras.net"
}

# Forward DNS (Cloudflare)

# A record: bootstrap1.tesseras.net -> boot1 IPv4
resource "cloudflare_record" "bootstrap1" {
  zone_id = var.cloudflare_zone_id
  name    = "bootstrap1"
  content = hcloud_server.boot1.ipv4_address
  type    = "A"
  ttl     = 300
  proxied = false # QUIC/UDP — Cloudflare proxy only supports HTTP/HTTPS
}

# AAAA record: bootstrap1.tesseras.net -> boot1 IPv6
# Hetzner returns a /64 block; cidrhost extracts first usable address
resource "cloudflare_record" "bootstrap1_v6" {
  zone_id = var.cloudflare_zone_id
  name    = "bootstrap1"
  content = cidrhost(hcloud_server.boot1.ipv6_address, 1)
  type    = "AAAA"
  ttl     = 300
  proxied = false
}

# SRV record for bootstrap discovery: _tesseras._udp.tesseras.net
resource "cloudflare_record" "bootstrap_srv" {
  zone_id = var.cloudflare_zone_id
  name    = "_tesseras._udp"
  type    = "SRV"
  data {
    service  = "_tesseras"
    proto    = "_udp"
    name     = "tesseras.net"
    priority = 10
    weight   = 100
    port     = 4433
    target   = "bootstrap1.tesseras.net"
  }
  ttl = 300
}
