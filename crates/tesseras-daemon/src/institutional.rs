use std::fmt;

use tesseras_core::{NodeId, NodeIdentity};

/// Parsed DNS TXT record for institutional verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionalRecord {
    pub version: String,
    pub node_id: NodeId,
    pub pubkey: [u8; 32],
}

/// Errors during institutional verification.
#[derive(Debug)]
pub enum VerifyError {
    DnsLookupFailed(String),
    RecordNotFound,
    InvalidFormat(String),
    NodeIdMismatch { expected: NodeId, got: NodeId },
    PubkeyMismatch,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::DnsLookupFailed(e) => write!(f, "DNS lookup failed: {e}"),
            VerifyError::RecordNotFound => {
                write!(f, "no _tesseras TXT record found")
            }
            VerifyError::InvalidFormat(e) => write!(f, "invalid TXT record format: {e}"),
            VerifyError::NodeIdMismatch { expected, got } => {
                write!(f, "NodeId mismatch: expected {expected}, got {got}")
            }
            VerifyError::PubkeyMismatch => write!(f, "pubkey mismatch"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Parse a tesseras DNS TXT record value.
///
/// Format: `v=tesseras1 node=<hex> pubkey=<hex>`
pub fn parse_txt_record(txt: &str) -> Result<InstitutionalRecord, VerifyError> {
    let mut version = None;
    let mut node_hex = None;
    let mut pubkey_hex = None;

    for part in txt.split_whitespace() {
        if let Some(v) = part.strip_prefix("v=") {
            version = Some(v.to_string());
        } else if let Some(n) = part.strip_prefix("node=") {
            node_hex = Some(n.to_string());
        } else if let Some(p) = part.strip_prefix("pubkey=") {
            pubkey_hex = Some(p.to_string());
        }
    }

    let version = version.ok_or_else(|| VerifyError::InvalidFormat("missing v= field".into()))?;

    if version != "tesseras1" {
        return Err(VerifyError::InvalidFormat(format!(
            "unsupported version: {version}"
        )));
    }

    let node_hex =
        node_hex.ok_or_else(|| VerifyError::InvalidFormat("missing node= field".into()))?;
    let pubkey_hex =
        pubkey_hex.ok_or_else(|| VerifyError::InvalidFormat("missing pubkey= field".into()))?;

    let node_bytes = hex::decode(&node_hex)
        .map_err(|e| VerifyError::InvalidFormat(format!("invalid node hex: {e}")))?;
    if node_bytes.len() != 20 {
        return Err(VerifyError::InvalidFormat(format!(
            "node must be 20 bytes, got {}",
            node_bytes.len()
        )));
    }
    let mut node_arr = [0u8; 20];
    node_arr.copy_from_slice(&node_bytes);

    let pubkey_bytes = hex::decode(&pubkey_hex)
        .map_err(|e| VerifyError::InvalidFormat(format!("invalid pubkey hex: {e}")))?;
    if pubkey_bytes.len() != 32 {
        return Err(VerifyError::InvalidFormat(format!(
            "pubkey must be 32 bytes, got {}",
            pubkey_bytes.len()
        )));
    }
    let mut pubkey_arr = [0u8; 32];
    pubkey_arr.copy_from_slice(&pubkey_bytes);

    Ok(InstitutionalRecord {
        version,
        node_id: NodeId::new(node_arr),
        pubkey: pubkey_arr,
    })
}

/// Verify a parsed record matches the local identity.
pub fn verify_identity(
    record: &InstitutionalRecord,
    identity: &NodeIdentity,
) -> Result<(), VerifyError> {
    if record.node_id != identity.node_id {
        return Err(VerifyError::NodeIdMismatch {
            expected: identity.node_id,
            got: record.node_id,
        });
    }
    if record.pubkey != identity.public_key {
        return Err(VerifyError::PubkeyMismatch);
    }
    Ok(())
}

/// Format the DNS TXT record string for a given identity.
#[allow(dead_code)]
pub fn format_txt_record(domain: &str, identity: &NodeIdentity) -> String {
    let node_hex = hex::encode(identity.node_id.as_bytes());
    let pubkey_hex = hex::encode(identity.public_key);
    format!("_tesseras.{domain} TXT \"v=tesseras1 node={node_hex} pubkey={pubkey_hex}\"")
}

/// Resolve DNS TXT record and verify against local identity.
///
/// Returns `Ok(())` if verification succeeds.
pub async fn verify_dns(domain: &str, identity: &NodeIdentity) -> Result<(), VerifyError> {
    use hickory_resolver::TokioResolver;

    let resolver: TokioResolver = TokioResolver::builder_tokio()
        .map_err(|e| VerifyError::DnsLookupFailed(e.to_string()))?
        .build();

    let lookup_name = format!("_tesseras.{domain}");
    let response = resolver
        .txt_lookup(lookup_name.as_str())
        .await
        .map_err(|e: hickory_resolver::ResolveError| VerifyError::DnsLookupFailed(e.to_string()))?;

    for txt_data in response.iter() {
        let txt = txt_data.to_string();
        if let Ok(parsed) = parse_txt_record(&txt) {
            if parsed.version == "tesseras1" {
                return verify_identity(&parsed, identity);
            }
        }
    }

    Err(VerifyError::RecordNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity() -> NodeIdentity {
        NodeIdentity {
            node_id: NodeId::new([0xab; 20]),
            public_key: [0xcd; 32],
            nonce: 42,
        }
    }

    #[test]
    fn parse_valid_txt_record() {
        let id = test_identity();
        let node_hex = hex::encode(id.node_id.as_bytes());
        let pubkey_hex = hex::encode(id.public_key);
        let txt = format!("v=tesseras1 node={node_hex} pubkey={pubkey_hex}");
        let record = parse_txt_record(&txt).unwrap();
        assert_eq!(record.version, "tesseras1");
        assert_eq!(record.node_id, id.node_id);
        assert_eq!(record.pubkey, id.public_key);
    }

    #[test]
    fn parse_rejects_missing_version() {
        let txt = "node=abab pubkey=cdcd";
        assert!(parse_txt_record(txt).is_err());
    }

    #[test]
    fn parse_rejects_wrong_version() {
        let txt = "v=tesseras99 node=abab pubkey=cdcd";
        assert!(parse_txt_record(txt).is_err());
    }

    #[test]
    fn parse_rejects_missing_node() {
        let pubkey_hex = hex::encode([0xcd; 32]);
        let txt = format!("v=tesseras1 pubkey={pubkey_hex}");
        assert!(parse_txt_record(&txt).is_err());
    }

    #[test]
    fn parse_rejects_wrong_node_length() {
        let pubkey_hex = hex::encode([0xcd; 32]);
        let txt = format!("v=tesseras1 node=abab pubkey={pubkey_hex}");
        assert!(parse_txt_record(&txt).is_err());
    }

    #[test]
    fn verify_identity_matches() {
        let id = test_identity();
        let record = InstitutionalRecord {
            version: "tesseras1".into(),
            node_id: id.node_id,
            pubkey: id.public_key,
        };
        assert!(verify_identity(&record, &id).is_ok());
    }

    #[test]
    fn verify_identity_rejects_node_mismatch() {
        let id = test_identity();
        let record = InstitutionalRecord {
            version: "tesseras1".into(),
            node_id: NodeId::new([0x00; 20]),
            pubkey: id.public_key,
        };
        assert!(matches!(
            verify_identity(&record, &id),
            Err(VerifyError::NodeIdMismatch { .. })
        ));
    }

    #[test]
    fn verify_identity_rejects_pubkey_mismatch() {
        let id = test_identity();
        let record = InstitutionalRecord {
            version: "tesseras1".into(),
            node_id: id.node_id,
            pubkey: [0x00; 32],
        };
        assert!(matches!(
            verify_identity(&record, &id),
            Err(VerifyError::PubkeyMismatch)
        ));
    }

    #[test]
    fn format_txt_record_roundtrips() {
        let id = test_identity();
        let formatted = format_txt_record("example.org", &id);
        assert!(formatted.contains("_tesseras.example.org"));
        // Extract the TXT value between quotes
        let start = formatted.find('"').unwrap() + 1;
        let end = formatted.rfind('"').unwrap();
        let txt_value = &formatted[start..end];
        let parsed = parse_txt_record(txt_value).unwrap();
        assert_eq!(parsed.node_id, id.node_id);
        assert_eq!(parsed.pubkey, id.public_key);
    }
}
