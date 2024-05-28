// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use futures_buffered::{try_join_all, BufferedStreamExt};
use futures_lite::StreamExt;
use iroh::{client::blobs::WrapOption, node::Node};
use iroh_base::ticket::BlobTicket;
use iroh_blobs::{format::collection::{self, Collection}, store::ImportMode, util::SetTagOption, BlobFormat, Hash, TempTag};
use iroh_net::Endpoint;
use rand::Rng;
use tauri::{InvokeError, Manager};
use walkdir::WalkDir;

struct AppState {
    pub iroh: Node<iroh_blobs::store::fs::Store>,
}

impl AppState {
    fn new(iroh: Node<iroh_blobs::store::fs::Store>) -> Self {
        AppState { iroh }
    }
}

async fn setup<R: tauri::Runtime>(handle: tauri::AppHandle<R>) -> Result<()> {
    let suffix = rand::thread_rng().gen::<[u8; 16]>();
    let iroh_data_dir =
        std::env::current_dir()?.join(format!(".ark-drop-data-{}", hex::encode(suffix)));

    // create the iroh node
    let node = iroh::node::Node::persistent(iroh_data_dir)
        .await?
        .spawn()
        .await?;
    handle.manage(AppState::new(node));

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();

            tauri::async_runtime::spawn(async move {
                println!("starting backend...");
                if let Err(err) = setup(handle).await {
                    eprintln!("failed: {:?}", err);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![generate_ticket])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn canonicalized_path_to_string(
    path: impl AsRef<Path>,
    must_be_relative: bool,
) -> anyhow::Result<String> {
    let mut path_str = String::new();
    let parts = path
        .as_ref()
        .components()
        .filter_map(|c| match c {
            Component::Normal(x) => {
                let c = match x.to_str() {
                    Some(c) => c,
                    None => return Some(Err(anyhow::anyhow!("invalid character in path"))),
                };

                if !c.contains('/') && !c.contains('\\') {
                    Some(Ok(c))
                } else {
                    Some(Err(anyhow::anyhow!("invalid path component {:?}", c)))
                }
            }
            Component::RootDir => {
                if must_be_relative {
                    Some(Err(anyhow::anyhow!("invalid path component {:?}", c)))
                } else {
                    path_str.push('/');
                    None
                }
            }
            _ => Some(Err(anyhow::anyhow!("invalid path component {:?}", c))),
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let parts = parts.join("/");
    path_str.push_str(&parts);
    Ok(path_str)
}


async fn create_file_collection(
    db: &Node<iroh_blobs::store::fs::Store>,
    path: PathBuf,
) -> Result<()> {



    let stream = try_join_all(WalkDir::new(&path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf()).map(|path| {
            async move {
                let addProgress = db.blobs.add_from_path(path, true, SetTagOption::Auto, WrapOption::NoWrap).await;
                match addProgress {
                    Ok(addProgress) => {
                        let progress = addProgress.finish().await;
                        Ok((path,progress))

                    }
                    Err(e) => {
                        Err(e)
                    }
                }
            }
        })).await?;

    

    Ok(())
}


#[tauri::command]
async fn generate_ticket(
    state: tauri::State<'_, AppState>,
    path: PathBuf,
) -> Result<BlobTicket, InvokeError> {
    let iroh_node = &state.iroh;

    let db = iroh_node;

    create_file_collection(db, path).await;

    let endpoint = state.iroh.endpoint();

    while endpoint.my_relay().is_none() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
   

    let addr = endpoint
        .my_addr()
        .await
        .map_err(|e| InvokeError::from(format!("failed to get my address: {:?}", e)))?;

    let hash = 

    BlobTicket::new(addr, hash, BlobFormat::HashSeq).map_err(InvokeError::from_anyhow)
}
