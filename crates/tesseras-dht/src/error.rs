#[derive(Debug, thiserror::Error)]
pub enum DhtError {
    #[error("lookup failed: {0}")]
    LookupFailed(String),

    #[error("bootstrap failed: no reachable seeds")]
    BootstrapFailed,

    #[error("publish failed: insufficient acks ({got}/{needed})")]
    PublishFailed { got: usize, needed: usize },

    #[error("invalid node identity: {0}")]
    InvalidIdentity(String),

    #[error("transport error: {0}")]
    Transport(#[from] tesseras_net::NetError),

    #[error("codec error: {0}")]
    Codec(String),

    #[error("rpc failed: {0}")]
    RpcFailed(String),

    #[error("shutdown")]
    Shutdown,
}
