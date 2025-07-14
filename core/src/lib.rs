use std::{
    path::{Component, Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use anyhow::Context;
use data_encoding::HEXLOWER;
use futures_buffered::BufferedStreamExt;
use futures_lite::stream::StreamExt;
use iroh::{protocol::Router, Endpoint, RelayMode, SecretKey, Watcher};
use iroh_blobs::{
    api::{
        blobs::{
            AddPathOptions, AddProgressItem, ExportMode, ExportOptions, ExportProgressItem,
            ImportMode,
        },
        remote::GetProgressItem,
        Store, TempTag,
    },
    format::collection::Collection,
    provider,
    store::fs::FsStore,
    ticket::BlobTicket,
    BlobFormat,
};
use iroh_blobs::{get::request::get_hash_seq_and_sizes, net_protocol::Blobs};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::iter::Iterator;
use tokio::{
    select,
    sync::{mpsc, oneshot, Mutex},
};
use tracing::{error, trace};
use walkdir::WalkDir;

// --- Public API ---
// Progress updates for the `send_file` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SendProgress {
    // The iroh node is being set up.
    SettingUp,
    // Hashing files and adding them to the database.
    Importing {
        // The total number of bytes to import.
        total_bytes: u64,
        // The number of bytes imported so far.
        imported_bytes: u64,
        // The name of the file currently being imported.
        current_file: String,
    },
    // Import is complete.
    Imported {
        // The hash of the created collection.
        hash: String,
        // The total size of the imported data.
        total_bytes: u64,
    },
    // The ticket is ready and the node is listening for connections.
    Ready {
        ticket: String,
    },
    // A peer has connected.
    PeerConnected(String),
    // A peer has disconnected.
    PeerDisconnected,
    // The send operation has completed successfully.
    Done,
}

// Progress updates for the `receive_file` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReceiveProgress {
    // Setting up the iroh node and parsing the ticket.
    SettingUp,
    // Connecting to the sender.
    Connecting,
    // Connected, figuring out what to download.
    Connected,
    // Downloading the content.
    Downloading {
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    // Download complete, writing files to disk.
    Exporting {
        // Number of files written so far.
        exported_files: u64,
        // Total number of files in the collection.
        total_files: u64,
        // The name of the file currently being written.
        current_file: String,
    },
    // All files received and saved.
    Done {
        // The path to the directory where files were saved.
        path: String,
    },
}

// A handle to gracefully shut down the sending background task.
// When this handle is dropped, the background task is notified to stop.
pub struct ShutdownHandle(Option<oneshot::Sender<()>>);

impl ShutdownHandle {
    pub fn new(sender: oneshot::Sender<()>) -> Self {
        ShutdownHandle(Some(sender))
    }
}

impl Drop for ShutdownHandle {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

// Sends a single file or directory.
//
// This function will spawn a background task that manages the iroh node and serves the data.
//
// # Arguments
// * `path`: The path to the file or directory to send.
// * `updates`: An `mpsc::Sender` to send `SendProgress` updates to.
// * `router_storage`: A mutex to store the router, ensuring it persists.
//
// # Returns
// A `Result` containing:
// * A tuple of (`BlobTicket` as a string, `ShutdownHandle`). The ticket should be shared
//   with the receiver. The handle can be used to stop the sending process.
pub async fn send_file(
    path: PathBuf,
    updates: mpsc::Sender<SendProgress>,
    router_storage: Arc<Mutex<Option<Router>>>,
) -> anyhow::Result<(String, ShutdownHandle)> {
    let (ticket_tx, ticket_rx) = oneshot::channel();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    let updates_task = updates.clone();

    tokio::spawn(async move {
        // Create a temporary directory for the iroh database.
        let temp_dir = std::env::temp_dir().join(format!(
            "sendme-send-{}",
            HEXLOWER.encode(&rand::thread_rng().gen::<[u8; 8]>())
        ));

        let res = async {
            updates.send(SendProgress::SettingUp).await?;

            tokio::fs::create_dir_all(&temp_dir).await?;
            let db = FsStore::load(&temp_dir).await?;

            let secret_key = SecretKey::generate(rand::rngs::OsRng);
            let endpoint = Endpoint::builder()
                .alpns(vec![iroh_blobs::protocol::ALPN.to_vec()])
                .secret_key(secret_key)
                .relay_mode(RelayMode::Default)
                .bind()
                .await?;

            let (provider_tx, mut provider_rx) = mpsc::channel(32);
            let blobs = Blobs::new(&db.clone(), endpoint.clone(), Some(provider_tx));

            let router = iroh::protocol::Router::builder(endpoint)
                .accept(iroh_blobs::ALPN, blobs.clone())
                .spawn();

            let mut router_guard = router_storage.lock().await;
            *router_guard = Some(router.clone());
            drop(router_guard);

            let _ = router.endpoint().home_relay().initialized().await?;

            let (tag, size, _collection) = import(path.clone(), blobs.store(), &updates).await?;
            let hash = *tag.hash();

            updates
                .send(SendProgress::Imported {
                    hash: hash.to_hex().to_string(),
                    total_bytes: size,
                })
                .await?;

            let addr = router.endpoint().node_addr().initialized().await?;
            let ticket = BlobTicket::new(addr, hash, BlobFormat::HashSeq);

            // Send the ticket back to the caller.
            if ticket_tx.send(ticket.to_string()).is_err() {
                anyhow::bail!("Caller dropped ticket receiver before ticket was ready");
            }

            updates
                .send(SendProgress::Ready {
                    ticket: ticket.to_string(),
                })
                .await?;

            // Main loop: wait for shutdown signal or handle provider events.
            loop {
                select! {
                    _ = &mut shutdown_rx => {
                        break;
                    },
                    Some(event) = provider_rx.recv() => {
                        handle_provider_event(event, &updates).await;
                    }
                }
            }

            // Cleanup
            drop(tag);
            router.shutdown().await?;
            Ok(())
        }
        .await;

        if let Err(e) = res {
            error!("Send task failed: {:?}", e);
        }

        let _ = tokio::fs::remove_dir_all(temp_dir).await;
        updates_task.send(SendProgress::Done).await.ok();
    });

    // Wait for the ticket to be created by the background task.
    let ticket = ticket_rx
        .await
        .context("Failed to get ticket from send task")?;
    let handle = ShutdownHandle(Some(shutdown_tx));

    Ok((ticket, handle))
}

// Receives a file or directory using a blob ticket.
//
// # Arguments
// * `ticket_str`: The blob ticket as a string, obtained from the sender.
// * `updates`: An `mpsc::Sender` to send `ReceiveProgress` updates to.
//
// # Returns
// A `Result` containing the path to the directory where the files were saved.
pub async fn receive_file(
    ticket_str: String,
    updates: mpsc::Sender<ReceiveProgress>,
) -> anyhow::Result<String> {
    updates.send(ReceiveProgress::SettingUp).await?;
    let ticket = BlobTicket::from_str(&ticket_str)?;

    // Create a temporary directory for the iroh database.
    let temp_db_dir =
        std::env::temp_dir().join(format!("sendme-recv-db-{}", ticket.hash().to_hex()));

    // Create a directory to store the final output files.
    let mut out_dir =
        std::env::temp_dir().join(format!("sendme-recv-out-{}", ticket.hash().to_hex()));

    // If the directory already exists, create a new one with a timestamp suffix
    let mut counter = 1;
    while out_dir.exists() {
        out_dir = std::env::temp_dir().join(format!(
            "sendme-recv-out-{}-{}",
            ticket.hash().to_hex(),
            counter
        ));
        counter += 1;
    }

    let res = async {
        println!("Creating output directory: {}", out_dir.display());

        if let Err(e) = tokio::fs::create_dir_all(&out_dir).await {
            anyhow::bail!(
                "Failed to create output directory {}: {}",
                out_dir.display(),
                e
            );
        }
        if !temp_db_dir.exists() {
            if let Err(e) = tokio::fs::create_dir_all(&temp_db_dir).await {
                anyhow::bail!(
                    "Failed to create temp database directory {}: {}",
                    temp_db_dir.display(),
                    e
                );
            }
        }

        let db = FsStore::load(&temp_db_dir).await?;

        let secret_key = SecretKey::generate(rand::rngs::OsRng);
        let endpoint = Endpoint::builder()
            .alpns(vec![])
            .secret_key(secret_key)
            .relay_mode(RelayMode::Default)
            .bind()
            .await?;

        let hash_and_format = ticket.hash_and_format();

        updates.send(ReceiveProgress::Connecting).await?;
        let connection = endpoint
            .connect(ticket.node_addr().clone(), iroh_blobs::protocol::ALPN)
            .await?;

        updates.send(ReceiveProgress::Connected).await?;
        let (_hash_seq, sizes) =
            get_hash_seq_and_sizes(&connection, &hash_and_format.hash, 1024 * 1024 * 32, None)
                .await?;

        let total_size = sizes.iter().copied().sum::<u64>();
        let local = db.remote().local(hash_and_format).await?;

        if !local.is_complete() {
            let get = db.remote().execute_get(connection, local.missing());
            let mut stream = get.stream();
            while let Some(item) = stream.next().await {
                match item {
                    GetProgressItem::Progress(offset) => {
                        updates
                            .send(ReceiveProgress::Downloading {
                                downloaded_bytes: local.local_bytes() + offset,
                                total_bytes: total_size,
                            })
                            .await?;
                    }
                    GetProgressItem::Done(_) => break,
                    GetProgressItem::Error(cause) => anyhow::bail!(cause),
                }
            }
        }

        let collection = Collection::load(hash_and_format.hash, db.as_ref()).await?;
        export(&db, collection, &out_dir, &updates).await?;

        updates
            .send(ReceiveProgress::Done {
                path: out_dir.to_string_lossy().to_string(),
            })
            .await?;

        Ok(out_dir.to_string_lossy().to_string())
    }
    .await;

    // Cleanup the temporary database directory regardless of the result.
    tokio::fs::remove_dir_all(&temp_db_dir).await?;

    res
}

// --- Internal Implementation ---

// Converts an already canonicalized path to a string for use in iroh collections.
fn canonicalized_path_to_string(path: impl AsRef<Path>) -> anyhow::Result<String> {
    let mut path_str = String::new();
    let parts = path
        .as_ref()
        .components()
        .filter_map(|c| match c {
            Component::Normal(x) => Some(
                x.to_str()
                    .ok_or_else(|| anyhow::anyhow!("Non-UTF8 path component")),
            ),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?;
    path_str.push_str(&parts.join("/"));
    Ok(path_str)
}

// Imports a file or directory into the iroh database.
async fn import(
    path: PathBuf,
    db: &Store,
    updates: &mpsc::Sender<SendProgress>,
) -> anyhow::Result<(TempTag, u64, Collection)> {
    let parallelism = num_cpus::get();
    let path = path.canonicalize()?;
    anyhow::ensure!(path.exists(), "path {} does not exist", path.display());
    let root = path.parent().context("Failed to get parent directory")?;

    let data_sources: Vec<(String, PathBuf)> = WalkDir::new(&path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            let file_path = entry.into_path();
            let relative = file_path.strip_prefix(root)?;
            let name = canonicalized_path_to_string(relative)?;
            Ok((name, file_path))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let total_bytes = data_sources
        .iter()
        .map(|(_, p)| p.metadata().map(|m| m.len()).unwrap_or(0))
        .sum();
    let mut imported_bytes = 0;

    let mut names_and_tags = n0_future::stream::iter(data_sources)
        .map(|(name, path)| {
            let db = db.clone();
            let updates = updates.clone();
            async move {
                let mut stream = db
                    .add_path_with_opts(AddPathOptions {
                        path,
                        mode: ImportMode::TryReference,
                        format: BlobFormat::Raw,
                    })
                    .stream()
                    .await;

                let mut item_size = 0;
                let temp_tag = loop {
                    updates
                        .send(SendProgress::Importing {
                            total_bytes,
                            imported_bytes,
                            current_file: name.clone(),
                        })
                        .await
                        .ok();
                    match stream
                        .next()
                        .await
                        .context("import stream ended unexpectedly")?
                    {
                        AddProgressItem::Size(size) => item_size = size,
                        AddProgressItem::CopyProgress(offset) => {
                            updates
                                .send(SendProgress::Importing {
                                    total_bytes,
                                    imported_bytes: imported_bytes + offset,
                                    current_file: name.clone(),
                                })
                                .await
                                .ok();
                        }
                        AddProgressItem::Done(tt) => break tt,
                        AddProgressItem::Error(e) => {
                            anyhow::bail!("Error importing {}: {}", name, e)
                        }
                        _ => {}
                    }
                };
                anyhow::Ok((name, temp_tag, item_size))
            }
        })
        .buffered_unordered(parallelism)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(|res| {
            if let Ok((_name, _tag, size)) = &res {
                imported_bytes += size;
            }
            res
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    names_and_tags.sort_by(|(a, _, _), (b, _, _)| a.cmp(b));
    let size = names_and_tags.iter().map(|(_, _, size)| *size).sum::<u64>();

    let (collection, tags) = names_and_tags
        .into_iter()
        .map(|(name, tag, _)| ((name, *tag.hash()), tag))
        .unzip::<_, _, Collection, Vec<_>>();

    let temp_tag = collection.clone().store(db).await?;
    drop(tags);

    Ok((temp_tag, size, collection))
}

// Exports a collection from the database to the filesystem.
async fn export(
    db: &Store,
    collection: Collection,
    root: &Path,
    updates: &mpsc::Sender<ReceiveProgress>,
) -> anyhow::Result<()> {
    let total_files = collection.len() as u64;
    for (i, (name, hash)) in collection.iter().enumerate() {
        let target = root.join(name.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        updates
            .send(ReceiveProgress::Exporting {
                exported_files: i as u64,
                total_files,
                current_file: name.clone(),
            })
            .await?;

        let mut stream = db
            .export_with_opts(ExportOptions {
                hash: *hash,
                target,
                mode: ExportMode::Copy,
            })
            .stream()
            .await;

        while let Some(item) = stream.next().await {
            match item {
                ExportProgressItem::Done => break,
                ExportProgressItem::Error(cause) => {
                    anyhow::bail!("error exporting {}: {}", name, cause)
                }
                _ => {}
            }
        }
    }
    Ok(())
}

// Handles events from the iroh provider to report progress.
async fn handle_provider_event(event: provider::Event, updates: &mpsc::Sender<SendProgress>) {
    trace!("Provider event: {:?}", event);
    match event {
        provider::Event::ClientConnected { connection_id, .. } => {
            updates
                .send(SendProgress::PeerConnected(connection_id.to_string()))
                .await
                .ok();
        }
        provider::Event::ConnectionClosed { .. } => {
            updates.send(SendProgress::PeerDisconnected).await.ok();
        }
        _ => {}
    }
}
