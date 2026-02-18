output "bootstrap_nodes" {
  description = "Bootstrap node addresses"
  value = {
    for k, v in local.bootstrap_nodes : k => {
      hostname = "${v.hostname}.tesseras.net"
      ipv4     = v.ipv4
      ipv6     = v.ipv6
      quic     = "${v.hostname}.tesseras.net:4433"
    }
  }
}
