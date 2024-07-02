use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, uniffi::Record)]
pub struct CollectionMetadata {
    pub header: Vec<u8>, // Must contain "CollectionV0."
    pub names: Vec<String>,
}
