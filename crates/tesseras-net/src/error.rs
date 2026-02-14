#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("send failed: {0}")]
    SendFailed(String),

    #[error("receive failed: {0}")]
    ReceiveFailed(String),

    #[error("message too large: {size} bytes (max {max})")]
    MessageTooLarge { size: usize, max: usize },

    #[error("invalid message: {0}")]
    InvalidMessage(String),

    #[error("timeout")]
    Timeout,

    #[error("transport closed")]
    Closed,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
