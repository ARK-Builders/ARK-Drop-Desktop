// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{anyhow, Result};
use drop_core::metadata::FileTransfer;
use drop_core::send::SendEvent;
use drop_core::IrohInstance;
use iroh_blobs::ticket::BlobTicket;
use iroh_blobs::BlobFormat;
use std::str::FromStr;
use std::sync::Arc;
use std::{path::PathBuf, vec};
use tauri::ipc::InvokeError;
use tauri::{generate_context, generate_handler, AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

struct AppState {
    inner: Mutex<mpsc::Sender<Event>>,
    sender: IrohInstance,
}

enum Event {
    Files(Vec<FileTransfer>),
}

impl AppState {
    async fn new(async_proc_input_tx: mpsc::Sender<Event>) -> Result<Self> {
        let (tx, _rx) = tokio::sync::mpsc::channel::<SendEvent>(32);
        let sender = IrohInstance::sender(Arc::new(tx))
            .await
            .map_err(|e| anyhow!("Failed to create sender: {}", e))?;

        Ok(AppState {
            inner: Mutex::new(async_proc_input_tx),
            sender,
        })
    }
}

async fn setup<R: tauri::Runtime>(
    handle: &tauri::AppHandle<R>,
    async_proc_input_tx: mpsc::Sender<Event>,
) -> Result<()> {
    let state = AppState::new(async_proc_input_tx)
        .await
        .map_err(|e| anyhow!("Failed to initialize app state: {}", e))?;
    handle.manage(state);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (async_proc_input_tx, async_proc_input_rx) = mpsc::channel(1);
    let (async_proc_output_tx, mut async_proc_output_rx) = mpsc::channel(1);

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                if let Err(e) = async_process_model(async_proc_input_rx, async_proc_output_tx).await
                {
                    eprintln!("Async process model failed: {:?}", e);
                }
            });

            tauri::async_runtime::spawn(async move {
                if let Err(err) = setup(&handle, async_proc_input_tx).await {
                    eprintln!("Setup failed: {:?}", err);
                }

                while let Some(output) = async_proc_output_rx.recv().await {
                    event_handler(output, &handle);
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
        .map_err(|e| eprintln!("Error while running tauri application: {:?}", e))
        .ok();
}

async fn async_process_model(
    mut input_rx: mpsc::Receiver<Event>,
    output_tx: mpsc::Sender<Event>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    while let Some(input) = input_rx.recv().await {
        output_tx
            .send(input)
            .await
            .map_err(|e| anyhow!("Failed to send output: {}", e))?;
    }
    Ok(())
}

fn event_handler(message: Event, manager: &AppHandle) {
    match message {
        Event::Files(progress) => {
            if let Err(e) = manager.emit("download_progress", &progress) {
                eprintln!("Failed to emit download_progress event: {:?}", e);
            }
        }
    }
}

#[tauri::command]
fn get_env(key: &str) -> Result<String, InvokeError> {
    std::env::var(key)
        .map_err(|e| InvokeError::from_anyhow(anyhow!("Failed to get env var: {}", e)))
}

#[tauri::command]
async fn generate_ticket(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> Result<BlobTicket, InvokeError> {
    state
        .sender
        .send_files(paths)
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!("Failed to generate ticket: {}", e)))
}

#[tauri::command]
async fn receive_files(
    state: tauri::State<'_, AppState>,
    ticket: String,
) -> Result<String, InvokeError> {
    let async_proc_input_tx = state.inner.lock().await.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<FileTransfer>>(2);

    let handle = tokio::spawn(async move {
        while let Some(files) = rx.recv().await {
            if let Err(e) = async_proc_input_tx.send(Event::Files(files)).await {
                eprintln!("Failed to send files event: {:?}", e);
                break;
            }
        }
    });

    let outpath = dirs::download_dir().unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("files")
    });

    std::fs::create_dir_all(&outpath)
        .map_err(|e| InvokeError::from_anyhow(anyhow!("Failed to create directory: {}", e)))?;

    let _files = IrohInstance::receive_files(ticket, tx)
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!("Failed to receive files: {}", e)))?;

    handle
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!("Failed to await handle: {}", e)))?;

    outpath
        .to_str()
        .ok_or_else(|| InvokeError::from_anyhow(anyhow!("Failed to convert path to string")))
        .map(|s| s.to_owned())
}

#[tauri::command]
fn open_directory(directory: PathBuf) -> Result<(), InvokeError> {
    open::that(directory)
        .map_err(|e| InvokeError::from_anyhow(anyhow!("Failed to open directory: {}", e)))
}

#[tauri::command]
fn is_valid_ticket(ticket: String) -> Result<bool, InvokeError> {
    let ticket = BlobTicket::from_str(&ticket)
        .map_err(|e| InvokeError::from_anyhow(anyhow!("Failed to parse ticket: {}", e)))?;
    Ok(ticket.format() == BlobFormat::HashSeq)
}
