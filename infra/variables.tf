variable "hcloud_token" {
  description = "Hetzner Cloud API token"
  type        = string
  sensitive   = true
}

variable "ssh_public_key" {
  description = "SSH public key for node access"
  type        = string
}

variable "daemon_version" {
  description = "tesseras-daemon version/tag to deploy"
  type        = string
  default     = "latest"
}
