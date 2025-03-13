pub mod erorr;
pub mod metadata;

use erorr::{IrohError, IrohResult};
use futures_buffered::try_join_all;
use futures_lite::stream::StreamExt;
use iroh::{
    client::blobs::{AddOutcome, WrapOption},
    node::Node,
};
use iroh_base::ticket::BlobTicket;
use iroh_blobs::{
    format::collection::Collection, get::db::DownloadProgress, hashseq::HashSeq,
    util::SetTagOption, BlobFormat,
};
use metadata::CollectionMetadata;
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

        let mut curr_metadata: Option<CollectionMetadata> = None;
        let mut curr_hashseq: Option<HashSeq> = None;
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
                DownloadProgress::FoundHashSeq { hash, .. } => {
                    let hashseq = self
                        .node
                        .0
                        .blobs()
                        .read_to_bytes(hash)
                        .await
                        .map_err(|e| IrohError::DownloadError(e.to_string()))?;
                    let hashseq = HashSeq::try_from(hashseq)
                        .map_err(|e| IrohError::InvalidMetadata(e.to_string()))?;

                    let metadata_hash = hashseq
                        .iter()
                        .next()
                        .ok_or(IrohError::InvalidMetadata("hashseq is empty".to_string()))?;
                    let metadata_bytes = self
                        .node
                        .0
                        .blobs()
                        .read_to_bytes(metadata_hash)
                        .await
                        .map_err(|e| IrohError::DownloadError(e.to_string()))?;

                    let metadata: CollectionMetadata = postcard::from_bytes(&metadata_bytes)
                        .map_err(|e| IrohError::InvalidMetadata(e.to_string()))?;

                    // The hash sequence should have one more element than the metadata
                    // because the first element is the metadata itself
                    if metadata.names.len() + 1 != hashseq.len() {
                        return Err(IrohError::InvalidMetadata(
                            "metadata does not match hashseq".to_string(),
                        ));
                    }
                    curr_hashseq = Some(hashseq);
                    curr_metadata = Some(metadata);
                }

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
                        files.push({
                            FileTransfer {
                                name: name.clone(),
                                transferred: content.len() as u64,
                                total: content.len() as u64,
                            }
                        })
                    }
                    handle_chunk
                        .0
                        .send(files.clone())
                        .map_err(|_| IrohError::SendError)?;

                    if debug_log {
                        println!("[DEBUG FILE]: {:?}", temp_dir.join("drop_debug.log"));
                    }

                    return Ok(collection.into());
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

                DownloadProgress::Found { id, hash, size, .. } => {
                    if let (Some(hashseq), Some(metadata)) = (&curr_hashseq, &curr_metadata) {
                        if let Some(idx) = hashseq.iter().position(|h| h == hash) {
                            if idx >= 1 && idx <= metadata.names.len() {
                                if let Some(name) = metadata.names.get(idx - 1) {
                                    files.push(FileTransfer {
                                        name: name.clone(),
                                        transferred: 0,
                                        total: size,
                                    });
                                    handle_chunk
                                        .0
                                        .send(files.clone())
                                        .map_err(|_| IrohError::SendError)?;
                                    map.insert(id, name.clone());
                                }
                            }
                        } else {
                            return Err(IrohError::Unreachable(
                                file!().to_string(),
                                line!().to_string(),
                            ));
                        }
                    }
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

                DownloadProgress::FoundLocal { hash, size, .. } => {
                    if let (Some(hashseq), Some(metadata)) = (&curr_hashseq, &curr_metadata) {
                        if let Some(idx) = hashseq.iter().position(|h| h == hash) {
                            if idx >= 1 && idx <= metadata.names.len() {
                                if let Some(name) = metadata.names.get(idx - 1) {
                                    if let Some(file) =
                                        files.iter_mut().find(|file| file.name == *name)
                                    {
                                        file.transferred = size.value();
                                        file.total = size.value();
                                        handle_chunk
                                            .0
                                            .send(files.clone())
                                            .map_err(|_| IrohError::SendError)?;
                                    }
                                }
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        if debug_log {
            println!("[DEBUG FILE]: {:?}", temp_dir.join("drop_debug.log"));
        }

        let collection = self
            .node
            .0
            .blobs()
            .get_collection(ticket.hash())
            .await
            .map_err(|e| IrohError::DownloadError(e.to_string()))?;

        Ok(collection.into())
    }
}

pub async fn import_blobs<'a>(
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

    use crate::{FileTransfer, IrohInstance};

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
        // Create an in-memory IrohInstance
        let send_instance = IrohInstance::new().await.unwrap();
        let receive_instance = IrohInstance::new().await.unwrap();

        let file1 = PathBuf::from("test_file1.txt");
        let file2 = PathBuf::from("test_file2.txt");
        std::fs::write(&file1, "content1").unwrap();
        std::fs::write(&file2, "content2").unwrap();
        let files = vec![
            fs::canonicalize(&file1).unwrap(),
            fs::canonicalize(&file2).unwrap(),
        ];
        let ticket = send_instance.send_files(files).await.unwrap();
        let ticket_str = ticket.to_string();

        let (tx, mut rx) = channel::<Vec<FileTransfer>>();
        let handle = Arc::new(crate::FileTransferHandle(tx)); // Assuming FileTransferHandle wraps the sender

        let collection = receive_instance
            .receive_files(ticket_str, handle)
            .await
            .unwrap();

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
}
