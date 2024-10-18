// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{anyhow, Result};
use drop_core::IrohInstance;
use drop_core::{FileTransfer, FileTransferHandle};
use iroh_base::ticket::BlobTicket;
use iroh_blobs::BlobFormat;
use std::sync::Arc;
use std::{path::PathBuf, str::FromStr, vec};
use tauri::ipc::InvokeError;
use tauri::{generate_context, generate_handler, AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
struct AppState {
    pub iroh: IrohInstance,
    inner: Mutex<mpsc::Sender<Event>>,
}

enum Event {
    Files(Vec<FileTransfer>),
}

impl AppState {
    fn new(iroh: IrohInstance, async_proc_input_tx: mpsc::Sender<Event>) -> Self {
        AppState {
            iroh,
            inner: Mutex::new(async_proc_input_tx),
        }
    }
}

async fn setup<R: tauri::Runtime>(
    handle: &tauri::AppHandle<R>,
    async_proc_input_tx: mpsc::Sender<Event>,
) -> Result<()> {
    let iroh = IrohInstance::new().await.map_err(|e| anyhow!(e))?;

    handle.manage(AppState::new(iroh, async_proc_input_tx));

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (async_proc_input_tx, async_proc_input_rx) = mpsc::channel(1);
    let (async_proc_output_tx, mut async_proc_output_rx) = mpsc::channel(1);

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                async_process_model(async_proc_input_rx, async_proc_output_tx).await
            });

            tauri::async_runtime::spawn(async move {
                if let Err(err) = setup(&handle, async_proc_input_tx).await {
                    eprintln!("failed: {:?}", err);
                }

                loop {
                    if let Some(output) = async_proc_output_rx.recv().await {
                        event_handler(output, &handle);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(generate_handler![
            generate_ticket,
            receive_files,
            open_directory,
            is_valid_ticket,
            get_env
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}

async fn async_process_model(
    mut input_rx: mpsc::Receiver<Event>,
    output_tx: mpsc::Sender<Event>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    while let Some(input) = input_rx.recv().await {
        let output = input;
        output_tx.send(output).await?;
    }

    Ok(())
}

fn event_handler(message: Event, manager: &AppHandle) {
    match message {
        Event::Files(progress) => {
            manager.emit("download_progress", &progress).unwrap();
        }
    }
}

#[tauri::command]
fn get_env(key: &str) -> String {
    std::env::var(String::from(key)).unwrap_or(String::from(""))
}

#[tauri::command]
async fn generate_ticket(
    state: tauri::State<'_, AppState>,
    paths: Vec<PathBuf>,
) -> Result<BlobTicket, InvokeError> {
    state
        .iroh
        .send_files(paths)
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!(e)))
}

#[tauri::command]
async fn receive_files(
    state: tauri::State<'_, AppState>,
    ticket: String,
) -> Result<PathBuf, InvokeError> {
    let async_proc_input_tx = state.inner.lock().await.clone();

    let mut handles = Vec::new();

    let (tx, rx) = std::sync::mpsc::channel::<Vec<FileTransfer>>();

    handles.push(tokio::spawn(async move {
        loop {
            let files = rx.recv();
            if let Ok(files) = files {
                let _ = async_proc_input_tx.send(Event::Files(files)).await;
            } else {
                break;
            }
        }
    }));

    let files = state
        .iroh
        .receive_files(ticket, Arc::new(FileTransferHandle(tx)))
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!(e)))?;

    for handle in handles {
        handle.await.unwrap();
    }

    let outpath = if let Some(path) = dirs::download_dir() {
        path
    } else {
        PathBuf::from(".")
    };

    for (name, hash) in files.0.iter() {
        let content = state
            .iroh
            .get_node()
            .0
            .blobs()
            .read_to_bytes(*hash)
            .await
            .map_err(|e| InvokeError::from_anyhow(anyhow!("failed to read blob: {}", e)))?;
        let file_path = outpath.join(name);
        std::fs::write(&file_path, content)
            .map_err(|e| InvokeError::from_anyhow(anyhow!("failed to write file: {}", e)))?;
    }

    Ok(outpath)
}

#[tauri::command]
fn open_directory(directory: PathBuf) -> Result<(), InvokeError> {
    open::that(directory).map_err(|e| InvokeError::from_anyhow(anyhow!(e)))
}

#[tauri::command]
fn is_valid_ticket(ticket: String) -> Result<bool, InvokeError> {
    let ticket = BlobTicket::from_str(&ticket)
        .map_err(|e| InvokeError::from_anyhow(anyhow::anyhow!("failed to parse ticket: {}", e)))?;

    Ok(ticket.format() == BlobFormat::HashSeq)
}
