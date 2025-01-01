use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CollectionMetadata {
    pub header: [u8; 13], // Must contain "CollectionV0."
    pub names: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileTransfer {
    pub name: String,
    pub transferred: u64,
    pub total: u64,
}
