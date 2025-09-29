// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{anyhow, Result};
use drop_core::{BlobTicket, FileTransfer, FileTransferHandle, IrohInstance};
use dropx_sender::SendFilesBubble;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::ipc::InvokeError;
use tauri::{
    generate_context, generate_handler, tray::TrayIconBuilder, AppHandle, Emitter, Manager,
};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

struct AppState {
    pub iroh: IrohInstance,
    inner: Mutex<mpsc::Sender<Event>>,
    // Store active send bubble to keep it alive
    active_send_bubble: Arc<Mutex<Option<SendFilesBubble>>>,
}

enum Event {
    Files(Vec<FileTransfer>),
}

impl AppState {
    fn new(iroh: IrohInstance, async_proc_input_tx: mpsc::Sender<Event>) -> Self {
        AppState {
            iroh,
            inner: Mutex::new(async_proc_input_tx),
            active_send_bubble: Arc::new(Mutex::new(None)),
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();

            // Setup system tray icon
            let _ = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("ARK Drop - File Transfer")
                .build(app)?;

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
    let async_proc_input_tx = state.inner.lock().await.clone();

    // Create channel for progress updates during sending
    let (tx, rx) = std::sync::mpsc::channel::<Vec<FileTransfer>>();

    // Spawn task to handle sending progress updates
    let _progress_handle = tokio::spawn(async move {
        while let Ok(files) = rx.recv() {
            let _ = async_proc_input_tx.send(Event::Files(files)).await;
        }
    });

    // Get both ticket and bubble from send_files
    let (ticket, bubble) = state
        .iroh
        .send_files(paths, Arc::new(FileTransferHandle(tx)))
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!(e)))?;

    // Store the bubble to keep it alive
    *state.active_send_bubble.lock().await = Some(bubble);

    // Spawn a task to manage the sender lifecycle
    let state_bubble = Arc::clone(&state.active_send_bubble);
    tokio::spawn(async move {
        // Wait for completion like ark-core CLI
        loop {
            let is_finished = {
                if let Some(bubble) = state_bubble.lock().await.as_ref() {
                    bubble.is_finished()
                } else {
                    true // No bubble, exit
                }
            };

            if is_finished {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Clear the bubble when done
        *state_bubble.lock().await = None;
    });

    Ok(ticket)
}

#[tauri::command]
async fn receive_files(
    state: tauri::State<'_, AppState>,
    ticket: String,
) -> Result<PathBuf, InvokeError> {
    let async_proc_input_tx = state.inner.lock().await.clone();

    let (tx, rx) = std::sync::mpsc::channel::<Vec<FileTransfer>>();

    // Spawn task to handle receiving progress updates
    let _handle = tokio::spawn(async move {
        while let Ok(files) = rx.recv() {
            let _ = async_proc_input_tx.send(Event::Files(files)).await;
        }
    });

    // Determine output directory
    let output_dir = if let Some(path) = dirs::download_dir() {
        path
    } else {
        // Android download path
        PathBuf::from("/storage/emulated/0/Download/")
    };

    // Receive files with proper file writing
    let _collection = state
        .iroh
        .receive_files(ticket, output_dir.clone(), Arc::new(FileTransferHandle(tx)))
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!(e)))?;

    // Return the output directory where files were saved
    Ok(output_dir)
}

#[tauri::command]
fn open_directory(directory: PathBuf) -> Result<(), InvokeError> {
    open::that(directory).map_err(|e| InvokeError::from_anyhow(anyhow!(e)))
}

#[tauri::command]
fn is_valid_ticket(ticket: String) -> Result<bool, InvokeError> {
    // With ark-core, we can simply try to parse the ticket
    match BlobTicket::parse(&ticket) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
