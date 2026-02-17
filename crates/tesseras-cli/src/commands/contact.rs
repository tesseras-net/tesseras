use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(clap::Subcommand)]
pub enum ContactCommands {
    /// Add a contact alias
    Add {
        /// Short alias (e.g. "wife", "friend")
        alias: String,
        /// Public key (hex-encoded ed25519 public key)
        public_key: String,
    },
    /// List all contacts
    List,
    /// Remove a contact
    Remove {
        /// Alias to remove
        alias: String,
    },
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ContactsFile {
    #[serde(default)]
    contacts: BTreeMap<String, String>,
}

fn contacts_path(base: &Path) -> PathBuf {
    base.join("contacts.toml")
}

fn load_contacts(base: &Path) -> Result<ContactsFile> {
    let path = contacts_path(base);
    if !path.exists() {
        return Ok(ContactsFile::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&content)
        .with_context(|| format!("invalid contacts file at {}", path.display()))
}

fn save_contacts(base: &Path, contacts: &ContactsFile) -> Result<()> {
    let path = contacts_path(base);
    let content = toml::to_string_pretty(contacts).context("failed to serialize contacts")?;
    std::fs::write(&path, content)
        .with_context(|| format!("failed to write {}", path.display()))
}

/// Look up a contact alias, returning the public key hex string.
/// Used by `pull_by_alias` (not yet wired up).
#[allow(dead_code)]
pub fn resolve_alias(base: &Path, alias: &str) -> Result<String> {
    let contacts = load_contacts(base)?;
    contacts
        .contacts
        .get(alias)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("unknown contact '{alias}'. Run 'tes contact add {alias} <key>'"))
}

pub async fn run(command: &ContactCommands, data_dir: &str) -> Result<()> {
    let base = super::init::expand_tilde(data_dir);

    match command {
        ContactCommands::Add { alias, public_key } => {
            if public_key.len() != 64 || !public_key.chars().all(|c| c.is_ascii_hexdigit()) {
                anyhow::bail!("public key must be 64 hex characters (ed25519 public key)");
            }
            let mut contacts = load_contacts(&base)?;
            contacts.contacts.insert(alias.clone(), public_key.clone());
            save_contacts(&base, &contacts)?;
            println!("Added contact '{alias}'");
        }
        ContactCommands::List => {
            let contacts = load_contacts(&base)?;
            if contacts.contacts.is_empty() {
                println!("No contacts. Add one with: tes contact add <alias> <public-key>");
                return Ok(());
            }
            for (alias, key) in &contacts.contacts {
                let short_key = &key[..16];
                println!("  @{alias} = {short_key}...");
            }
        }
        ContactCommands::Remove { alias } => {
            let mut contacts = load_contacts(&base)?;
            if contacts.contacts.remove(alias).is_some() {
                save_contacts(&base, &contacts)?;
                println!("Removed contact '{alias}'");
            } else {
                anyhow::bail!("contact '{alias}' not found");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contacts_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = dir.path();

        // Initially empty
        let contacts = load_contacts(base).unwrap();
        assert!(contacts.contacts.is_empty());

        // Add a contact
        let mut contacts = ContactsFile::default();
        let key = "a".repeat(64);
        contacts.contacts.insert("wife".into(), key.clone());
        save_contacts(base, &contacts).unwrap();

        // Reload and verify
        let loaded = load_contacts(base).unwrap();
        assert_eq!(loaded.contacts.get("wife").unwrap(), &key);

        // Resolve alias
        let resolved = resolve_alias(base, "wife").unwrap();
        assert_eq!(resolved, key);

        // Unknown alias fails
        assert!(resolve_alias(base, "unknown").is_err());
    }

    #[test]
    fn contacts_remove() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = dir.path();

        let mut contacts = ContactsFile::default();
        contacts.contacts.insert("friend".into(), "b".repeat(64));
        save_contacts(base, &contacts).unwrap();

        let mut contacts = load_contacts(base).unwrap();
        assert!(contacts.contacts.remove("friend").is_some());
        save_contacts(base, &contacts).unwrap();

        let loaded = load_contacts(base).unwrap();
        assert!(loaded.contacts.is_empty());
    }
}
