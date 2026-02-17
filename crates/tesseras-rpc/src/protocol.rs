use serde::{Deserialize, Serialize};
use tesseras_core::{ContentHash, NodeInfo};

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    // --- Daily operations ---
    Push {
        paths: Vec<String>,
        visibility: String,
        circle: Option<String>,
        name: Option<String>,
        tags: Vec<String>,
    },
    Pull {
        target: PullTarget,
    },
    List {
        circle: Option<String>,
    },
    Show {
        hash: String,
    },
    Delete {
        hash: String,
    },

    // --- Circle management ---
    CircleCreate {
        name: String,
    },
    CircleDelete {
        name: String,
    },
    CircleAddMember {
        circle: String,
        alias: String,
        pubkey: String,
    },
    CircleRemoveMember {
        circle: String,
        alias: String,
    },
    CircleList {
        name: Option<String>,
    },

    // --- Contact management ---
    ContactAdd {
        alias: String,
        pubkey: String,
    },
    ContactRemove {
        alias: String,
    },
    ContactList,

    // --- Status ---
    Status,
    QueueStatus,

    // --- Legacy (kept for backward compat during transition) ---
    Publish {
        hash: ContentHash,
    },
    Fetch {
        hash: ContentHash,
    },
    TesseraStatus {
        hash: ContentHash,
    },
    Peers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PullTarget {
    Hash(String),
    Alias(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Pushed {
        hash: String,
        memory_count: u32,
        queued: bool,
    },
    Pulled {
        hash: String,
        memories: u32,
        bytes: u64,
        queued: bool,
    },
    Listed {
        records: Vec<TesseraInfo>,
    },
    Shown {
        record: TesseraInfo,
        memories: Vec<MemoryInfo>,
    },
    Deleted {
        hash: String,
        tombstone_published: bool,
    },

    CircleCreated {
        name: String,
    },
    CircleDeleted {
        name: String,
    },
    CircleMemberAdded,
    CircleMemberRemoved,
    Circles {
        circles: Vec<CircleInfo>,
    },

    ContactAdded,
    ContactRemoved,
    Contacts {
        contacts: Vec<ContactInfo>,
    },

    NodeStatus {
        online: bool,
        peer_count: u32,
        external_ip: Option<String>,
        node_id: String,
        uptime_secs: u64,
        queue_pending: u32,
        queue_completed: u32,
        queue_failed: u32,
    },
    QueueEntries {
        entries: Vec<QueueEntryInfo>,
    },

    // Legacy
    Published {
        hash: ContentHash,
        fragments_created: u32,
    },
    Fetched {
        hash: ContentHash,
        memories: u32,
        bytes: u64,
    },
    LegacyStatus {
        hash: ContentHash,
        state: PublishState,
        fragments_total: u32,
        fragments_placed: u32,
        peers_holding: u32,
    },
    PeerList {
        peers: Vec<NodeInfo>,
    },

    Error {
        code: crate::error::ErrorCode,
        message: String,
    },
}

/// Lightweight tessera info for list/show responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TesseraInfo {
    pub hash: String,
    pub created_at: String,
    pub memory_count: u32,
    pub size_bytes: u64,
    pub visibility: String,
    pub is_mine: bool,
}

/// Memory info for show response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub hash: String,
    pub memory_type: String,
    pub media_path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircleInfo {
    pub name: String,
    pub member_count: u32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub alias: String,
    pub pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntryInfo {
    pub id: i64,
    pub op_type: String,
    pub status: String,
    pub created_at: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublishState {
    Local,
    Publishing,
    Replicated,
    Healthy,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseras_core::ContentHash;

    #[test]
    fn request_publish_roundtrip() {
        let hash = ContentHash::new([0xab; 32]);
        let req = Request::Publish { hash };
        let bytes = rmp_serde::to_vec(&req).unwrap();
        let decoded: Request = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            Request::Publish { hash: h } => assert_eq!(h, hash),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn response_published_roundtrip() {
        let hash = ContentHash::new([0xcd; 32]);
        let resp = Response::Published {
            hash,
            fragments_created: 24,
        };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: Response = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            Response::Published {
                hash: h,
                fragments_created,
            } => {
                assert_eq!(h, hash);
                assert_eq!(fragments_created, 24);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn request_push_roundtrip() {
        let req = Request::Push {
            paths: vec!["/tmp/photo.jpg".to_string()],
            visibility: "private".to_string(),
            circle: None,
            name: Some("vacation".to_string()),
            tags: vec!["summer".to_string()],
        };
        let bytes = rmp_serde::to_vec(&req).unwrap();
        let decoded: Request = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            Request::Push {
                paths, visibility, ..
            } => {
                assert_eq!(paths.len(), 1);
                assert_eq!(visibility, "private");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn response_pushed_roundtrip() {
        let resp = Response::Pushed {
            hash: "abc123".to_string(),
            memory_count: 3,
            queued: true,
        };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: Response = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            Response::Pushed {
                hash,
                memory_count,
                queued,
            } => {
                assert_eq!(hash, "abc123");
                assert_eq!(memory_count, 3);
                assert!(queued);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn response_node_status_roundtrip() {
        let resp = Response::NodeStatus {
            online: true,
            peer_count: 5,
            external_ip: Some("1.2.3.4".to_string()),
            node_id: "deadbeef".to_string(),
            uptime_secs: 3600,
            queue_pending: 2,
            queue_completed: 10,
            queue_failed: 1,
        };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: Response = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            Response::NodeStatus {
                online,
                peer_count,
                uptime_secs,
                ..
            } => {
                assert!(online);
                assert_eq!(peer_count, 5);
                assert_eq!(uptime_secs, 3600);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn response_error_roundtrip() {
        let resp = Response::Error {
            code: crate::error::ErrorCode::NotFound,
            message: "tessera not found".into(),
        };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: Response = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            Response::Error { message, .. } => {
                assert_eq!(message, "tessera not found");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn request_circle_create_roundtrip() {
        let req = Request::CircleCreate {
            name: "family".to_string(),
        };
        let bytes = rmp_serde::to_vec(&req).unwrap();
        let decoded: Request = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            Request::CircleCreate { name } => assert_eq!(name, "family"),
            _ => panic!("wrong variant"),
        }
    }
}
