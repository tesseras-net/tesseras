output "boot1_ip" {
  description = "Public IPv4 of bootstrap node 1 (Falkenstein, DE)"
  value       = hcloud_server.boot1.ipv4_address
}

output "bootstrap_dns" {
  description = "Bootstrap node DNS address"
  value       = "bootstrap1.tesseras.net:4433"
}
