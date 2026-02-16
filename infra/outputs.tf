output "boot1_ip" {
  description = "Public IPv4 of bootstrap node 1 (Falkenstein, DE)"
  value       = hcloud_server.boot1.ipv4_address
}

output "boot2_ip" {
  description = "Public IPv4 of bootstrap node 2 (Helsinki, FI)"
  value       = hcloud_server.boot2.ipv4_address
}

output "bootstrap_dns" {
  description = "Bootstrap node DNS addresses"
  value = [
    "bootstrap1.tesseras.net:4433",
    "bootstrap2.tesseras.net:4433",
  ]
}
