output "boot1_ip" {
  description = "Public IP of bootstrap node 1 (Falkenstein, DE)"
  value       = hcloud_server.boot1.ipv4_address
}

output "boot2_ip" {
  description = "Public IP of bootstrap node 2 (Helsinki, FI)"
  value       = hcloud_server.boot2.ipv4_address
}

output "bootstrap_peers" {
  description = "Comma-separated bootstrap peer addresses"
  value       = "${hcloud_server.boot1.ipv4_address}:4433,${hcloud_server.boot2.ipv4_address}:4433"
}
