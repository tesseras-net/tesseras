variable "cloudflare_api_token" {
  description = "Cloudflare API token with DNS edit permission for tesseras.net zone"
  type        = string
  sensitive   = true
}

variable "cloudflare_zone_id" {
  description = "Cloudflare zone ID for tesseras.net"
  type        = string
}
