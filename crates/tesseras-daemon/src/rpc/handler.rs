use std::path::PathBuf;
use std::sync::Arc;

use tesseras_core::ports::{
    BlobStore, CircleRepository, FragmentStore, MemoryRepository, OperationQueue,
    TesseraRepository, TombstoneRepository,
};
use tesseras_dht::engine::DhtEngine;
use tesseras_rpc::error::ErrorCode;
use tesseras_rpc::protocol::{Request, Response};
use tesseras_replication::ReplicationService;
use tesseras_storage::CasStore;

pub struct RpcHandler {
    pub tessera_repo: Arc<dyn TesseraRepository>,
    pub memory_repo: Arc<dyn MemoryRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub fragment_store: Arc<dyn FragmentStore>,
    pub replication: Arc<ReplicationService>,
    pub cas: Arc<CasStore>,
    pub dht_engine: Arc<DhtEngine>,
    pub tombstone_repo: Arc<dyn TombstoneRepository>,
    pub circle_repo: Arc<dyn CircleRepository>,
    pub operation_queue: Arc<dyn OperationQueue>,
    pub data_dir: PathBuf,
    pub start_time: std::time::Instant,
}

impl RpcHandler {
    pub async fn handle(&self, request: Request) -> Response {
        match request {
            // Legacy
            Request::Publish { hash } => self.handle_publish(hash).await,
            Request::Fetch { hash } => self.handle_fetch(hash).await,
            Request::TesseraStatus { hash } => self.handle_tessera_status(hash).await,
            Request::Peers => self.handle_peers().await,

            // New operations
            Request::List { circle } => self.handle_list(circle.as_deref()),
            Request::Show { hash } => self.handle_show(&hash),
            Request::Delete { hash } => self.handle_delete(&hash).await,
            Request::Status => self.handle_node_status().await,
            Request::QueueStatus => self.handle_queue_status(),

            Request::CircleCreate { name } => self.handle_circle_create(&name),
            Request::CircleDelete { name } => self.handle_circle_delete(&name),
            Request::CircleAddMember {
                circle,
                alias,
                pubkey,
            } => self.handle_circle_add_member(&circle, &alias, &pubkey),
            Request::CircleRemoveMember { circle, alias } => {
                self.handle_circle_remove_member(&circle, &alias)
            }
            Request::CircleList { .. } => self.handle_circle_list(),

            // Not yet implemented
            _ => Response::Error {
                code: ErrorCode::Internal,
                message: "not yet implemented".to_string(),
            },
        }
    }

    async fn handle_peers(&self) -> Response {
        Response::PeerList {
            peers: self.dht_engine.all_peers().await,
        }
    }

    fn handle_list(&self, _circle: Option<&str>) -> Response {
        let all = match self.tessera_repo.list() {
            Ok(ts) => ts,
            Err(e) => {
                return Response::Error {
                    code: ErrorCode::Internal,
                    message: format!("storage error: {e}"),
                }
            }
        };

        let records: Vec<tesseras_rpc::TesseraInfo> = all
            .into_iter()
            .map(|t| tesseras_rpc::TesseraInfo {
                hash: t.hash.to_string(),
                created_at: t.created_at.to_rfc3339(),
                memory_count: t.memory_count,
                size_bytes: t.size_bytes,
                visibility: t.visibility.clone(),
                is_mine: t.is_mine,
            })
            .collect();

        Response::Listed { records }
    }

    fn handle_show(&self, hash_str: &str) -> Response {
        let hash = match resolve_hash(hash_str, self.tessera_repo.as_ref()) {
            Ok(h) => h,
            Err(resp) => return resp,
        };

        let tessera = match self.tessera_repo.find_by_hash(&hash) {
            Ok(Some(t)) => t,
            Ok(None) => {
                return Response::Error {
                    code: ErrorCode::NotFound,
                    message: format!("tessera not found: {hash}"),
                }
            }
            Err(e) => {
                return Response::Error {
                    code: ErrorCode::Internal,
                    message: format!("storage error: {e}"),
                }
            }
        };

        let memories = match self.memory_repo.list_by_tessera(&hash) {
            Ok(ms) => ms
                .into_iter()
                .map(|m| tesseras_rpc::MemoryInfo {
                    hash: m.hash.to_string(),
                    memory_type: m.memory_type,
                    media_path: m.media_path,
                    created_at: m.created_at.to_rfc3339(),
                })
                .collect(),
            Err(_) => vec![],
        };

        Response::Shown {
            record: tesseras_rpc::TesseraInfo {
                hash: tessera.hash.to_string(),
                created_at: tessera.created_at.to_rfc3339(),
                memory_count: tessera.memory_count,
                size_bytes: tessera.size_bytes,
                visibility: tessera.visibility,
                is_mine: tessera.is_mine,
            },
            memories,
        }
    }

    async fn handle_delete(&self, hash_str: &str) -> Response {
        let hash = match resolve_hash(hash_str, self.tessera_repo.as_ref()) {
            Ok(h) => h,
            Err(resp) => return resp,
        };

        // Create and store tombstone
        let tombstone = tesseras_core::Tombstone {
            hash,
            retracted_at: chrono::Utc::now(),
            creator_pubkey: String::new(), // TODO: fill from identity
            ed25519_signature: vec![],     // TODO: sign
            mldsa_signature: vec![],       // TODO: sign
        };

        if let Err(e) = self.tombstone_repo.store(&tombstone) {
            return Response::Error {
                code: ErrorCode::Internal,
                message: format!("failed to store tombstone: {e}"),
            };
        }

        // Delete from local storage (best-effort)
        let _ = self.blob_store.delete_tessera(&hash);
        let _ = self.tessera_repo.delete(&hash);

        // Propagate retraction to DHT (best-effort)
        let tombstone_published = self.dht_engine.retract(tombstone).await.unwrap_or(0) > 0;

        Response::Deleted {
            hash: hash.to_string(),
            tombstone_published,
        }
    }

    async fn handle_node_status(&self) -> Response {
        let peers = self.dht_engine.all_peers().await;
        let (pending, completed, failed) = self
            .operation_queue
            .count_by_status()
            .unwrap_or((0, 0, 0));

        Response::NodeStatus {
            online: !peers.is_empty(),
            peer_count: peers.len() as u32,
            external_ip: None,
            node_id: self.dht_engine.node_id().to_string(),
            uptime_secs: self.start_time.elapsed().as_secs(),
            queue_pending: pending,
            queue_completed: completed,
            queue_failed: failed,
        }
    }

    fn handle_queue_status(&self) -> Response {
        let entries = match self.operation_queue.list_pending() {
            Ok(es) => es,
            Err(e) => {
                return Response::Error {
                    code: ErrorCode::Internal,
                    message: format!("queue error: {e}"),
                }
            }
        };

        let info: Vec<tesseras_rpc::QueueEntryInfo> = entries
            .into_iter()
            .map(|e| tesseras_rpc::QueueEntryInfo {
                id: e.id,
                op_type: format!("{:?}", e.operation),
                status: e.status.clone(),
                created_at: e.created_at.to_rfc3339(),
                error: e.error.clone(),
            })
            .collect();

        Response::QueueEntries { entries: info }
    }

    fn handle_circle_create(&self, name: &str) -> Response {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut key = [0u8; 32];
        rng.fill(&mut key);

        match self.circle_repo.create_circle(name, &key) {
            Ok(()) => Response::CircleCreated {
                name: name.to_string(),
            },
            Err(e) => {
                let msg = e.to_string();
                let code = if msg.contains("already exists") || msg.contains("UNIQUE") {
                    ErrorCode::AlreadyExists
                } else {
                    ErrorCode::Internal
                };
                Response::Error {
                    code,
                    message: format!("failed to create circle: {e}"),
                }
            }
        }
    }

    fn handle_circle_delete(&self, name: &str) -> Response {
        match self.circle_repo.delete_circle(name) {
            Ok(()) => Response::CircleDeleted {
                name: name.to_string(),
            },
            Err(e) => Response::Error {
                code: ErrorCode::CircleNotFound,
                message: format!("failed to delete circle: {e}"),
            },
        }
    }

    fn handle_circle_add_member(&self, circle: &str, alias: &str, pubkey: &str) -> Response {
        match self.circle_repo.add_member(circle, alias, pubkey, &[]) {
            Ok(()) => Response::CircleMemberAdded,
            Err(e) => Response::Error {
                code: ErrorCode::Internal,
                message: format!("failed to add member: {e}"),
            },
        }
    }

    fn handle_circle_remove_member(&self, circle: &str, alias: &str) -> Response {
        match self.circle_repo.remove_member(circle, alias) {
            Ok(()) => Response::CircleMemberRemoved,
            Err(e) => Response::Error {
                code: ErrorCode::Internal,
                message: format!("failed to remove member: {e}"),
            },
        }
    }

    fn handle_circle_list(&self) -> Response {
        let circles = match self.circle_repo.list_circles() {
            Ok(cs) => cs,
            Err(e) => {
                return Response::Error {
                    code: ErrorCode::Internal,
                    message: format!("failed to list circles: {e}"),
                }
            }
        };

        let infos: Vec<tesseras_rpc::CircleInfo> = circles
            .into_iter()
            .map(|c| {
                let member_count = self
                    .circle_repo
                    .list_members(&c.name)
                    .map(|ms| ms.len() as u32)
                    .unwrap_or(0);
                tesseras_rpc::CircleInfo {
                    name: c.name,
                    member_count,
                    created_at: c.created_at.to_rfc3339(),
                }
            })
            .collect();

        Response::Circles { circles: infos }
    }

    async fn handle_publish(&self, hash: tesseras_core::ContentHash) -> Response {
        // 1. Pack tessera from storage
        let packed = match crate::rpc::pack::pack_tessera(
            &hash,
            self.tessera_repo.as_ref(),
            self.memory_repo.as_ref(),
            self.blob_store.as_ref(),
        ) {
            Ok(data) => data,
            Err(e) => {
                let msg = e.to_string();
                let code = if msg.contains("not found") {
                    ErrorCode::NotFound
                } else {
                    ErrorCode::Internal
                };
                return Response::Error {
                    code,
                    message: msg,
                };
            }
        };

        // 2. Replicate to network
        match self.replication.replicate_tessera(&hash, &packed).await {
            Ok(report) => Response::Published {
                hash,
                fragments_created: report.fragments_distributed as u32,
            },
            Err(e) => Response::Error {
                code: ErrorCode::Internal,
                message: format!("replication failed: {e}"),
            },
        }
    }

    async fn handle_fetch(&self, hash: tesseras_core::ContentHash) -> Response {
        // Fetch from network/local fragments
        let data = match self.replication.fetch_tessera(&hash).await {
            Ok(d) => d,
            Err(e) => {
                return Response::Error {
                    code: ErrorCode::NotFound,
                    message: format!("failed to fetch tessera: {e}"),
                };
            }
        };

        // Unpack
        let files = match tesseras_core::pack::unpack(&data) {
            Ok(f) => f,
            Err(e) => {
                return Response::Error {
                    code: ErrorCode::Internal,
                    message: format!("failed to unpack tessera: {e}"),
                };
            }
        };

        // Import into local storage (tessera repo + memory repo + blob store)
        let (memory_count, total_bytes) = match crate::rpc::import::import_tessera(
            &files,
            self.tessera_repo.as_ref(),
            self.memory_repo.as_ref(),
            self.blob_store.as_ref(),
        ) {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!(error = %e, "failed to import fetched tessera into storage");
                // Fall back to CAS-only storage
                let mut total_bytes: u64 = 0;
                let mut seen_memories = std::collections::HashSet::new();
                for file in &files {
                    total_bytes += file.data.len() as u64;
                    if let Some(rest) = file.path.strip_prefix("memories/") {
                        if let Some(mem_hash) = rest.split('/').next() {
                            seen_memories.insert(mem_hash.to_string());
                        }
                    }
                    if let Err(e) = self.cas.put(&file.data) {
                        tracing::warn!(path = %file.path, error = %e, "failed to store fetched blob");
                    }
                }
                (seen_memories.len() as u32, total_bytes)
            }
        };

        Response::Fetched {
            hash,
            memories: memory_count,
            bytes: total_bytes,
        }
    }

    async fn handle_tessera_status(&self, hash: tesseras_core::ContentHash) -> Response {
        // Check if tessera exists locally
        let exists = match self.tessera_repo.exists(&hash) {
            Ok(e) => e,
            Err(e) => {
                return Response::Error {
                    code: ErrorCode::Internal,
                    message: format!("storage error: {e}"),
                };
            }
        };

        if !exists {
            return Response::Error {
                code: ErrorCode::NotFound,
                message: format!("tessera not found: {hash}"),
            };
        }

        // Query replication status
        match self.replication.status(&hash) {
            Ok(status) => {
                let (state, fragments_total, fragments_placed) = match &status.health {
                    tesseras_replication::ReplicationHealth::Healthy => (
                        tesseras_rpc::PublishState::Healthy,
                        status.fragments_held as u32,
                        status.fragments_held as u32,
                    ),
                    tesseras_replication::ReplicationHealth::Degraded { live, target } => (
                        tesseras_rpc::PublishState::Replicated,
                        *target as u32,
                        *live as u32,
                    ),
                    tesseras_replication::ReplicationHealth::Critical { live, target } => (
                        tesseras_rpc::PublishState::Publishing,
                        *target as u32,
                        *live as u32,
                    ),
                };

                // If no fragments at all, it's still local-only
                let state = if status.fragments_held == 0 {
                    tesseras_rpc::PublishState::Local
                } else {
                    state
                };

                Response::LegacyStatus {
                    hash,
                    state,
                    fragments_total,
                    fragments_placed,
                    peers_holding: 0, // TODO: peer count from DHT
                }
            }
            Err(e) => Response::Error {
                code: ErrorCode::Internal,
                message: format!("replication status error: {e}"),
            },
        }
    }
}

/// Resolve a user-provided hash string (full hex, full base32, or prefix) to a ContentHash.
fn resolve_hash(
    hash_str: &str,
    repo: &dyn TesseraRepository,
) -> Result<tesseras_core::ContentHash, Response> {
    use tesseras_core::types::HashPrefix;

    let prefix = HashPrefix::parse(hash_str).map_err(|_| Response::Error {
        code: ErrorCode::InvalidInput,
        message: format!("invalid hash: {hash_str}"),
    })?;

    match prefix {
        HashPrefix::Exact(h) => Ok(h),
        HashPrefix::HexPrefix(hex) => match repo.find_by_hex_prefix(&hex) {
            Ok(matches) if matches.len() == 1 => Ok(matches[0].hash),
            Ok(matches) if matches.is_empty() => Err(Response::Error {
                code: ErrorCode::NotFound,
                message: format!("tessera not found: {hash_str}"),
            }),
            Ok(matches) => Err(Response::Error {
                code: ErrorCode::InvalidInput,
                message: format!("ambiguous prefix '{hash_str}': {} matches", matches.len()),
            }),
            Err(e) => Err(Response::Error {
                code: ErrorCode::Internal,
                message: format!("storage error: {e}"),
            }),
        },
        HashPrefix::Base32Prefix {
            hex_prefix,
            base32_prefix,
        } => {
            match repo.find_by_hex_prefix(&hex_prefix) {
                Ok(matches) => {
                    // Post-filter: only keep matches whose base32 starts with the prefix
                    let filtered: Vec<_> = matches
                        .into_iter()
                        .filter(|t| t.hash.to_base32().starts_with(&base32_prefix))
                        .collect();
                    match filtered.len() {
                        1 => Ok(filtered[0].hash),
                        0 => Err(Response::Error {
                            code: ErrorCode::NotFound,
                            message: format!("tessera not found: {hash_str}"),
                        }),
                        n => Err(Response::Error {
                            code: ErrorCode::InvalidInput,
                            message: format!("ambiguous prefix '{hash_str}': {n} matches"),
                        }),
                    }
                }
                Err(e) => Err(Response::Error {
                    code: ErrorCode::Internal,
                    message: format!("storage error: {e}"),
                }),
            }
        }
    }
}
