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

/// Application state shared across all Tauri commands.
struct AppState {
    /// Iroh instance for peer-to-peer file transfers
    pub iroh: IrohInstance,
    /// Channel sender for internal event communication
    inner: Mutex<mpsc::Sender<Event>>,
    /// Active send bubble to keep it alive during transfers
    active_send_bubble: Arc<Mutex<Option<SendFilesBubble>>>,
    /// Custom download directory set by user
    custom_download_dir: Mutex<Option<PathBuf>>,
    /// User display name for file transfer identification
    user_display_name: Mutex<Option<String>>,
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
            custom_download_dir: Mutex::new(None),
            user_display_name: Mutex::new(None),
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
        .plugin(tauri_plugin_store::Builder::default().build())
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
            set_download_directory,
            get_download_directory,
            set_display_name,
            get_display_name,
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

/// Gets an environment variable value.
///
/// # Arguments
/// * `key` - The environment variable name
///
/// # Returns
/// The value of the environment variable, or an empty string if not found
#[tauri::command]
fn get_env(key: &str) -> String {
    std::env::var(String::from(key)).unwrap_or(String::from(""))
}

/// Generates a ticket for sending files.
///
/// Creates a ticket that encodes the file paths and connection information,
/// which can be shared with a receiver to initiate a file transfer.
///
/// # Arguments
/// * `paths` - List of file paths to include in the transfer
///
/// # Returns
/// A BlobTicket containing the transfer information
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

/// Receives files using a transfer ticket.
///
/// Downloads files from a sender using the provided ticket and saves them to
/// the configured download directory.
///
/// # Arguments
/// * `ticket` - The transfer ticket string from the sender
///
/// # Returns
/// The path to the directory where files were saved
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

    let output_dir = {
        let custom_dir = state.custom_download_dir.lock().await;
        custom_dir
            .clone()
            .or_else(|| dirs::download_dir())
            .unwrap_or_else(|| PathBuf::from("/storage/emulated/0/Download/"))
    };

    // Get display name with fallback chain: custom → system username → None
    let display_name = {
        let custom_name = state.user_display_name.lock().await;
        custom_name.clone()
            .or_else(|| Some(whoami::username()))
    };

    // Receive files with proper file writing
    let _collection = state
        .iroh
        .receive_files(
            ticket,
            output_dir.clone(),
            Arc::new(FileTransferHandle(tx)),
            display_name
        )
        .await
        .map_err(|e| InvokeError::from_anyhow(anyhow!(e)))?;

    // Return the output directory where files were saved
    Ok(output_dir)
}

/// Sets a custom download directory for received files.
///
/// # Arguments
/// * `path` - The filesystem path to the directory
///
/// # Errors
/// Returns an error if the path doesn't exist or is not a directory
#[tauri::command]
async fn set_download_directory(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), InvokeError> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Err(InvokeError::from_anyhow(anyhow!(
            "Directory does not exist: {}",
            path
        )));
    }

    if !path_buf.is_dir() {
        return Err(InvokeError::from_anyhow(anyhow!(
            "Path is not a directory: {}",
            path
        )));
    }

    let mut custom_dir = state.custom_download_dir.lock().await;
    *custom_dir = Some(path_buf);

    Ok(())
}

/// Gets the current download directory.
///
/// Returns the custom directory if set, otherwise returns the system default
/// download directory, or the Android download path as a fallback.
///
/// # Returns
/// The absolute path to the download directory as a string
#[tauri::command]
async fn get_download_directory(state: tauri::State<'_, AppState>) -> Result<String, InvokeError> {
    let custom_dir = state.custom_download_dir.lock().await;

    let output_dir = custom_dir
        .clone()
        .or_else(|| dirs::download_dir())
        .unwrap_or_else(|| PathBuf::from("/storage/emulated/0/Download/"));

    Ok(output_dir.to_string_lossy().to_string())
}

/// Sets a custom display name for the user.
///
/// # Arguments
/// * `name` - The display name to set
///
/// # Errors
/// Returns an error if the name is empty or exceeds 50 characters
#[tauri::command]
async fn set_display_name(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<(), InvokeError> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Err(InvokeError::from_anyhow(anyhow!("Display name cannot be empty")));
    }

    if trimmed.len() > 50 {
        return Err(InvokeError::from_anyhow(anyhow!("Display name cannot exceed 50 characters")));
    }

    let mut display_name = state.user_display_name.lock().await;
    *display_name = Some(trimmed.to_string());

    Ok(())
}

/// Gets the current display name.
///
/// Returns the custom name if set, otherwise returns the system username.
///
/// # Returns
/// The display name as a string
#[tauri::command]
async fn get_display_name(state: tauri::State<'_, AppState>) -> Result<String, InvokeError> {
    let custom_name = state.user_display_name.lock().await;

    let name = custom_name
        .clone()
        .unwrap_or_else(|| whoami::username());

    Ok(name)
}

/// Opens a directory in the system's file manager.
///
/// # Arguments
/// * `directory` - Path to the directory to open
///
/// # Errors
/// Returns an error if the directory cannot be opened
#[tauri::command]
fn open_directory(directory: PathBuf) -> Result<(), InvokeError> {
    open::that(directory).map_err(|e| InvokeError::from_anyhow(anyhow!(e)))
}

/// Validates a transfer ticket format.
///
/// # Arguments
/// * `ticket` - The ticket string to validate
///
/// # Returns
/// `true` if the ticket is valid, `false` otherwise
#[tauri::command]
fn is_valid_ticket(ticket: String) -> Result<bool, InvokeError> {
    match BlobTicket::parse(&ticket) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
