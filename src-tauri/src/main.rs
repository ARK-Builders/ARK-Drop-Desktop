// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{path::PathBuf, str::FromStr, vec};

use anyhow::{anyhow, Context, Result};
use drop_core::FileTransfer;
use drop_core::IrohInstance;
use futures_buffered::join_all;
use futures_buffered::try_join_all;
use iroh_base::ticket::BlobTicket;
use iroh_blobs::export::export_collection;
use iroh_blobs::get::db::DownloadProgress;
use iroh_blobs::BlobFormat;
use tauri::{generate_context, generate_handler, InvokeError, Manager};
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
    let iroh = IrohInstance::new().await?;

    handle.manage(AppState::new(iroh, async_proc_input_tx));

    Ok(())
}

fn main() {
    let (async_proc_input_tx, async_proc_input_rx) = mpsc::channel(1);
    let (async_proc_output_tx, mut async_proc_output_rx) = mpsc::channel(1);

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();

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
            recieve_files,
            open_file,
            is_valid_ticket
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

fn event_handler<R: tauri::Runtime>(message: Event, manager: &impl Manager<R>) {
    match message {
        Event::Files(progress) => {
            manager.emit_all("download_progress", &progress).unwrap();
        }
    }
}

#[tauri::command]
async fn generate_ticket(
    state: tauri::State<'_, AppState>,
    paths: Vec<PathBuf>,
) -> Result<BlobTicket, InvokeError> {
    state
        .iroh
        .send_files(&paths)
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!(e)))
}

#[tauri::command]
async fn recieve_files(
    state: tauri::State<'_, AppState>,
    ticket: String,
) -> Result<(), InvokeError> {
    let async_proc_input_tx = state.inner.lock().await;

    let mut handles = Vec::new();

    let files = state
        .iroh
        .recieve_files(ticket, |progress| {
            let async_proc_input_tx = async_proc_input_tx.clone();

            let handle =
                tokio::spawn(async move { async_proc_input_tx.send(Event::Files(progress)).await });

            handles.push(handle);
        })
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!(e)))?;

    for handle in handles {
        let _ = handle.await;
    }

    let outpath = dirs::download_dir().unwrap();

    state
        .iroh
        .export_collection(files, outpath)
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!(e)))?;

    return Ok(());
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
