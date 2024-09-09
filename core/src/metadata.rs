use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CollectionMetadata {
    pub header: [u8; 13], // Must contain "CollectionV0."
    pub names: Vec<String>,
}
