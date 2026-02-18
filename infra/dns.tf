# Forward DNS (Cloudflare) — A/AAAA records for each bootstrap node

resource "cloudflare_record" "bootstrap_a" {
  for_each = local.bootstrap_nodes

  zone_id = var.cloudflare_zone_id
  name    = each.value.hostname
  content = each.value.ipv4
  type    = "A"
  ttl     = 300
  proxied = false # QUIC/UDP — Cloudflare proxy only supports HTTP/HTTPS
}

resource "cloudflare_record" "bootstrap_aaaa" {
  for_each = local.bootstrap_nodes

  zone_id = var.cloudflare_zone_id
  name    = each.value.hostname
  content = each.value.ipv6
  type    = "AAAA"
  ttl     = 300
  proxied = false
}

# SRV records for bootstrap discovery: _tesseras._udp.tesseras.net
resource "cloudflare_record" "bootstrap_srv" {
  for_each = local.bootstrap_nodes

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
    target   = "${each.value.hostname}.tesseras.net"
  }
  ttl = 300
}
