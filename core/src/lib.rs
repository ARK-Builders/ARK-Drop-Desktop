pub mod error;
pub mod metadata;
pub mod send;

use std::{
    path::PathBuf,
    str::FromStr,
    sync::{mpsc::Sender, Arc},
};

use anyhow::{Context, Error};
use data_encoding::HEXLOWER;
use futures_buffered::{join_all, try_join_all};
use iroh::{discovery::pkarr::PkarrPublisher, protocol::Router, Endpoint, SecretKey};
use iroh_blobs::{
    net_protocol::Blobs, store::ImportMode, ticket::BlobTicket, util::SetTagOption, BlobFormat,
    Hash, Tag,
};
use iroh_blobs::{store::Store, util::local_pool::LocalPool};

use rand::Rng;
use send::{SendEvent, SendStatus};

fn get_or_create_secret() -> anyhow::Result<SecretKey> {
    match std::env::var("IROH_SECRET") {
        Ok(secret) => SecretKey::from_str(&secret).context("invalid secret"),
        Err(_) => {
            let key = SecretKey::generate(rand::rngs::OsRng);
            Ok(key)
        }
    }
}

pub struct IrohInstance {}

impl IrohInstance {
    pub async fn send_files(
        files: Vec<String>,
        sender: Arc<Sender<SendEvent>>,
    ) -> Result<BlobTicket, Error> {
        let secret_key = get_or_create_secret()?;
        let mut builder = Endpoint::builder()
            .alpns(vec![iroh_blobs::protocol::ALPN.to_vec()])
            .secret_key(secret_key)
            .relay_mode(iroh::RelayMode::Default);
        builder =
            builder.add_discovery(|secret_key| Some(PkarrPublisher::n0_dns(secret_key.clone())));

        let ps = SendStatus::new(sender.clone());

        let rt = LocalPool::default();
        let endpoint = builder.bind().await?;
        let blobs = Blobs::memory()
            .local_pool(rt.handle().clone())
            .events(ps.into())
            .build(&endpoint);

        let router = Router::builder(endpoint)
            .accept(iroh_blobs::ALPN, blobs.clone())
            .spawn()
            .await
            .unwrap();

        let _ = router.endpoint().home_relay().initialized().await?;

        let paths: Vec<PathBuf> = files
            .into_iter()
            .map(|path| Ok(PathBuf::from_str(&path)?))
            .filter_map(|path: Result<PathBuf, Error>| path.ok())
            .collect();

        let (hash, _tag) = IrohInstance::import_collection(blobs, paths)
            .await
            .expect("Failed to Import collection");

        Ok(BlobTicket::new(
            router.endpoint().node_id().into(),
            hash,
            iroh_blobs::BlobFormat::HashSeq,
        )
        .expect("Failed to create ticket"))
    }

    pub async fn import_collection(
        blobs: Blobs<iroh_blobs::store::mem::Store>,
        paths: Vec<PathBuf>,
    ) -> Result<(Hash, Tag), Error> {
        let db = blobs.store();

        let (send, recv) = async_channel::bounded(32);
        let progress = iroh_blobs::util::progress::AsyncChannelProgressSender::new(send);

        println!("Importing collection: {:?}", paths);

        tokio::spawn(async move {
            while let Ok(message) = recv.recv().await {
                println!("Received progress message: {:?}", message);
            }
            println!("Receiver closed: all progress messages processed.");
        });

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

        println!("Finished Importing Collection");

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

        Ok(blobs
            .client()
            .create_collection(collection, SetTagOption::Auto, Default::default())
            .await
            .expect("Failed to create collection."))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{send::SendEvent, IrohInstance};
    use tokio;

    #[tokio::test]
    async fn test_send_files() {
        tracing_subscriber::fmt::init();
        let cwd = std::env::current_dir().unwrap();

        let files = vec![cwd.join("Cargo.toml").to_string_lossy().to_string()];

        let (tx, rx) = std::sync::mpsc::channel::<SendEvent>();

        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                println!("Received event: {:?}", event);
            }
            println!("Receiver closed: all events processed.");
        });

        println!("Sending files: {:?}", files);

        // Call send_files and await the ticket
        let ticket = IrohInstance::send_files(files, Arc::new(tx)).await.unwrap();

        println!("Ticket: {:?}", ticket);
    }
}
