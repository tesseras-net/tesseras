resource "hcloud_ssh_key" "tesseras" {
  name       = "tesseras-bootstrap"
  public_key = var.ssh_public_key
}

resource "hcloud_server" "boot1" {
  name        = "tesseras-boot1"
  server_type = "cx22"
  image       = "debian-12"
  location    = "fsn1"
  ssh_keys    = [hcloud_ssh_key.tesseras.id]

  labels = {
    role    = "bootstrap"
    project = "tesseras"
  }

  user_data = templatefile("${path.module}/scripts/provision.sh", {
    node_name       = "boot1"
    bootstrap_peers = ""
    daemon_version  = var.daemon_version
  })
}

# boot2 commented out for MVP — uncomment when adding second bootstrap node
# resource "hcloud_server" "boot2" {
#   name        = "tesseras-boot2"
#   server_type = "cx22"
#   image       = "debian-12"
#   location    = "hel1"
#   ssh_keys    = [hcloud_ssh_key.tesseras.id]
#
#   labels = {
#     role    = "bootstrap"
#     project = "tesseras"
#   }
#
#   user_data = templatefile("${path.module}/scripts/provision.sh", {
#     node_name       = "boot2"
#     bootstrap_peers = "${hcloud_server.boot1.ipv4_address}:4433"
#     daemon_version  = var.daemon_version
#   })
#
#   depends_on = [hcloud_server.boot1]
# }
