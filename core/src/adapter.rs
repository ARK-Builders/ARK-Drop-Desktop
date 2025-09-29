use crate::{
    error::{IrohError, IrohResult},
    FileTransfer, FileTransferHandle,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

// ARK-Core imports
use anyhow::{anyhow, Result};
use drop_entities::Profile;
use dropx_receiver::{
    receive_files, ReceiveFilesBubble, ReceiveFilesConnectingEvent, ReceiveFilesReceivingEvent,
    ReceiveFilesRequest as ReceiverRequest, ReceiveFilesSubscriber, ReceiverConfig,
    ReceiverProfile,
};
use dropx_sender::{
    send_files, SendFilesRequest, SenderConfig, SenderFile, SenderFileData, SenderProfile,
};

pub struct DropCoreAdapter;

impl DropCoreAdapter {
    pub async fn send_files(&self, files: Vec<PathBuf>, profile: Profile) -> Result<()> {
        if files.is_empty() {
            return Err(anyhow!("Cannot send an empty list of files"));
        }

        // Validate all files exist before starting
        for path in &files {
            if !path.exists() {
                return Err(anyhow!("File does not exist: {}", path.display()));
            }
            if !path.is_file() {
                return Err(anyhow!("Path is not a file: {}", path.display()));
            }
        }

        let sender_files = files.iter().map(|path| SenderFile {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            data: Arc::new(FileDataAdapter::from_path(path.clone())?),
        });

        let request = SendFilesRequest {
            files,
            profile: SenderProfile {
                name: profile.name,
                avatar_b64: profile.avatar_b64,
            },
            config: SenderConfig::default(),
        };

        let bubble = send_files(request).await.map_err(|e| anyhow!(e))?;

        let subscriber = FileSendSubscriber::new(verbose);
        bubble.subscribe(Arc::new(subscriber));

        println!("📦 Ready to send files!");
        println!("🎫 Ticket: \"{}\"", bubble.get_ticket());
        println!("🔑 Confirmation: \"{}\"", bubble.get_confirmation());
        println!("⏳ Waiting for receiver... (Press Ctrl+C to cancel)");

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("🚫 Cancelling file transfer...");
                let _ = bubble.cancel().await;
                println!("✅ Transfer cancelled");
            }
            _ = wait_for_send_completion(&bubble) => {
                println!("✅ All files sent successfully!");
            }
        }

        Ok(())
    }

    pub async fn receive_files(
        &self,
        ticket_str: String,
        handle: Arc<FileTransferHandle>,
    ) -> IrohResult<Collection> {
        // Parse ticket to extract confirmation
        let (ticket, confirmation) = TicketWrapper::parse(&ticket_str)?;

        // Create default receiver profile
        let profile = ReceiverProfile {
            name: "Anonymous".to_string(),
            avatar_b64: None,
        };

        // Create receive request
        let request = ReceiverRequest {
            ticket,
            confirmation,
            profile,
            config: Some(ReceiverConfig::default()),
        };

        // Create progress subscriber that bridges to our FileTransferHandle
        let progress_subscriber = Arc::new(ProgressSubscriber::new(handle.clone()));

        // Use ark-core to receive files
        let bubble = receive_files(request)
            .await
            .map_err(|e| IrohError::DownloadError(e.to_string()))?;

        // Subscribe to progress updates
        bubble.subscribe(progress_subscriber);

        // Start the receive operation
        bubble
            .start()
            .map_err(|e| IrohError::DownloadError(e.to_string()))?;

        // Wait for completion and extract collection
        let collection = self.wait_for_completion(Arc::new(bubble), handle).await?;

        Ok(collection)
    }

    // Helper methods
    async fn convert_paths_to_sender_files(
        &self,
        paths: Vec<PathBuf>,
    ) -> IrohResult<Vec<SenderFile>> {
        let mut sender_files = Vec::new();

        for path in paths {
            let file_name = path
                .file_name()
                .ok_or_else(|| IrohError::NodeError("Invalid file name".to_string()))?
                .to_string_lossy()
                .to_string();

            let file_data = FileDataAdapter::from_path(path)?;

            sender_files.push(SenderFile {
                name: file_name,
                data: Arc::new(file_data),
            });
        }

        Ok(sender_files)
    }

    async fn wait_for_completion(
        &self,
        bubble: Arc<ReceiveFilesBubble>,
        _handle: Arc<FileTransferHandle>,
    ) -> IrohResult<Collection> {
        // Wait for the operation to finish
        while !bubble.is_finished() && !bubble.is_cancelled() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        if bubble.is_cancelled() {
            return Err(IrohError::DownloadError(
                "Transfer was cancelled".to_string(),
            ));
        }

        // For now, return an empty collection
        // In a full implementation, we'd extract file information from the bubble
        // or from the progress tracking we've done
        Ok(Collection::new())
    }
}

// Adapter to bridge ark-core progress events to our FileTransfer format
struct ProgressSubscriber {
    handle: Arc<FileTransferHandle>,
}

impl ProgressSubscriber {
    fn new(handle: Arc<FileTransferHandle>) -> Self {
        Self { handle }
    }
}

impl dropx_receiver::ReceiveFilesSubscriber for ProgressSubscriber {
    fn get_id(&self) -> String {
        "progress_subscriber".to_string()
    }

    fn log(&self, _message: String) {
        // Log messages can be ignored for now
    }

    fn notify_receiving(&self, event: dropx_receiver::ReceiveFilesReceivingEvent) {
        // Convert ark-core receiving event to our FileTransfer format
        // This is a simplified conversion
        let file_transfer = FileTransfer {
            name: format!("file_{}", event.id),
            transferred: event.data.len() as u64,
            total: event.data.len() as u64, // We'll need to track total separately
        };

        // Send progress update
        let _ = self.handle.0.send(vec![file_transfer]);
    }

    fn notify_connecting(&self, event: dropx_receiver::ReceiveFilesConnectingEvent) {
        // Convert connecting event to initial progress with file info
        let files: Vec<FileTransfer> = event
            .files
            .iter()
            .map(|f| FileTransfer {
                name: f.name.clone(),
                transferred: 0,
                total: f.len,
            })
            .collect();

        let _ = self.handle.0.send(files);
    }
}

// File data adapter to read from filesystem for ark-core
// Based on the CLI implementation pattern
struct FileDataAdapter {
    is_finished: std::sync::atomic::AtomicBool,
    path: PathBuf,
    reader: std::sync::RwLock<Option<std::fs::File>>,
    size: u64,
    bytes_read: std::sync::atomic::AtomicU64,
}

impl FileDataAdapter {
    fn from_path(path: PathBuf) -> Result<Self> {
        let metadata = std::fs::metadata(&path)
            .map_err(|e| IrohError::NodeError(format!("Failed to get file metadata: {}", e)))?;

        Ok(Self {
            is_finished: std::sync::atomic::AtomicBool::new(false),
            path,
            reader: std::sync::RwLock::new(None),
            size: metadata.len(),
            bytes_read: std::sync::atomic::AtomicU64::new(0),
        })
    }
}

impl SenderFileData for FileDataAdapter {
    fn len(&self) -> u64 {
        self.size
    }

    fn read(&self) -> Option<u8> {
        use std::io::Read;
        use std::sync::atomic::Ordering;

        if self.is_finished.load(Ordering::Relaxed) {
            return None;
        }

        if self.reader.read().unwrap().is_none() {
            match std::fs::File::open(&self.path) {
                Ok(file) => {
                    *self.reader.write().unwrap() = Some(file);
                }
                Err(_) => {
                    self.is_finished.store(true, Ordering::Relaxed);
                    return None;
                }
            }
        }

        let mut reader = self.reader.write().unwrap();
        if let Some(file) = reader.as_mut() {
            let mut buffer = [0u8; 1];
            match file.read(&mut buffer) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        *reader = None;
                        self.is_finished.store(true, Ordering::Relaxed);
                        None
                    } else {
                        Some(buffer[0])
                    }
                }
                Err(_) => {
                    *reader = None;
                    self.is_finished.store(true, Ordering::Relaxed);
                    None
                }
            }
        } else {
            None
        }
    }

    fn read_chunk(&self, size: u64) -> Vec<u8> {
        use std::{
            io::{Read, Seek, SeekFrom},
            sync::atomic::Ordering,
        };

        if self.is_finished.load(Ordering::Acquire) {
            return Vec::new();
        }

        // Atomically claim the next chunk position
        let current_position = self.bytes_read.fetch_add(size, Ordering::AcqRel);

        // Check if we've already passed the end of the file
        if current_position >= self.size {
            self.bytes_read.store(self.size, Ordering::Release);
            self.is_finished.store(true, Ordering::Release);
            return Vec::new();
        }

        // Calculate how much to actually read
        let remaining = self.size - current_position;
        let to_read = std::cmp::min(size, remaining) as usize;

        // Open a new file handle for this read operation
        let mut file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(_) => {
                self.is_finished.store(true, Ordering::Release);
                return Vec::new();
            }
        };

        // Seek to the claimed position
        if file.seek(SeekFrom::Start(current_position)).is_err() {
            self.is_finished.store(true, Ordering::Release);
            return Vec::new();
        }

        // Read the chunk
        let mut buffer = vec![0u8; to_read];
        match file.read_exact(&mut buffer) {
            Ok(()) => {
                // Check if we've finished reading the entire file
                if current_position + to_read as u64 >= self.size {
                    self.is_finished.store(true, Ordering::Release);
                }
                buffer
            }
            Err(_) => {
                self.is_finished.store(true, Ordering::Release);
                Vec::new()
            }
        }
    }
}

// Collection type to match current interface
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    files: Vec<(String, String)>, // (name, hash) pairs
}

impl Collection {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add_file(&mut self, name: String, hash: String) {
        self.files.push((name, hash));
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, String)> {
        self.files.iter()
    }
}

// Ticket wrapper to handle ark-core's ticket + confirmation format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketWrapper {
    ticket: String,
    confirmation: u8,
}

impl TicketWrapper {
    pub fn new(ticket: String, confirmation: u8) -> Self {
        Self {
            ticket,
            confirmation,
        }
    }

    pub fn parse(combined: &str) -> IrohResult<(String, u8)> {
        // Parse combined ticket format: "ticket:confirmation"
        if let Some((ticket, conf_str)) = combined.rsplit_once(':') {
            if let Ok(confirmation) = conf_str.parse::<u8>() {
                return Ok((ticket.to_string(), confirmation));
            }
        }

        // Fallback: assume no confirmation for backward compatibility
        Ok((combined.to_string(), 0))
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.ticket, self.confirmation)
    }
}
