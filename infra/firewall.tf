resource "hcloud_firewall" "tesseras" {
  name = "tesseras-bootstrap"

  # QUIC (DHT + data transfer)
  rule {
    direction = "in"
    protocol  = "udp"
    port      = "4433"
    source_ips = ["0.0.0.0/0", "::/0"]
  }

  # SSH
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = "22"
    source_ips = ["0.0.0.0/0", "::/0"]
  }

  # Prometheus metrics (internal only)
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = "9190"
    source_ips = ["10.0.0.0/8"]
  }
}

resource "hcloud_firewall_attachment" "boot1" {
  firewall_id = hcloud_firewall.tesseras.id
  server_ids  = [hcloud_server.boot1.id]
}
