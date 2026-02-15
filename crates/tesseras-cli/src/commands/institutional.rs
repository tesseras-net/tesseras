use anyhow::{Context, Result};
use tesseras_core::ports::{IdentityStore, KeyAlgorithm};
use tesseras_storage::FsIdentityStore;

use super::init::expand_tilde;

/// Run `tes institutional setup --domain <domain> [--check]`
pub async fn run_setup(domain: &str, check: bool, data_dir: &str) -> Result<()> {
    let base = expand_tilde(data_dir);
    let identity_dir = base.join("identity");

    // Ensure identity exists
    tokio::fs::create_dir_all(&identity_dir).await?;
    let identity_store = FsIdentityStore::new(base.clone());
    if !identity_store.keypair_exists(KeyAlgorithm::Ed25519)? {
        let keypair = tesseras_crypto::ed25519::Ed25519KeyGenerator::generate();
        let material: tesseras_core::ports::KeyMaterial = (&keypair).into();
        identity_store.save_keypair(&material)?;
        println!("Generated new Ed25519 identity");
    }

    // Load identity
    let key_material = identity_store
        .load_keypair(KeyAlgorithm::Ed25519)
        .context("failed to load Ed25519 keypair")?;
    let public_key: [u8; 32] = key_material
        .public
        .as_slice()
        .try_into()
        .context("public key must be 32 bytes")?;

    // Compute NodeId (same logic as daemon)
    let identity = compute_node_identity(&public_key, &base)
        .await
        .context("failed to compute node identity")?;

    let node_hex = hex::encode(identity.node_id.as_bytes());
    let pubkey_hex = hex::encode(identity.public_key);

    if check {
        run_check(domain, &identity).await
    } else {
        println!("Add this DNS TXT record:\n");
        println!("  _tesseras.{domain} TXT \"v=tesseras1 node={node_hex} pubkey={pubkey_hex}\"");
        println!();
        println!("Then add to ~/.tesseras/config.toml:");
        println!();
        println!("  [institutional]");
        println!("  domain = \"{domain}\"");
        println!("  pledge_bytes = 536870912000  # 500 GB");
        println!();
        println!("Verify DNS propagation with:");
        println!("  tes institutional setup --domain {domain} --check");
        Ok(())
    }
}

async fn run_check(domain: &str, identity: &tesseras_core::NodeIdentity) -> Result<()> {
    use hickory_resolver::TokioResolver;

    let resolver: TokioResolver = TokioResolver::builder_tokio()
        .context("failed to create DNS resolver")?
        .build();

    let lookup_name = format!("_tesseras.{domain}");
    println!("Resolving {lookup_name}...");

    let response = resolver
        .txt_lookup(&lookup_name)
        .await
        .context(format!("DNS lookup failed for {lookup_name}"))?;

    let node_hex = hex::encode(identity.node_id.as_bytes());
    let pubkey_hex = hex::encode(identity.public_key);

    for txt_data in response.iter() {
        let txt = txt_data.to_string();
        if txt.contains("v=tesseras1") {
            if txt.contains(&node_hex) && txt.contains(&pubkey_hex) {
                println!("DNS verified: _tesseras.{domain} matches local identity");
                return Ok(());
            } else {
                anyhow::bail!(
                    "DNS record found but does not match local identity.\n\
                     Expected node={node_hex} pubkey={pubkey_hex}\n\
                     Got: {txt}"
                );
            }
        }
    }

    anyhow::bail!("No tesseras TXT record found at _tesseras.{domain}")
}

/// Load the PoW nonce from identity.key and compute NodeId.
async fn compute_node_identity(
    public_key: &[u8; 32],
    base: &std::path::Path,
) -> Result<tesseras_core::NodeIdentity> {
    let identity_path = base.join("identity.key");
    let data = tokio::fs::read(&identity_path)
        .await
        .context("identity.key not found — run `tes init` or start the daemon first")?;

    // identity.key format: 32 bytes pubkey + 8 bytes nonce (little-endian)
    if data.len() < 40 {
        anyhow::bail!(
            "identity.key too short (expected 40 bytes, got {})",
            data.len()
        );
    }
    let nonce = u64::from_le_bytes(data[32..40].try_into()?);

    // Compute NodeId = BLAKE3(pubkey || nonce)[..20]
    let mut hasher = blake3::Hasher::new();
    hasher.update(public_key);
    hasher.update(&nonce.to_le_bytes());
    let hash = hasher.finalize();
    let mut node_id = [0u8; 20];
    node_id.copy_from_slice(&hash.as_bytes()[..20]);

    Ok(tesseras_core::NodeIdentity {
        node_id: tesseras_core::types::NodeId::new(node_id),
        public_key: *public_key,
        nonce,
    })
}
