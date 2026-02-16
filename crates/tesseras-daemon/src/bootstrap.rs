//! SRV-based bootstrap peer discovery.
//!
//! Resolves `_tesseras._udp.<dns_domain>` SRV records to find bootstrap nodes.
//! Falls back to `hardcoded` addresses from config if DNS fails.

use std::net::SocketAddr;

use crate::config::BootstrapConfig;

/// Resolve bootstrap peer addresses via SRV DNS lookup, falling back to hardcoded list.
///
/// Performs a SRV lookup on `_tesseras._udp.<dns_domain>` and resolves each target's
/// A/AAAA records. If DNS fails or returns no results, falls back to the hardcoded
/// addresses in config.
pub async fn resolve_bootstrap_peers(config: &BootstrapConfig) -> Vec<SocketAddr> {
    match resolve_srv(&config.dns_domain).await {
        Ok(addrs) if !addrs.is_empty() => {
            tracing::info!(count = addrs.len(), "resolved bootstrap peers via SRV");
            addrs
        }
        Ok(_) => {
            tracing::warn!(domain = %config.dns_domain, "SRV lookup returned no results, using hardcoded");
            resolve_hardcoded(&config.hardcoded).await
        }
        Err(e) => {
            tracing::warn!(domain = %config.dns_domain, error = %e, "SRV lookup failed, using hardcoded");
            resolve_hardcoded(&config.hardcoded).await
        }
    }
}

/// Perform SRV lookup on `_tesseras._udp.<domain>` and resolve targets to socket addresses.
async fn resolve_srv(dns_domain: &str) -> anyhow::Result<Vec<SocketAddr>> {
    use hickory_resolver::TokioResolver;

    let resolver: TokioResolver = TokioResolver::builder_tokio()
        .map_err(|e| anyhow::anyhow!("failed to create DNS resolver: {e}"))?
        .build();

    let srv_name = format!("_tesseras._udp.{dns_domain}");
    let srv_lookup = resolver.srv_lookup(&srv_name).await?;

    let mut addrs = Vec::new();
    for srv in srv_lookup.iter() {
        let target = srv.target().to_string();
        let port = srv.port();

        match resolver.lookup_ip(&target).await {
            Ok(ips) => {
                for ip in ips.iter() {
                    addrs.push(SocketAddr::new(ip, port));
                }
            }
            Err(e) => {
                tracing::warn!(target = %target, error = %e, "failed to resolve SRV target");
            }
        }
    }

    Ok(addrs)
}

/// Resolve hardcoded address strings (host:port) to socket addresses.
async fn resolve_hardcoded(hardcoded: &[String]) -> Vec<SocketAddr> {
    let mut resolved = Vec::new();
    for addr in hardcoded {
        if addr.is_empty() {
            continue;
        }
        match tokio::net::lookup_host(addr).await {
            Ok(addrs) => {
                let all: Vec<_> = addrs.into_iter().collect();
                if all.is_empty() {
                    tracing::warn!(addr = %addr, "DNS resolved but returned no addresses");
                } else {
                    tracing::debug!(addr = %addr, results = ?all, "resolved hardcoded address");
                    resolved.extend(all);
                }
            }
            Err(e) => {
                tracing::warn!(addr = %addr, error = %e, "failed to resolve hardcoded address");
            }
        }
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hardcoded_empty_list_returns_empty() {
        let addrs = resolve_hardcoded(&[]).await;
        assert!(addrs.is_empty());
    }

    #[tokio::test]
    async fn hardcoded_skips_empty_strings() {
        let addrs = resolve_hardcoded(&["".to_string()]).await;
        assert!(addrs.is_empty());
    }

    #[tokio::test]
    async fn hardcoded_resolves_localhost() {
        let addrs = resolve_hardcoded(&["127.0.0.1:4433".to_string()]).await;
        assert_eq!(addrs.len(), 1);
        assert_eq!(addrs[0], "127.0.0.1:4433".parse::<SocketAddr>().unwrap());
    }

    #[tokio::test]
    async fn fallback_on_invalid_srv_domain() {
        let config = BootstrapConfig {
            dns_domain: "nonexistent.invalid.test".to_string(),
            hardcoded: vec!["127.0.0.1:4433".to_string()],
        };
        let addrs = resolve_bootstrap_peers(&config).await;
        // Should fall back to hardcoded
        assert!(!addrs.is_empty());
        assert_eq!(addrs[0], "127.0.0.1:4433".parse::<SocketAddr>().unwrap());
    }

    #[tokio::test]
    async fn empty_config_returns_empty() {
        let config = BootstrapConfig {
            dns_domain: "nonexistent.invalid.test".to_string(),
            hardcoded: vec![],
        };
        let addrs = resolve_bootstrap_peers(&config).await;
        assert!(addrs.is_empty());
    }
}
