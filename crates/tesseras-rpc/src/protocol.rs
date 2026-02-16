use serde::{Deserialize, Serialize};
use tesseras_core::ContentHash;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Publish { hash: ContentHash },
    Fetch { hash: ContentHash },
    Status { hash: ContentHash },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Published {
        hash: ContentHash,
        fragments_created: u32,
    },
    Fetched {
        hash: ContentHash,
        memories: u32,
        bytes: u64,
    },
    Status {
        hash: ContentHash,
        state: PublishState,
        fragments_total: u32,
        fragments_placed: u32,
        peers_holding: u32,
    },
    Error {
        code: crate::error::ErrorCode,
        message: String,
    },
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
    fn response_status_roundtrip() {
        let hash = ContentHash::new([0xef; 32]);
        let resp = Response::Status {
            hash,
            state: PublishState::Replicated,
            fragments_total: 24,
            fragments_placed: 18,
            peers_holding: 4,
        };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: Response = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            Response::Status {
                fragments_total,
                fragments_placed,
                peers_holding,
                ..
            } => {
                assert_eq!(fragments_total, 24);
                assert_eq!(fragments_placed, 18);
                assert_eq!(peers_holding, 4);
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
}
