use anyhow::Result;
use bytes::Bytes;
use iroh_blobs::format::collection::Collection;

use crate::erorr::IrohError;

pub struct CollectionMetadata {
    pub file_count: u64,
    pub file_names: Vec<String>,
}

impl CollectionMetadata {
    pub fn from_bytes(bytes: Bytes) -> Result<Self, IrohError> {
        let header = Collection::HEADER;
        if !bytes.starts_with(header) {
            return Err(IrohError::InvalidMetadata);
        }

        let string = String::from_utf8_lossy(&bytes[header.len()..]);

        let file_names: Vec<String> = string
            .split_whitespace()
            .map(|s| s.to_string())
            .skip(1)
            .collect();

        Ok(CollectionMetadata {
            file_count: file_names.len() as u64,
            file_names,
        })
    }
}
