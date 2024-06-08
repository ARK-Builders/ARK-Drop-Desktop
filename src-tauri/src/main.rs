// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use futures_buffered::try_join_all;
use iroh::{
    client::blobs::{AddOutcome, WrapOption},
    node::Node,
};
use iroh_base::ticket::BlobTicket;
use iroh_blobs::{util::SetTagOption, BlobFormat};
use serde::Serialize;
use tauri::{InvokeError, Manager};
use walkdir::WalkDir;

struct AppState {
    pub iroh: Node<iroh_blobs::store::mem::Store>,
}

impl AppState {
    fn new(iroh: Node<iroh_blobs::store::mem::Store>) -> Self {
        AppState { iroh }
    }
}

async fn setup<R: tauri::Runtime>(handle: tauri::AppHandle<R>) -> Result<()> {
    // create the iroh node
    let node = iroh::node::Node::memory().spawn().await?;

    handle.manage(AppState::new(node));

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();

            tauri::async_runtime::spawn(async move {
                if let Err(err) = setup(handle).await {
                    eprintln!("failed: {:?}", err);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            generate_ticket,
            recieve_files,
            open_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn create_file_collection(
    db: &Node<iroh_blobs::store::mem::Store>,
    path: &PathBuf,
) -> Result<Vec<(PathBuf, AddOutcome)>> {
    try_join_all(
        WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .map(|path| async move {
                let add_progress = db
                    .blobs
                    .add_from_path(path.clone(), true, SetTagOption::Auto, WrapOption::NoWrap)
                    .await;
                match add_progress {
                    Ok(add_progress) => {
                        let progress = add_progress.finish().await;
                        if let Ok(progress) = progress {
                            Ok((path, progress))
                        } else {
                            return Err(progress.err().unwrap());
                        }
                    }
                    Err(e) => return Err(e),
                }
            }),
    )
    .await
}

#[tauri::command]
async fn generate_ticket(
    state: tauri::State<'_, AppState>,
    path: PathBuf,
) -> Result<BlobTicket, InvokeError> {
    let node = &state.iroh;

    let outcome = create_file_collection(node, &path)
        .await
        .map_err(InvokeError::from_anyhow)?;

    let collection = outcome
        .into_iter()
        .map(|(path, outcome)| {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let hash = outcome.hash;
            return (name, hash);
        })
        .collect();

    let (hash, _) = node
        .blobs
        .create_collection(collection, SetTagOption::Auto, Default::default())
        .await
        .map_err(InvokeError::from_anyhow)?;

    node.blobs
        .share(hash, BlobFormat::HashSeq, Default::default())
        .await
        .map_err(InvokeError::from_anyhow)
}

#[derive(Serialize)]
struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

#[derive(Serialize)]
struct RecieveFilesResponse {
    pub files: Vec<FileInfo>,
    pub downloaded_size: u64,
}

#[tauri::command]
async fn recieve_files(
    state: tauri::State<'_, AppState>,
    ticket: String,
) -> Result<RecieveFilesResponse, InvokeError> {
    let node = &state.iroh;

    let ticket = BlobTicket::from_str(&ticket)
        .map_err(|e| InvokeError::from_anyhow(anyhow::anyhow!("failed to parse ticket: {}", e)))?;

    if ticket.format() != BlobFormat::HashSeq {
        return Err(InvokeError::from_anyhow(anyhow::anyhow!(
            "unsupported format: {:?}",
            ticket.format()
        )));
    }

    let download_stream = node
        .blobs
        .download_hash_seq(ticket.hash(), ticket.node_addr().clone())
        .await
        .map_err(InvokeError::from_anyhow)?;

    let outcome = download_stream
        .await
        .context("unable to download hash")
        .map_err(InvokeError::from_anyhow)?;

    let collection = node
        .blobs
        .get_collection(ticket.hash())
        .await
        .context("expect hash with `BlobFormat::HashSeq` to be a collection")
        .map_err(InvokeError::from_anyhow)?;

    let mut files: Vec<FileInfo> = Vec::new();

    for (name, hash) in collection.iter() {
        let content = node
            .blobs
            .read_to_bytes(*hash)
            .await
            .map_err(InvokeError::from_anyhow)?;

        let path = PathBuf::from(name);

        let file_path = dirs::desktop_dir()
            .context("failed to get current directory")
            .map_err(InvokeError::from_anyhow)?
            .join(
                path.file_name()
                    .context("failed to get file name")
                    .map_err(InvokeError::from_anyhow)?,
            );

        files.push(FileInfo {
            path: file_path.clone(),
            name: name.clone(),
            size: content.len() as u64,
        });

        std::fs::write(&file_path, content)
            .context("failed to write file")
            .map_err(InvokeError::from_anyhow)?;
    }

    Ok(RecieveFilesResponse {
        files,
        downloaded_size: outcome.downloaded_size,
    })
}

#[tauri::command]
fn open_file(file: PathBuf) -> Result<(), InvokeError> {
    open::that(file)
        .map_err(|e| InvokeError::from_anyhow(anyhow::anyhow!("failed to open file: {}", e)))
}

#[tauri::command]
fn is_valid_ticket(ticket: String) -> Result<bool, InvokeError> {
    let ticket = BlobTicket::from_str(&ticket)
        .map_err(|e| InvokeError::from_anyhow(anyhow::anyhow!("failed to parse ticket: {}", e)))?;

    Ok(ticket.format() == BlobFormat::HashSeq)
}
