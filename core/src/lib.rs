pub mod error;
pub mod metadata;
pub mod send;

use std::collections::BTreeMap;
use std::{path::PathBuf, str::FromStr, sync::Arc};

use anyhow::{Context, Error, Result};
use error::IrohError;
use futures_buffered::join_all;
use futures_lite::StreamExt;
use iroh::{protocol::Router, Endpoint, SecretKey};
use iroh_blobs::format::collection::Collection;
use iroh_blobs::get::db::DownloadProgress;

use iroh_blobs::hashseq::HashSeq;
use iroh_blobs::store::{ImportProgress, Store};
use iroh_blobs::{
    net_protocol::Blobs, store::ImportMode, ticket::BlobTicket, util::SetTagOption, BlobFormat,
    Hash, Tag,
};
use metadata::{CollectionMetadata, FileTransfer};
use send::{SendEvent, SendStatus};
use tokio::sync::mpsc::Sender;

fn get_or_create_secret() -> anyhow::Result<SecretKey> {
    match std::env::var("IROH_SECRET") {
        Ok(secret) => SecretKey::from_str(&secret).context("invalid secret"),
        Err(_) => Ok(SecretKey::generate(rand::rngs::OsRng)),
    }
}

pub struct IrohInstance {
    router: Router,
    blobs: Blobs<iroh_blobs::store::mem::Store>,
}

impl IrohInstance {
    pub async fn sender(sender: Arc<Sender<SendEvent>>) -> Result<Self> {
        let secret_key = get_or_create_secret()?;

        let endpoint = Endpoint::builder()
            .alpns(vec![iroh_blobs::protocol::ALPN.to_vec()])
            .discovery_n0()
            .discovery_local_network()
            .secret_key(secret_key)
            .relay_mode(iroh::RelayMode::Default)
            .bind()
            .await?;

        let ps = SendStatus::new(sender);
        let blobs = Blobs::memory().events(ps.into()).build(&endpoint);

        let router = Router::builder(endpoint.clone())
            .accept(iroh_blobs::protocol::ALPN, blobs.clone())
            .spawn()
            .await?;

        router
            .endpoint()
            .home_relay()
            .initialized()
            .await
            .context("Failed to initialize home relay")?;

        Ok(Self { router, blobs })
    }

    pub async fn send_files(&self, files: Vec<String>) -> Result<BlobTicket> {
        let paths: Vec<PathBuf> = files
            .into_iter()
            .filter_map(|path| PathBuf::from_str(&path).ok())
            .collect();

        let (hash, _tag) = self.import_collection(paths).await?;

        Ok(BlobTicket::new(
            self.router.endpoint().node_id().into(),
            hash,
            BlobFormat::HashSeq,
        )?)
    }

    pub async fn receive_files(
        ticket: String,
        tx: Sender<Vec<FileTransfer>>,
    ) -> Result<(Blobs<iroh_blobs::store::mem::Store>, Collection)> {
        let ticket = BlobTicket::from_str(&ticket)?;
        let addr = ticket.node_addr().clone();
        let secret_key = get_or_create_secret()?;

        let endpoint = Endpoint::builder()
            .alpns(vec![iroh_blobs::protocol::ALPN.to_vec()])
            .discovery_n0()
            .discovery_local_network()
            .secret_key(secret_key)
            .relay_mode(iroh::RelayMode::Default)
            .bind()
            .await?;

        let blobs = Blobs::memory().build(&endpoint);

        let mut download_stream = blobs
            .client()
            .download_hash_seq(ticket.hash(), ticket.node_addr().clone())
            .await
            .map_err(|e| IrohError::DownloadError(e.to_string()))?;

        let mut curr_metadata: Option<CollectionMetadata> = None;
        let mut curr_hashseq: Option<HashSeq> = None;
        let mut files: Vec<FileTransfer> = Vec::new();
        let mut map: BTreeMap<u64, String> = BTreeMap::new();

        while let Some(event) = download_stream.next().await {
            let event = event.map_err(|e| IrohError::DownloadError(e.to_string()))?;

            match event {
                DownloadProgress::FoundHashSeq { hash, .. } => {
                    let hashseq = blobs
                        .client()
                        .read_to_bytes(hash)
                        .await
                        .map_err(|e| IrohError::DownloadError(e.to_string()))?;
                    let hashseq = HashSeq::try_from(hashseq)
                        .map_err(|e| IrohError::InvalidMetadata(e.to_string()))?;
                    let metadata_hash = hashseq.iter().next().ok_or_else(|| {
                        IrohError::InvalidMetadata("hashseq is empty".to_string())
                    })?;
                    let metadata_bytes = blobs
                        .client()
                        .read_to_bytes(metadata_hash)
                        .await
                        .map_err(|e| IrohError::DownloadError(e.to_string()))?;
                    let metadata: CollectionMetadata = postcard::from_bytes(&metadata_bytes)
                        .map_err(|e| IrohError::InvalidMetadata(e.to_string()))?;

                    if metadata.names.len() + 1 != hashseq.len() {
                        return Err(IrohError::InvalidMetadata(
                            "metadata does not match hashseq".to_string(),
                        )
                        .into());
                    }
                    curr_hashseq = Some(hashseq);
                    curr_metadata = Some(metadata);
                }

                DownloadProgress::AllDone(_) => {
                    let collection = blobs
                        .client()
                        .get_collection(ticket.hash())
                        .await
                        .map_err(|e| IrohError::DownloadError(e.to_string()))?;
                    files.clear();
                    for (name, hash) in collection.iter() {
                        let content = blobs
                            .client()
                            .read_to_bytes(*hash)
                            .await
                            .map_err(|e| IrohError::DownloadError(e.to_string()))?;
                        files.push(FileTransfer {
                            name: name.clone(),
                            transferred: content.len() as u64,
                            total: content.len() as u64,
                        });
                    }
                    tx.send(files.clone())
                        .await
                        .map_err(|_| IrohError::SendError)?;

                    break;
                    // return Ok((blobs, collection.into()));
                }

                DownloadProgress::Done { id } => {
                    if let Some(name) = map.get(&id) {
                        if let Some(file) = files.iter_mut().find(|file| file.name == *name) {
                            file.transferred = file.total;
                        }
                    }
                    tx.send(files.clone())
                        .await
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
                                    tx.send(files.clone())
                                        .await
                                        .map_err(|_| IrohError::SendError)?;
                                    map.insert(id, name.clone());
                                }
                            }
                        }
                    }
                }

                DownloadProgress::Progress { id, offset } => {
                    if let Some(name) = map.get(&id) {
                        if let Some(file) = files.iter_mut().find(|file| file.name == *name) {
                            file.transferred = offset;
                        }
                    }
                    tx.send(files.clone())
                        .await
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
                                        tx.send(files.clone())
                                            .await
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

        let collection = blobs
            .client()
            .get_collection(ticket.hash())
            .await
            .map_err(|e| IrohError::DownloadError(e.to_string()))?;

        Ok((blobs, collection.into()))
    }

    pub async fn import_collection(&self, paths: Vec<PathBuf>) -> Result<(Hash, Tag)> {
        let db = self.blobs.store();
        let (send, recv) = async_channel::bounded(32);
        let progress = iroh_blobs::util::progress::AsyncChannelProgressSender::new(send);
        let _progress_task = tokio::spawn(IrohInstance::show_ingest_progress(recv));

        let outcomes = join_all(paths.into_iter().map(|path| {
            let progress = progress.clone();
            async move {
                let outcome = db
                    .import_file(
                        path.clone(),
                        ImportMode::TryReference,
                        BlobFormat::Raw,
                        progress,
                    )
                    .await?;
                Ok((path, outcome))
            }
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

        let collection = Collection::from_iter(
            outcomes
                .into_iter()
                .map(|(path, outcome)| {
                    let name = path
                        .file_name()
                        .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?
                        .to_string_lossy()
                        .to_string();
                    let hash = outcome.0.hash().clone();
                    Ok((name, hash))
                })
                .collect::<Result<Vec<_>>>()?,
        );

        Ok(self
            .blobs
            .client()
            .create_collection(collection, SetTagOption::Auto, Default::default())
            .await?)
    }

    pub async fn show_ingest_progress(
        recv: async_channel::Receiver<ImportProgress>,
    ) -> anyhow::Result<()> {
        while let Ok(_event) = recv.recv().await {
            // Add logging here if desired
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_receive_files() -> Result<()> {
        tracing_subscriber::fmt::init();
        let files = vec!["Cargo.toml".to_string()];
        let (tx, mut rx) = tokio::sync::mpsc::channel::<SendEvent>(32);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                println!("[SENDER]: Received event, {:?}", event);
            }
            println!("[SENDER]: Receiver closed, all events processed.");
        });

        let sender = IrohInstance::sender(Arc::new(tx)).await?;
        let ticket = sender.send_files(files).await?;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<FileTransfer>>(32);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                println!("[RECEIVER]: Received event, {:?}", event);
            }
            println!("[RECEIVER]: Receiver closed, all events processed.");
        });

        let _collection = IrohInstance::receive_files(ticket.to_string(), tx).await?;
        Ok(())
    }
}
