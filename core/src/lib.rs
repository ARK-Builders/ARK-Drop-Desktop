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
use std::{
    collections::BTreeMap,
    iter::Iterator,
    sync::{mpsc::Sender, Arc},
    vec,
};
use std::{path::PathBuf, str::FromStr};

uniffi::setup_scaffolding!();

#[derive(uniffi::Object)]
pub struct IrohNode(pub Node<iroh_blobs::store::mem::Store>);

#[derive(uniffi::Object)]
pub struct IrohInstance {
    node: Arc<IrohNode>,
}

#[derive(Debug, Serialize, Deserialize, Clone, uniffi::Record)]
pub struct FileTransfer {
    pub name: String,
    pub transferred: u64,
    pub total: u64,
}

uniffi::custom_type!(PathBuf, String);

impl UniffiCustomTypeConverter for PathBuf {
    type Builtin = String;

    fn into_custom(val: Self::Builtin) -> uniffi::Result<Self> {
        Ok(PathBuf::from(val))
    }

    fn from_custom(obj: Self) -> Self::Builtin {
        obj.to_string_lossy().to_string()
    }
}

uniffi::custom_type!(BlobTicket, String);

impl UniffiCustomTypeConverter for BlobTicket {
    type Builtin = String;

    fn into_custom(val: Self::Builtin) -> uniffi::Result<Self> {
        Ok(BlobTicket::from_str(&val)?)
    }

    fn from_custom(obj: Self) -> Self::Builtin {
        obj.to_string()
    }
}

#[derive(uniffi::Object)]
pub struct DropCollection(pub Collection);

impl From<Collection> for DropCollection {
    fn from(collection: Collection) -> Self {
        Self(collection)
    }
}

#[derive(uniffi::Object)]
pub struct FileTransferHandle(pub Sender<Vec<FileTransfer>>);

#[uniffi::export]
impl IrohInstance {
    #[uniffi::constructor]
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

    /// Accepts a list of file paths.
    ///
    /// Returns a `BlobTicket`, which is a string that
    /// can be used to retrieve the files from another node.
    pub async fn send_files(&self, files: Vec<PathBuf>) -> IrohResult<BlobTicket> {
        // Import a series of blobs from the file system paths
        let outcomes = import_blobs(self, files).await?;

        // A series of blobs is the same as a collection,
        // but we need to convert the structure slightly to implicitly create it
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

        // we now also import this collection into the node
        let (hash, _) = self
            .node
            .0
            .blobs()
            .create_collection(collection, SetTagOption::Auto, Default::default())
            .await
            .map_err(|e| IrohError::NodeError(e.to_string()))?;

        // We can now generate a ticket from this collection
        self.node
            .0
            .blobs()
            .share(hash, BlobFormat::HashSeq, Default::default())
            .await
            .map_err(|e| IrohError::NodeError(e.to_string()))
    }

    /// Accepts a `BlobTicket` and a `FileTransferHandle`
    /// (a channel to send progress updates to the client)
    ///
    /// Returns a `DropCollection` (a wrapper around a collection).
    pub async fn receive_files(
        &self,
        ticket: String,
        handle_chunk: Arc<FileTransferHandle>,
    ) -> IrohResult<DropCollection> {
        let ticket = BlobTicket::from_str(&ticket).map_err(|_| IrohError::InvalidTicket)?;

        if ticket.format() != BlobFormat::HashSeq {
            return Err(IrohError::UnsupportedFormat);
        }

        // Download the collection from the node
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

        // the download stream is a stream of download progress events
        // we can send these events to the client to update the progress
        while let Some(event) = download_stream.next().await {
            let event = event.map_err(|e| IrohError::DownloadError(e.to_string()))?;
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
                            return Err(IrohError::Unreachable(file!().to_string(), line!().to_string()));
                        }
                    } else {
                        return Err(IrohError::Unreachable(file!().to_string(), line!().to_string()));
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

        // If we reach this point, the download stream has ended without completing the download
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
                    Err(IrohError::NodeError("Failed to import blob".to_string()))
                }
            }
            Err(e) => Err(IrohError::NodeError(e.to_string())),
        }
    });

    try_join_all(outcomes).await
}
