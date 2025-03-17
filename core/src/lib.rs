pub mod error;
pub mod metadata;
pub mod send;

use std::collections::BTreeMap;
use std::{path::PathBuf, str::FromStr, sync::Arc};

use anyhow::{Context, Error, Result};
use futures_buffered::join_all;
use iroh::{protocol::Router, Endpoint, SecretKey};
use iroh_blobs::format::collection::Collection;
use iroh_blobs::get::db::DownloadProgress;
use iroh_blobs::get::request::get_hash_seq_and_sizes;
use iroh_blobs::store::{ImportProgress, Store};
use iroh_blobs::HashAndFormat;
use iroh_blobs::{
    net_protocol::Blobs, store::ImportMode, ticket::BlobTicket, util::SetTagOption, BlobFormat,
    Hash, Tag,
};
use metadata::FileTransfer;
use send::{SendEvent, SendStatus};
use tokio::sync::mpsc::Sender;

fn get_or_create_secret() -> anyhow::Result<SecretKey> {
    match std::env::var("IROH_SECRET") {
        Ok(secret) => SecretKey::from_str(&secret).context("invalid secret"),
        Err(_) => {
            let key = SecretKey::generate(rand::rngs::OsRng);
            Ok(key)
        }
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

        let ps = SendStatus::new(sender.clone());

        let blobs = Blobs::memory().events(ps.into()).build(&endpoint);

        let router = Router::builder(endpoint)
            .accept(iroh_blobs::ALPN, blobs.clone())
            .spawn()
            .await
            .unwrap();

        let _ = router.endpoint().home_relay().initialized().await?;

        Ok(Self { router, blobs })
    }

    pub async fn send_files(&self, files: Vec<String>) -> Result<BlobTicket, Error> {
        let paths: Vec<PathBuf> = files
            .into_iter()
            .map(|path| Ok(PathBuf::from_str(&path)?))
            .filter_map(|path: Result<PathBuf, Error>| path.ok())
            .collect();

        let (hash, _tag) = self
            .import_collection(paths)
            .await
            .expect("Failed to Import collection");

        Ok(BlobTicket::new(
            self.router.endpoint().node_id().into(),
            hash,
            iroh_blobs::BlobFormat::HashSeq,
        )
        .expect("Failed to create ticket"))
    }

    pub async fn receive_files(
        ticket: String,
        tx: Sender<Vec<FileTransfer>>,
    ) -> Result<Collection, Error> {
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

        let connection = endpoint.connect(addr, iroh_blobs::protocol::ALPN).await?;

        let dir_name = format!(".drop-get-{}", ticket.hash().to_hex());
        let iroh_data_dir = std::env::current_dir()?.join(dir_name);
        let db = iroh_blobs::store::fs::Store::load(&iroh_data_dir).await?;
        let hash_and_format = HashAndFormat {
            hash: ticket.hash(),
            format: ticket.format(),
        };
        let (_hash_seq, sizes) =
            get_hash_seq_and_sizes(&connection, &hash_and_format.hash, 1024 * 1024 * 32).await?;
        let total_size = sizes.iter().sum::<u64>();
        let total_files = sizes.len().saturating_sub(1);
        let payload_size = sizes.iter().skip(1).sum::<u64>();
        let (send, recv) = async_channel::bounded(32);
        let progress = iroh_blobs::util::progress::AsyncChannelProgressSender::new(send);
        let _task = tokio::spawn(IrohInstance::show_download_progress(recv, total_size));
        let get_conn = || async move { Ok(connection) };
        let stats =
            iroh_blobs::get::db::get_to_db(&db, get_conn, &hash_and_format, progress).await?;
        let collection = Collection::load_db(&db, &hash_and_format.hash).await?;

        Ok(collection)
    }

    pub async fn import_collection(&self, paths: Vec<PathBuf>) -> Result<(Hash, Tag), Error> {
        let db = self.blobs.store();

        let (send, recv) = async_channel::bounded(32);
        let progress = iroh_blobs::util::progress::AsyncChannelProgressSender::new(send);
        let show_progress = tokio::spawn(IrohInstance::show_ingest_progress(recv));

        let outcomes = join_all(paths.into_iter().map(|path| {
            let progress = progress.clone();
            async move {
                (
                    path.clone(),
                    db.import_file(
                        path.clone(),
                        ImportMode::TryReference,
                        BlobFormat::Raw,
                        progress, // Use the cloned progress
                    )
                    .await
                    .expect("Failed to import file."),
                )
            }
        }))
        .await;

        let collection = outcomes
            .into_iter()
            .map(|(path, outcome)| {
                let name = path
                    .file_name()
                    .expect("The file name is not valid.")
                    .to_string_lossy()
                    .to_string();

                let hash = outcome.0.hash().clone();
                (name, hash)
            })
            .collect();

        Ok(self
            .blobs
            .client()
            .create_collection(collection, SetTagOption::Auto, Default::default())
            .await
            .expect("Failed to create collection."))
    }

    pub async fn show_download_progress(
        recv: async_channel::Receiver<DownloadProgress>,
        total_size: u64,
    ) -> anyhow::Result<()> {
        let mut total_done = 0;
        let mut sizes = BTreeMap::new();
        loop {
            let x = recv.recv().await;
            match x {
                Ok(DownloadProgress::Connected) => {
                    println!("[RECV]: Connected");
                }
                Ok(DownloadProgress::FoundHashSeq { children, .. }) => {
                    println!("[RECV]: FoundHashSeq");
                }
                Ok(DownloadProgress::Found { id, size, .. }) => {
                    sizes.insert(id, size);

                    println!("[RECV]: Found");
                }
                Ok(DownloadProgress::Progress { offset, .. }) => {
                    println!("[RECV]: Progress");
                }
                Ok(DownloadProgress::Done { id }) => {
                    total_done += sizes.remove(&id).unwrap_or_default();
                }
                Ok(DownloadProgress::AllDone(stats)) => {
                    println!("[RECV]: All Done");
                    break;
                }
                Ok(DownloadProgress::Abort(e)) => {
                    anyhow::bail!("download aborted: {e:?}");
                }
                Err(e) => {
                    anyhow::bail!("error reading progress: {e:?}");
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn show_ingest_progress(
        recv: async_channel::Receiver<ImportProgress>,
    ) -> anyhow::Result<()> {
        loop {
            let event = recv.recv().await;
            match event {
                Ok(_) => {}
                Err(_e) => {
                    break;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{metadata::FileTransfer, send::SendEvent, IrohInstance};
    use tokio;

    #[tokio::test]
    async fn test_receive_files() {
        tracing_subscriber::fmt::init();

        let files = vec!["Cargo.toml".to_string()];

        let (tx, mut rx) = tokio::sync::mpsc::channel::<SendEvent>(32);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                println!("[SENDER]: Received event, {:?}", event);
            }
            println!("[SENDER]: Receiver closed, all events processed.");
        });

        let sender = IrohInstance::sender(Arc::new(tx)).await.unwrap();

        let ticket = sender.send_files(files).await.unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<FileTransfer>>(32);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                println!("[RECV]: Received event, {:?}", event);
            }
            println!("[RECV]: Receiver closed, all events processed.");
        });

        let collection = IrohInstance::receive_files(ticket.to_string(), tx)
            .await
            .unwrap();

        assert!(true);
    }
}
