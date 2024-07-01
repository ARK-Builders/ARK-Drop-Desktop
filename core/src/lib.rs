pub mod erorr;
pub mod metadata;

use anyhow::{Context, Result};

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
use std::{collections::BTreeMap, iter::Iterator, sync::Arc, vec};
use std::{path::PathBuf, str::FromStr};

#[derive(uniffi::Object)]
struct IrohNode(pub Node<iroh_blobs::store::mem::Store>);

uniffi::setup_scaffolding!();

#[derive(uniffi::Object)]
pub struct IrohInstance {
    node: Arc<IrohNode>,
}

#[derive(uniffi::Object)]
struct DropCollection(Collection);

#[derive(Debug, Serialize, Deserialize, Clone, uniffi::Object)]
pub struct FileTransfer {
    pub name: String,
    pub transfered: u64,
    pub total: u64,
}

#[uniffi::export]
impl IrohInstance {
    #[uniffi::constructor]
    pub async fn new() -> IrohResult<Self> {
        let node = Node::memory().spawn().await?;
        Ok(Self {
            node: Arc::new(IrohNode(node)),
        })
    }

    pub fn get_node(&self) -> Arc<IrohNode> {
        self.node.clone()
    }

    pub async fn send_files(&self, files: &[PathBuf]) -> IrohResult<BlobTicket> {
        let outcome = create_collection_from_files(self, files).await?;

        let collection = outcome
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
            .blobs
            .create_collection(collection, SetTagOption::Auto, Default::default())
            .await?;

        self.node
            .0
            .blobs
            .share(hash, BlobFormat::HashSeq, Default::default())
            .await
            .map_err(|e| e.into())
    }

    pub async fn recieve_files(
        &self,
        ticket: String,
        // closure to handle each chunk
        mut handle_chunk: impl FnMut(Vec<FileTransfer>),
    ) -> IrohResult<DropCollection> {
        let ticket = BlobTicket::from_str(&ticket)?;

        if ticket.format() != BlobFormat::HashSeq {
            return Err(IrohError::UnsupportedFormat(ticket.format()));
        }

        let mut download_stream = self
            .node
            .0
            .blobs
            .download_hash_seq(ticket.hash(), ticket.node_addr().clone())
            .await?;

        let mut metadata: Option<CollectionMetadata> = None;
        let mut hashseq: Option<HashSeq> = None;
        let mut files: Vec<FileTransfer> = Vec::new();

        let mut map: BTreeMap<u64, String> = BTreeMap::new();

        while let Some(chunk) = download_stream.next().await {
            let chunk = chunk?;
            match chunk {
                DownloadProgress::FoundHashSeq { hash, .. } => {
                    let hs = self.node.blobs.read_to_bytes(hash).await?;
                    let hs = HashSeq::try_from(hs)?;
                    let meta_hash = hs.iter().next().context("No metadata hash found")?;
                    let meta_bytes = self.node.blobs.read_to_bytes(meta_hash).await?;

                    let meta: CollectionMetadata =
                        postcard::from_bytes(&meta_bytes).context("Failed to parse metadata")?;

                    // The hash sequence should have one more element than the metadata
                    // because the first element is the metadata itself
                    if meta.names.len() + 1 != hs.len() {
                        return Err(anyhow::anyhow!("names and links length mismatch").into());
                    }
                    hashseq = Some(hs);
                    metadata = Some(meta);
                }
                DownloadProgress::AllDone(_) => {
                    let collection = self.node.blobs.get_collection(ticket.hash()).await?;
                    files = vec![];
                    for (name, hash) in collection.iter() {
                        let content = self.node.blobs.read_to_bytes(*hash).await?;
                        files.push({
                            FileTransfer {
                                name: name.clone(),
                                transfered: content.len() as u64,
                                total: content.len() as u64,
                            }
                        })
                    }
                    handle_chunk(files.clone());
                    return Ok(collection);
                }
                DownloadProgress::Done { id } => {
                    if let Some(name) = map.get(&id) {
                        if let Some(file) = files.iter_mut().find(|file| file.name == *name) {
                            file.transfered = file.total;
                        }
                    }
                    handle_chunk(files.clone());
                }
                DownloadProgress::Found { id, hash, size, .. } => {
                    if let (Some(hashseq), Some(metadata)) = (&hashseq, &metadata) {
                        if let Some(idx) = hashseq.iter().position(|h| h == hash) {
                            if idx >= 1 && idx <= metadata.names.len() {
                                if let Some(name) = metadata.names.get(idx - 1) {
                                    files.push(FileTransfer {
                                        name: name.clone(),
                                        transfered: 0,
                                        total: size,
                                    });
                                    handle_chunk(files.clone());
                                    map.insert(id, name.clone());
                                }
                            }
                        }
                    }
                }
                DownloadProgress::Progress { id, offset } => {
                    if let Some(name) = map.get(&id) {
                        if let Some(file) = files.iter_mut().find(|file| file.name == **name) {
                            file.transfered = offset;
                        }
                    }
                    handle_chunk(files.clone());
                }
                DownloadProgress::FoundLocal { hash, size, .. } => {
                    if let (Some(hashseq), Some(metadata)) = (&hashseq, &metadata) {
                        if let Some(idx) = hashseq.iter().position(|h| h == hash) {
                            if idx >= 1 && idx <= metadata.names.len() {
                                if let Some(name) = metadata.names.get(idx - 1) {
                                    if let Some(file) =
                                        files.iter_mut().find(|file| file.name == *name)
                                    {
                                        file.transfered = size.value();
                                        file.total = size.value();
                                        handle_chunk(files.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let collection = self.node.0.blobs.get_collection(ticket.hash()).await?;
        Ok(DropCollection(collection))
    }
}

pub async fn create_collection_from_files<'a>(
    iroh: &IrohInstance,
    paths: &'a [PathBuf],
) -> IrohResult<Vec<(&'a PathBuf, AddOutcome)>> {
    try_join_all(paths.iter().map(|path| async move {
        let add_progress = iroh
            .get_node()
            .blobs
            .add_from_path(path.clone(), true, SetTagOption::Auto, WrapOption::NoWrap)
            .await;
        match add_progress {
            Ok(add_progress) => {
                let progress = add_progress.finish().await;
                if let Ok(progress) = progress {
                    Ok((path, progress))
                } else {
                    Err(progress.err().unwrap().into())
                }
            }
            Err(e) => Err(e.into()),
        }
    }))
    .await
}

uniffi::custom_type!(BlobTicket, String);

impl UniffiCustomTypeConverter for BlobTicket {
    type Builtin = String;

    fn into_custom(val: Self::Builtin) -> uniffi::Result<Self> {
        Ok(BlobTicket::from_str(&val).map_err(|e| anyhow::anyhow!(e))?)
    }

    fn from_custom(obj: Self) -> Self::Builtin {
        obj.to_string()
    }
}
