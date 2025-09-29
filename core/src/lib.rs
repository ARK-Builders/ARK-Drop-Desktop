pub mod adapter;
pub mod error;

use adapter::{IrohInstanceAdapter, TicketWrapper};
use error::{IrohError, IrohResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{mpsc::Sender, Arc};

// Re-export types from adapter for direct use
pub use adapter::{Collection, TicketWrapper as BlobTicket};

// Main IrohInstance - uses ark-core internally
pub struct IrohInstance {
    adapter: IrohInstanceAdapter,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileTransfer {
    pub name: String,
    pub transferred: u64,
    pub total: u64,
}

pub struct FileTransferHandle(pub Sender<Vec<FileTransfer>>);

impl IrohInstance {
    pub async fn new() -> IrohResult<Self> {
        let adapter = IrohInstanceAdapter::new().await?;
        Ok(Self { adapter })
    }

    pub async fn send_files(&self, files: Vec<PathBuf>) -> IrohResult<BlobTicket> {
        self.adapter.send_files(files).await
    }

    pub async fn receive_files(
        &self,
        ticket: String,
        handle_chunk: Arc<FileTransferHandle>,
    ) -> IrohResult<Collection> {
        self.adapter.receive_files(ticket, handle_chunk).await
    }
}

// Tests will be updated later to work with the new implementation
#[cfg(test)]
mod test {
    use std::{
        fs,
        path::PathBuf,
        sync::{mpsc::channel, Arc},
    };

    use crate::{FileTransfer, FileTransferHandle, IrohInstance};

    #[tokio::test]
    async fn test_send_files_basic() {
        let instance = IrohInstance::new().await.unwrap();

        // Create a test file
        let file1 = PathBuf::from("./test_file1.txt");
        std::fs::write(&file1, "content1").unwrap();
        let files = vec![fs::canonicalize(&file1).unwrap()];

        // Call send_files and verify the result
        let ticket = instance.send_files(files).await.unwrap();
        assert!(!ticket.to_string().is_empty(), "Ticket should not be empty");

        // Clean up
        std::fs::remove_file(&file1).unwrap();
    }

    // Additional tests to be implemented after validation
}
