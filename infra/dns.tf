# DNS records for bootstrap node discovery.
# Requires tesseras.net zone to be managed in Hetzner DNS
# or configured manually if using an external DNS provider.

# A records for bootstrap nodes
resource "hcloud_rdns" "boot1_v4" {
  server_id  = hcloud_server.boot1.id
  ip_address = hcloud_server.boot1.ipv4_address
  dns_ptr    = "boot1.tesseras.net"
}

resource "hcloud_rdns" "boot2_v4" {
  server_id  = hcloud_server.boot2.id
  ip_address = hcloud_server.boot2.ipv4_address
  dns_ptr    = "boot2.tesseras.net"
}

# Note: Forward DNS (A records, TXT records for _tesseras._udp.tesseras.net)
# must be configured at the domain registrar / DNS provider separately.
# Example TXT records for bootstrap discovery:
#   _tesseras._udp.tesseras.net  TXT  "boot1.tesseras.net:4433"
#   _tesseras._udp.tesseras.net  TXT  "boot2.tesseras.net:4433"
