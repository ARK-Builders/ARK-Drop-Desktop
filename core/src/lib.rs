pub mod error;

use error::{IrohError, IrohResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{mpsc::Sender, Arc};

// ARK-Core imports
use dropx_receiver::{
    receive_files, ReceiveFilesBubble, ReceiveFilesConnectingEvent, ReceiveFilesReceivingEvent,
    ReceiveFilesRequest as ReceiverRequest, ReceiveFilesSubscriber, ReceiverConfig,
    ReceiverProfile,
};
use dropx_sender::{
    send_files, SendFilesBubble, SendFilesConnectingEvent, SendFilesRequest, SendFilesSendingEvent,
    SendFilesSubscriber, SenderConfig, SenderFile, SenderFileData, SenderProfile,
};

// Ticket wrapper to handle ark-core's ticket + confirmation format
#[derive(Debug, Clone)]
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
                // Basic validation: ticket should not be empty
                if ticket.is_empty() {
                    return Err(IrohError::NodeError("Empty ticket".to_string()));
                }
                return Ok((ticket.to_string(), confirmation));
            }
        }

        // For tickets without confirmation, still validate they're not empty
        if combined.trim().is_empty() {
            return Err(IrohError::NodeError("Empty ticket".to_string()));
        }

        // Basic ticket format validation - should be reasonable length and contain valid characters
        if combined.len() < 10 || combined.len() > 200 {
            return Err(IrohError::NodeError("Invalid ticket length".to_string()));
        }

        // Allow alphanumeric, hyphens, underscores, and some special chars typical in base64/hex
        if !combined
            .chars()
            .all(|c| c.is_alphanumeric() || "-_=+/".contains(c))
        {
            return Err(IrohError::NodeError(
                "Invalid ticket characters".to_string(),
            ));
        }

        Ok((combined.to_string(), 0))
    }

    pub fn from_string(combined: &str) -> IrohResult<Self> {
        let (ticket, confirmation) = Self::parse(combined)?;
        Ok(Self::new(ticket, confirmation))
    }

    pub fn is_valid(combined: &str) -> bool {
        Self::parse(combined).is_ok()
    }
}

// Implement Display trait so it serializes as string for Tauri
impl std::fmt::Display for TicketWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.ticket, self.confirmation)
    }
}

// Custom serde implementation to serialize as string for frontend
impl Serialize for TicketWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}:{}", self.ticket, self.confirmation))
    }
}

impl<'de> Deserialize<'de> for TicketWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let (ticket, confirmation) = TicketWrapper::parse(&s)
            .map_err(|_| serde::de::Error::custom("Invalid ticket format"))?;
        Ok(TicketWrapper::new(ticket, confirmation))
    }
}

// Collection type to match current interface
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    files: Vec<(String, String)>, // (name, hash) pairs
}

impl Default for Collection {
    fn default() -> Self {
        Self::new()
    }
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

// Re-export TicketWrapper as BlobTicket for compatibility
pub type BlobTicket = TicketWrapper;

// Main IrohInstance - uses ark-core internally
pub struct IrohInstance;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileTransfer {
    pub name: String,
    pub transferred: u64,
    pub total: u64,
}

pub struct FileTransferHandle(pub Sender<Vec<FileTransfer>>);

impl IrohInstance {
    pub async fn new() -> IrohResult<Self> {
        Ok(Self {})
    }

    pub async fn send_files(
        &self,
        files: Vec<PathBuf>,
        handle: Arc<FileTransferHandle>,
    ) -> IrohResult<(BlobTicket, SendFilesBubble)> {
        if files.is_empty() {
            return Err(IrohError::NodeError(
                "Cannot send an empty list of files".to_string(),
            ));
        }

        // Validate all files exist before starting
        for path in &files {
            if !path.exists() {
                return Err(IrohError::NodeError(format!(
                    "File does not exist: {}",
                    path.display()
                )));
            }
            if !path.is_file() {
                return Err(IrohError::NodeError(format!(
                    "Path is not a file: {}",
                    path.display()
                )));
            }
        }

        let sender_files = self.convert_paths_to_sender_files(files).await?;

        let request = SendFilesRequest {
            files: sender_files,
            profile: SenderProfile {
                name: "Anonymous".to_string(),
                avatar_b64: None,
            },
            config: SenderConfig::default(),
        };

        let bubble = send_files(request)
            .await
            .map_err(|e| IrohError::NodeError(e.to_string()))?;

        // Subscribe to sending progress updates
        let progress_subscriber = Arc::new(SendProgressSubscriber::new(handle.clone()));
        bubble.subscribe(progress_subscriber);

        // Return both the ticket and bubble - bubble must be kept alive!
        let ticket = TicketWrapper::new(bubble.get_ticket(), bubble.get_confirmation());
        Ok((ticket, bubble))
    }

    pub async fn receive_files(
        &self,
        ticket_str: String,
        output_dir: PathBuf,
        handle: Arc<FileTransferHandle>,
    ) -> IrohResult<Collection> {
        // Parse ticket to extract confirmation
        let (ticket, confirmation) = TicketWrapper::parse(&ticket_str)?;

        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            std::fs::create_dir_all(&output_dir).map_err(|e| {
                IrohError::DownloadError(format!("Failed to create output directory: {}", e))
            })?;
        }

        // Create unique subdirectory for this transfer to avoid conflicts
        let receiving_path = output_dir.join(format!(
            "drop_transfer_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ));
        std::fs::create_dir(&receiving_path).map_err(|e| {
            IrohError::DownloadError(format!("Failed to create receiving directory: {}", e))
        })?;

        // Create receiver profile
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

        // Create shared collection for tracking received files
        let collection = Arc::new(std::sync::Mutex::new(Collection::new()));

        // Create progress subscriber with receiving path for file writing
        let progress_subscriber = Arc::new(ReceiveProgressSubscriber::new(
            handle.clone(),
            collection.clone(),
            receiving_path.clone(),
        ));

        // Use ark-core to receive files
        let bubble = receive_files(request).await.map_err(|e| {
            let error_msg = format!("Failed to connect to sender: {}", e);
            IrohError::DownloadError(error_msg)
        })?;

        // Subscribe to progress updates
        bubble.subscribe(progress_subscriber);

        // Start the receive operation
        bubble.start().map_err(|e| {
            let error_msg = format!("Failed to start receiving: {}", e);
            IrohError::DownloadError(error_msg)
        })?;

        // Wait for completion and return the collection with actual file info
        self.wait_for_completion(Arc::new(bubble), collection).await
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
        collection: Arc<std::sync::Mutex<Collection>>,
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

        // Return the collection with actual received file information
        let collection_guard = collection.lock().unwrap();
        let result = collection_guard.clone();

        Ok(result)
    }
}

// Send progress subscriber to track sending progress
struct SendProgressSubscriber {
    handle: Arc<FileTransferHandle>,
}

impl SendProgressSubscriber {
    fn new(handle: Arc<FileTransferHandle>) -> Self {
        Self { handle }
    }
}

impl SendFilesSubscriber for SendProgressSubscriber {
    fn get_id(&self) -> String {
        "send_progress_subscriber".to_string()
    }

    fn log(&self, _message: String) {
        // Log messages can be ignored for now
    }

    fn notify_sending(&self, event: SendFilesSendingEvent) {
        let total = event.sent + event.remaining;
        let file_transfer = FileTransfer {
            name: event.name,
            transferred: event.sent,
            total,
        };
        let _ = self.handle.0.send(vec![file_transfer]);
    }

    fn notify_connecting(&self, _event: SendFilesConnectingEvent) {
        // Connection established with receiver
    }
}

// Receive progress subscriber to track receiving progress and write files
struct ReceiveProgressSubscriber {
    handle: Arc<FileTransferHandle>,
    collection: Arc<std::sync::Mutex<Collection>>,
    receiving_path: PathBuf,
    files: std::sync::RwLock<Vec<dropx_receiver::ReceiveFilesFile>>,
}

impl ReceiveProgressSubscriber {
    fn new(
        handle: Arc<FileTransferHandle>,
        collection: Arc<std::sync::Mutex<Collection>>,
        receiving_path: PathBuf,
    ) -> Self {
        Self {
            handle,
            collection,
            receiving_path,
            files: std::sync::RwLock::new(Vec::new()),
        }
    }
}

impl ReceiveFilesSubscriber for ReceiveProgressSubscriber {
    fn get_id(&self) -> String {
        "receive_progress_subscriber".to_string()
    }

    fn log(&self, _message: String) {
        // Log messages can be ignored for now
    }

    fn notify_receiving(&self, event: ReceiveFilesReceivingEvent) {
        // Find the file for this event
        let files = match self.files.read() {
            Ok(files) => files,
            Err(_) => return,
        };

        let file = match files.iter().find(|f| f.id == event.id) {
            Some(file) => file,
            None => {
                return;
            }
        };

        // Write the received data to the file
        let file_path = self.receiving_path.join(&file.name);

        // Create or append to the file
        match std::fs::File::options()
            .create(true)
            .append(true)
            .open(&file_path)
        {
            Ok(mut file_handle) => {
                use std::io::Write;
                if let Err(_e) = file_handle.write_all(&event.data) {
                    return;
                }
                if let Err(_e) = file_handle.flush() {
                    return;
                }
            }
            Err(_e) => {
                return;
            }
        }

        // Update progress
        let file_transfer = FileTransfer {
            name: file.name.clone(),
            transferred: event.data.len() as u64, // This is incremental data
            total: file.len,
        };
        let _ = self.handle.0.send(vec![file_transfer]);
    }

    fn notify_connecting(&self, event: ReceiveFilesConnectingEvent) {
        // Store file information in collection and files list
        if let Ok(mut collection) = self.collection.lock() {
            for file in &event.files {
                collection.add_file(file.name.clone(), format!("hash_{}", file.len));
            }
        }

        // Store files for later reference
        if let Ok(mut files) = self.files.write() {
            files.extend(event.files.clone());
        }

        // Send initial progress with 0 transferred
        let file_transfers: Vec<FileTransfer> = event
            .files
            .iter()
            .map(|f| FileTransfer {
                name: f.name.clone(),
                transferred: 0,
                total: f.len,
            })
            .collect();
        let _ = self.handle.0.send(file_transfers);
    }
}

// File data adapter to read from filesystem for ark-core
struct FileDataAdapter {
    is_finished: std::sync::atomic::AtomicBool,
    path: PathBuf,
    reader: std::sync::RwLock<Option<std::fs::File>>,
    size: u64,
    bytes_read: std::sync::atomic::AtomicU64,
}

impl FileDataAdapter {
    fn from_path(path: PathBuf) -> IrohResult<Self> {
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

        let current_position = self.bytes_read.fetch_add(size, Ordering::AcqRel);

        if current_position >= self.size {
            self.bytes_read.store(self.size, Ordering::Release);
            self.is_finished.store(true, Ordering::Release);
            return Vec::new();
        }

        let remaining = self.size - current_position;
        let to_read = std::cmp::min(size, remaining) as usize;

        let mut file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(_) => {
                self.is_finished.store(true, Ordering::Release);
                return Vec::new();
            }
        };

        if file.seek(SeekFrom::Start(current_position)).is_err() {
            self.is_finished.store(true, Ordering::Release);
            return Vec::new();
        }

        let mut buffer = vec![0u8; to_read];
        match file.read_exact(&mut buffer) {
            Ok(()) => {
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
