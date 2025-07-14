// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use drop_core::{self, receive_file, send_file, ReceiveProgress, SendProgress, ShutdownHandle};

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tauri::ipc::InvokeError;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "payload")]
enum AppEvent {
    Send(SendProgress),
    Receive(ReceiveProgress),
}

struct AppState {
    progress_emitter: mpsc::Sender<AppEvent>,
    shutdown_handle: Arc<Mutex<Option<ShutdownHandle>>>,
}

impl AppState {
    fn new(progress_emitter: mpsc::Sender<AppEvent>) -> Self {
        Self {
            progress_emitter,
            shutdown_handle: Arc::new(Mutex::new(None)),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (progress_tx, progress_rx) = mpsc::channel(32);

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            app.manage(AppState::new(progress_tx));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                async_process_model(progress_rx, handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            generate_ticket,
            receive_files,
            cancel_send,
            open_directory,
            is_valid_ticket,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn async_process_model(mut progress_rx: mpsc::Receiver<AppEvent>, handle: AppHandle) {
    while let Some(event) = progress_rx.recv().await {
        event_handler(event, &handle);
    }
}

fn event_handler(event: AppEvent, manager: &AppHandle) {
    let (event_name, payload) = match event {
        AppEvent::Send(progress) => ("send_progress", serde_json::to_value(progress).unwrap()),
        AppEvent::Receive(progress) => {
            ("receive_progress", serde_json::to_value(progress).unwrap())
        }
    };

    if let Err(e) = manager.emit(event_name, payload) {
        eprintln!("Failed to emit event '{}': {:?}", event_name, e);
    }
}

#[tauri::command]
async fn generate_ticket(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> Result<String, InvokeError> {
    let path_str = paths.first().ok_or_else(|| {
        InvokeError::from_anyhow(anyhow!("No path provided to generate a ticket."))
    })?;
    let path = PathBuf::from(path_str);

    let (tx, mut rx) = mpsc::channel(32);

    let main_emitter = state.progress_emitter.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(progress) = rx.recv().await {
            main_emitter.send(AppEvent::Send(progress)).await.ok();
        }
    });

    match send_file(path, tx).await {
        Ok((ticket, handle)) => {
            let mut shutdown_guard = state.shutdown_handle.lock().await;
            *shutdown_guard = Some(handle);
            Ok(ticket)
        }
        Err(e) => Err(InvokeError::from_anyhow(e)),
    }
}

#[tauri::command]
async fn receive_files(
    state: tauri::State<'_, AppState>,
    ticket: String,
) -> Result<String, InvokeError> {
    let (tx, mut rx) = mpsc::channel(32);

    let main_emitter = state.progress_emitter.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(progress) = rx.recv().await {
            main_emitter.send(AppEvent::Receive(progress)).await.ok();
        }
    });

    receive_file(ticket, tx)
        .await
        .map_err(InvokeError::from_anyhow)
}

#[tauri::command]
async fn cancel_send(state: tauri::State<'_, AppState>) -> Result<(), InvokeError> {
    let mut shutdown_guard = state.shutdown_handle.lock().await;
    if let Some(handle) = shutdown_guard.take() {
        // Dropping the handle sends the shutdown signal.
        drop(handle);
        println!("Send operation cancelled.");
    } else {
        eprintln!("No active send operation to cancel.");
    }
    Ok(())
}

#[tauri::command]
fn open_directory(directory: PathBuf) -> Result<(), InvokeError> {
    open::that(&directory).map_err(|e| {
        InvokeError::from_anyhow(anyhow!(
            "Failed to open directory '{}': {}",
            directory.display(),
            e
        ))
    })
}

#[tauri::command]
fn is_valid_ticket(ticket: String) -> bool {
    iroh_blobs::ticket::BlobTicket::from_str(&ticket).is_ok()
}
