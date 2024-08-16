pub type IrohResult<T> = Result<T, IrohError>;

#[derive(Debug, Clone, uniffi::Error)]
pub enum IrohError {
    NodeError(String),
    DownloadError(String),
    InvalidMetadata(String),
    InvalidTicket,
    UnsupportedFormat,
    SendError,
    Unknown,
}

impl std::fmt::Display for IrohError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IrohError::InvalidTicket => write!(f, "Invalid ticket"),
            IrohError::UnsupportedFormat => write!(f, "Unsupported format"),
            IrohError::SendError => write!(f, "Send error"),
            IrohError::Unknown => write!(f, "Unknown error"),
            IrohError::NodeError(e) => write!(f, "Node error: {}", e),
            IrohError::DownloadError(e) => write!(f, "Download error: {}", e),
            IrohError::InvalidMetadata(e) => write!(f, "Invalid metadata: {}", e),
        }
    }
}