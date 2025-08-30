pub mod error;

use error::{IrohError, IrohResult};
use futures_buffered::try_join_all;
use futures_lite::stream::StreamExt;
use iroh::{
    client::blobs::{AddOutcome, WrapOption},
    node::Node,
};
use iroh_base::ticket::BlobTicket;
use iroh_blobs::{
    format::collection::Collection, get::db::DownloadProgress, util::SetTagOption, BlobFormat,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{
    collections::BTreeMap,
    iter::Iterator,
    sync::{mpsc::Sender, Arc},
    vec,
};
use std::{path::PathBuf, str::FromStr};

pub struct IrohNode(pub Node<iroh_blobs::store::mem::Store>);

pub struct IrohInstance {
    node: Arc<IrohNode>,
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
        let node = Node::memory()
            .spawn()
            .await
            .map_err(|e| IrohError::NodeError(e.to_string()))?;
        Ok(Self {
            node: Arc::new(IrohNode(node)),
        })
    }

    pub fn get_node(&self) -> Arc<IrohNode> {
        self.node.clone()
    }

    pub async fn send_files(&self, files: Vec<PathBuf>) -> IrohResult<BlobTicket> {
        let outcomes = import_blobs(self, files).await?;

        let collection = outcomes
            .into_iter()
            .map(|(path, outcome)| {
                let name = path
                    .file_name()
                    .expect("The file name is not valid.")
                    .to_string_lossy()
                    .to_string();

                let hash = outcome.hash;
                (name, hash)
            })
            .collect();

        let (hash, _) = self
            .node
            .0
            .blobs()
            .create_collection(collection, SetTagOption::Auto, Default::default())
            .await
            .map_err(|e| IrohError::NodeError(e.to_string()))?;

        self.node
            .0
            .blobs()
            .share(hash, BlobFormat::HashSeq, Default::default())
            .await
            .map_err(|e| IrohError::NodeError(e.to_string()))
    }

    pub async fn receive_files(
        &self,
        ticket: String,
        handle_chunk: Arc<FileTransferHandle>,
    ) -> IrohResult<Collection> {
        let ticket = BlobTicket::from_str(&ticket).map_err(|_| IrohError::InvalidTicket)?;

        if ticket.format() != BlobFormat::HashSeq {
            return Err(IrohError::UnsupportedFormat);
        }

        let mut download_stream = self
            .node
            .0
            .blobs()
            .download_hash_seq(ticket.hash(), ticket.node_addr().clone())
            .await
            .map_err(|e| IrohError::DownloadError(e.to_string()))?;

        let mut files: Vec<FileTransfer> = Vec::new();

        let mut map: BTreeMap<u64, String> = BTreeMap::new();

        let debug_log = std::env::var("DROP_DEBUG_LOG").is_ok();
        let temp_dir = std::env::temp_dir();

        while let Some(event) = download_stream.next().await {
            let event = event.map_err(|e| IrohError::DownloadError(e.to_string()))?;

            if debug_log {
                let mut log_file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(temp_dir.join("drop_debug.log"))
                    .expect("Failed to open log file");
                writeln!(log_file, "{:?}", event).expect("Failed to write to log file");
            }

            match event {
                DownloadProgress::FoundHashSeq { .. } => {}

                DownloadProgress::AllDone(_) => {
                    let collection = self
                        .node
                        .0
                        .blobs()
                        .get_collection(ticket.hash())
                        .await
                        .map_err(|e: anyhow::Error| IrohError::DownloadError(e.to_string()))?;
                    files = vec![];
                    for (name, hash) in collection.iter() {
                        let content = self
                            .node
                            .0
                            .blobs()
                            .read_to_bytes(*hash)
                            .await
                            .map_err(|e| IrohError::DownloadError(e.to_string()))?;
                        files.push(FileTransfer {
                            name: name.clone(),
                            transferred: content.len() as u64,
                            total: content.len() as u64,
                        });
                    }
                    handle_chunk
                        .0
                        .send(files.clone())
                        .map_err(|_| IrohError::SendError)?;

                    if debug_log {
                        println!("[DEBUG FILE]: {:?}", temp_dir.join("drop_debug.log"));
                    }

                    return Ok(collection);
                }

                DownloadProgress::Done { id } => {
                    if let Some(name) = map.get(&id) {
                        if let Some(file) = files.iter_mut().find(|file| file.name == *name) {
                            file.transferred = file.total;
                        }
                    }
                    handle_chunk
                        .0
                        .send(files.clone())
                        .map_err(|_| IrohError::SendError)?;
                }

                DownloadProgress::Found { id, size, .. } => {
                    // Track the download with a temporary name
                    let name = format!("file_{}", id);
                    files.push(FileTransfer {
                        name: name.clone(),
                        transferred: 0,
                        total: size,
                    });
                    handle_chunk
                        .0
                        .send(files.clone())
                        .map_err(|_| IrohError::SendError)?;
                    map.insert(id, name);

                    if debug_log {
                        let mut log_file = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(temp_dir.join("drop_debug.log"))
                            .expect("Failed to open log file");
                        writeln!(log_file, "{:?}", event).expect("Failed to write to log file");
                    }
                }

                DownloadProgress::Progress { id, offset } => {
                    if let Some(name) = map.get(&id) {
                        if let Some(file) = files.iter_mut().find(|file| file.name == **name) {
                            file.transferred = offset;
                        }
                    }
                    handle_chunk
                        .0
                        .send(files.clone())
                        .map_err(|_| IrohError::SendError)?;
                }

                DownloadProgress::FoundLocal { size, .. } => {
                    // Local file found, update if we're tracking it
                    if let Some(file) = files.last_mut() {
                        file.transferred = size.value();
                        file.total = size.value();
                        handle_chunk
                            .0
                            .send(files.clone())
                            .map_err(|_| IrohError::SendError)?;
                    }
                }

                _ => {}
            }
        }

        if debug_log {
            println!("[DEBUG FILE]: {:?}", temp_dir.join("drop_debug.log"));
        }

        // This should not be reached if AllDone was processed
        // Try to get the collection as a fallback
        let collection = self
            .node
            .0
            .blobs()
            .get_collection(ticket.hash())
            .await
            .map_err(|e| IrohError::DownloadError(e.to_string()))?;

        Ok(collection)
    }
}

pub async fn import_blobs(
    iroh: &IrohInstance,
    paths: Vec<PathBuf>,
) -> IrohResult<Vec<(PathBuf, AddOutcome)>> {
    let outcomes = paths.into_iter().map(|path| async move {
        let add_progress = iroh
            .get_node()
            .0
            .blobs()
            .add_from_path(path.clone(), true, SetTagOption::Auto, WrapOption::NoWrap)
            .await;

        match add_progress {
            Ok(add_progress) => {
                let outcome = add_progress.finish().await;
                if let Ok(progress) = outcome {
                    Ok((path.clone(), progress))
                } else {
                    Err(IrohError::NodeError(format!(
                        "Failed to import blob: {:?}",
                        outcome
                    )))
                }
            }
            Err(e) => Err(IrohError::NodeError(e.to_string())),
        }
    });

    try_join_all(outcomes).await
}

#[cfg(test)]
mod test {
    use std::{
        fs,
        path::PathBuf,
        sync::{mpsc::channel, Arc},
    };

    use tokio;

    use crate::{FileTransfer, FileTransferHandle, IrohInstance};

    #[tokio::test]
    async fn test_send_files() {
        let instance = IrohInstance::new().await.unwrap();

        // Create files directly in the current directory
        let file1 = PathBuf::from("./test_file1.txt");
        let file2 = PathBuf::from("./test_file2.txt");
        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();
        let files = vec![
            fs::canonicalize(&file1).unwrap(),
            fs::canonicalize(&file2).unwrap(),
        ];

        // Call send_files and verify the result
        let ticket = instance.send_files(files).await.unwrap();
        assert!(!ticket.to_string().is_empty(), "Ticket should not be empty");

        // Clean up
        std::fs::remove_file(&file1).unwrap();
        std::fs::remove_file(&file2).unwrap();
    }

    #[tokio::test]
    async fn test_receive_files() {
        let instance = IrohInstance::new().await.unwrap();

        let file1 = PathBuf::from("test_file1.txt");
        let file2 = PathBuf::from("test_file2.txt");
        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();
        let files = vec![
            fs::canonicalize(&file1).unwrap(),
            fs::canonicalize(&file2).unwrap(),
        ];
        let ticket = instance.send_files(files).await.unwrap();
        let ticket_str = ticket.to_string();

        let (tx, _rx) = channel::<Vec<FileTransfer>>();
        let handle = Arc::new(crate::FileTransferHandle(tx));

        let collection = instance.receive_files(ticket_str, handle).await.unwrap();

        // Verify the collection
        let names: Vec<String> = collection.iter().map(|(name, _)| name.clone()).collect();
        assert_eq!(names.len(), 2, "Collection should contain two files");
        assert!(
            names.contains(&"test_file1.txt".to_string()),
            "Collection should contain test_file1.txt"
        );
        assert!(
            names.contains(&"test_file2.txt".to_string()),
            "Collection should contain test_file2.txt"
        );

        // Clean up
        std::fs::remove_file(&file1).unwrap();
        std::fs::remove_file(&file2).unwrap();
    }

    #[tokio::test]
    async fn test_large_file_transfer() {
        // Create a 1MB test file
        let test_file = PathBuf::from("test_1mb_file.bin");
        let file_size = 1024 * 1024; // 1MB
        let test_data: Vec<u8> = (0..file_size).map(|i| (i % 256) as u8).collect();
        fs::write(&test_file, &test_data).expect("Failed to create test file");

        println!("Created test file with size: {} bytes", file_size);

        // Create a single instance for both sending and receiving (like the other tests)
        let instance = IrohInstance::new().await.unwrap();

        // Send the file
        let files = vec![fs::canonicalize(&test_file).unwrap()];
        let ticket = instance.send_files(files).await.unwrap();
        let ticket_str = ticket.to_string();

        println!("Generated ticket: {}", ticket_str);

        // Set up transfer monitoring
        let (tx, rx) = channel::<Vec<FileTransfer>>();
        let handle = Arc::new(FileTransferHandle(tx));

        // Receive the file using the same instance
        let collection = instance
            .receive_files(ticket_str.clone(), handle)
            .await
            .unwrap();

        // Verify the collection
        for (name, hash) in collection.iter() {
            println!("Received file: {} with hash: {}", name, hash);

            // Read the received content
            let content = instance
                .get_node()
                .0
                .blobs()
                .read_to_bytes(*hash)
                .await
                .expect("Failed to read blob");

            println!("Received file size: {} bytes", content.len());

            // Verify size
            assert_eq!(
                content.len(),
                file_size,
                "File size mismatch! Expected {} bytes, got {} bytes",
                file_size,
                content.len()
            );

            // Verify content
            assert_eq!(content.to_vec(), test_data, "File content mismatch!");

            println!("✅ File transfer successful! Size and content match.");
        }

        // Check transfer progress messages
        while let Ok(progress) = rx.try_recv() {
            for file in progress {
                println!(
                    "Transfer progress: {} - {}/{} bytes",
                    file.name, file.transferred, file.total
                );
            }
        }

        // Clean up
        fs::remove_file(&test_file).unwrap();
    }

    #[tokio::test]
    async fn test_multiple_files_transfer() {
        // Create multiple test files of different sizes
        let file1 = PathBuf::from("test_file_100kb.bin");
        let file2 = PathBuf::from("test_file_500kb.bin");
        let file3 = PathBuf::from("test_file_1mb.bin");

        let data1: Vec<u8> = (0..102400).map(|i| (i % 256) as u8).collect(); // 100KB
        let data2: Vec<u8> = (0..512000).map(|i| (i % 256) as u8).collect(); // 500KB
        let data3: Vec<u8> = (0..1048576).map(|i| (i % 256) as u8).collect(); // 1MB

        fs::write(&file1, &data1).unwrap();
        fs::write(&file2, &data2).unwrap();
        fs::write(&file3, &data3).unwrap();

        let instance = IrohInstance::new().await.unwrap();

        // Send multiple files
        let files = vec![
            fs::canonicalize(&file1).unwrap(),
            fs::canonicalize(&file2).unwrap(),
            fs::canonicalize(&file3).unwrap(),
        ];

        let ticket = instance.send_files(files).await.unwrap();
        let ticket_str = ticket.to_string();

        let (tx, _rx) = channel::<Vec<FileTransfer>>();
        let handle = Arc::new(FileTransferHandle(tx));

        let collection = instance.receive_files(ticket_str, handle).await.unwrap();

        // Verify all files were received
        let names: Vec<String> = collection.iter().map(|(name, _)| name.clone()).collect();
        assert_eq!(names.len(), 3, "Should receive 3 files");

        // Verify each file
        for (name, hash) in collection.iter() {
            let content = instance
                .get_node()
                .0
                .blobs()
                .read_to_bytes(*hash)
                .await
                .unwrap();

            let expected_size = if name.contains("100kb") {
                102400
            } else if name.contains("500kb") {
                512000
            } else {
                1048576
            };

            assert_eq!(content.len(), expected_size, "File {} size mismatch", name);

            println!(
                "✅ File {} transferred successfully ({} bytes)",
                name,
                content.len()
            );
        }

        // Clean up
        fs::remove_file(&file1).unwrap();
        fs::remove_file(&file2).unwrap();
        fs::remove_file(&file3).unwrap();
    }
}
