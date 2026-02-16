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
  description = "tesd version/tag to deploy"
  type        = string
  default     = "latest"
}

variable "cloudflare_api_token" {
  description = "Cloudflare API token with DNS edit permission for tesseras.net zone"
  type        = string
  sensitive   = true
}

variable "cloudflare_zone_id" {
  description = "Cloudflare zone ID for tesseras.net"
  type        = string
}
