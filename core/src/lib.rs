pub mod erorr;
pub mod metadata;

use anyhow::Result;

use erorr::{IrohError, IrohResult};
use futures_buffered::try_join_all;
use futures_lite::stream::StreamExt;
use iroh::{
    client::blobs::{AddOutcome, WrapOption},
    node::Node,
};
use iroh_base::ticket::BlobTicket;
use iroh_blobs::{
    format::collection::Collection,
    get::db::{BlobId, DownloadProgress},
    store,
    util::SetTagOption,
    BlobFormat,
};
use metadata::CollectionMetadata;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File, iter::Iterator, num::NonZeroU64, vec};
use std::{path::PathBuf, str::FromStr};

type IrohNode = Node<iroh_blobs::store::mem::Store>;
pub struct IrohInstance {
    node: IrohNode,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileTransfer {
    pub name: String,
    pub transfered: u64,
    pub total: u64,
    pub child: u64,
    pub id: u64,
}

impl IrohInstance {
    pub async fn new() -> Result<Self> {
        let node = iroh::node::Node::memory().spawn().await?;
        Ok(Self { node })
    }

    pub fn get_node(&self) -> &IrohNode {
        &self.node
    }

    pub async fn create_collection_from_files<'a>(
        &self,
        paths: &'a Vec<PathBuf>,
    ) -> IrohResult<Vec<(&'a PathBuf, AddOutcome)>> {
        try_join_all(paths.into_iter().map(|path| async move {
            let add_progress = self
                .node
                .blobs
                .add_from_path(path.clone(), true, SetTagOption::Auto, WrapOption::NoWrap)
                .await;
            match add_progress {
                Ok(add_progress) => {
                    let progress = add_progress.finish().await;
                    if let Ok(progress) = progress {
                        Ok((path, progress))
                    } else {
                        return Err(progress.err().unwrap().into());
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }))
        .await
    }

    pub async fn send_files(&self, files: &Vec<PathBuf>) -> IrohResult<BlobTicket> {
        let outcome = self.create_collection_from_files(files).await?;

        let collection = outcome
            .into_iter()
            .map(|(path, outcome)| {
                let name = path
                    .file_name()
                    .expect("The file name is not valid.")
                    .to_string_lossy()
                    .to_string();
                let hash = outcome.hash;
                return (name, hash);
            })
            .collect();

        let (hash, _) = self
            .node
            .blobs
            .create_collection(collection, SetTagOption::Auto, Default::default())
            .await?;

        self.node
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
    ) -> IrohResult<Collection> {
        let ticket = BlobTicket::from_str(&ticket)?;

        if ticket.format() != BlobFormat::HashSeq {
            return Err(IrohError::UnsupportedFormat(ticket.format()));
        }

        let mut download_stream = self
            .node
            .blobs
            .download_hash_seq(ticket.hash(), ticket.node_addr().clone())
            .await?;

        let mut metaDataHash = None;
        let mut metadata: Option<CollectionMetadata> = None;

        // we should use BTreeMap to keep the order of the files
        let mut files: Vec<FileTransfer> = Vec::new();

        while let Some(chunk) = download_stream.next().await {
            let chunk = chunk?;
            match chunk {
                DownloadProgress::Found {
                    id,
                    child,
                    hash,
                    size,
                } => match u64::from(child) {
                    1 => {
                        // blob indicating all the hashes in the collection
                    }
                    2 => {
                        // blob indicating the metadata of the collection
                        metaDataHash = Some((id, hash));
                    }
                    n => {
                        // blob indicating the file
                        files.push(FileTransfer {
                            name: "Unknown File".to_string().into(),
                            transfered: 0,
                            total: size,
                            child: n - 2,
                            id,
                        });
                    }
                },
                DownloadProgress::Progress { id, offset } => {
                    if let Some(file) = files.iter_mut().find(|file| file.id == id) {
                        file.transfered = offset;
                        // if we have metadata, we can update the file name
                        if let Some(metadata) = &metadata {
                            if let Some(name) = metadata.file_names.get(file.child as usize) {
                                file.name = name.clone();
                            }
                        }
                    }
                }
                DownloadProgress::Done { id } => {
                    if let Some((metadata_id, metadata_hash)) = metaDataHash {
                        if id == metadata_id {
                            metadata = Some(CollectionMetadata::from_bytes(
                                self.node.blobs.read_to_bytes(metadata_hash).await?,
                            )?);
                        }
                    } else {
                        if let Some(file) = files.iter_mut().find(|file| file.id == id) {
                            file.transfered = file.total;
                        }
                    }
                }
                _ => {}
            }

            handle_chunk(files.clone());
        }

        let collection = self.node.blobs.get_collection(ticket.hash()).await?;

        files = vec![];

        for (name, hash) in collection.iter() {
            let content = self.node.blobs.read_to_bytes(*hash).await?;
            files.push({
                FileTransfer {
                    name: name.clone(),
                    transfered: content.len() as u64,
                    total: content.len() as u64,
                    child: 0,
                    id: 0,
                }
            })
        }

        handle_chunk(files.clone());

        Ok(collection)
    }

    pub async fn export_collection(
        &self,
        collection: Collection,
        outpath: PathBuf,
    ) -> IrohResult<()> {
        for (name, hash) in collection.iter() {
            let content = self.node.blobs.read_to_bytes(*hash).await?;
            let file_path = outpath.join(name);
            let _ = std::fs::write(&file_path, content);
        }

        Ok(())
    }
}
