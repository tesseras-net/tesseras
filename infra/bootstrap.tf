# Bootstrap node addresses.
# Servers are managed externally (not provisioned by OpenTofu).

locals {
  bootstrap_nodes = {
    boot1 = {
      hostname = "bootstrap1"
      ipv4     = "157.90.160.207"        # hetzner (Falkenstein, DE)
      ipv6     = "2a01:4f8:1c1e:7c11::1"
    }
    boot2 = {
      hostname = "bootstrap2"
      ipv4     = "46.23.94.11"           # m0x (OpenBSD)
      ipv6     = "2a03:6000:6f66:601::11"
    }
  }
}
