use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, uniffi::Object)]
pub struct CollectionMetadata {
    pub header: [u8; 13], // Must contain "CollectionV0."
    pub names: Vec<String>,
}
