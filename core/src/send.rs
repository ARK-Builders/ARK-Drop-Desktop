use std::sync::{mpsc::Sender, Arc};

use futures_lite::future::Boxed;
use iroh_blobs::provider::{self, CustomEventSender};

#[derive(Debug, Clone)]
pub struct SendEvent {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct SendStatus {
    sender: Arc<Sender<SendEvent>>,
}

impl SendStatus {
    pub fn new(sender: Arc<Sender<SendEvent>>) -> Self {
        Self { sender }
    }
}

impl CustomEventSender for SendStatus {
    fn send(&self, event: iroh_blobs::provider::Event) -> Boxed<()> {
        self.try_send(event);
        Box::pin(std::future::ready(()))
    }

    fn try_send(&self, event: provider::Event) {
        match event {
            provider::Event::ClientConnected { connection_id } => {
                let _ = self.sender.send(SendEvent {
                    message: format!("{} client connected", connection_id),
                });
            }
            provider::Event::TransferBlobCompleted {
                connection_id,
                hash,
                index,
                size,
                ..
            } => {
                let _ = self.sender.send(SendEvent {
                    message: format!(
                        "{} transfer blob completed {} {} {}",
                        connection_id, hash, index, size
                    ),
                });
            }
            provider::Event::TransferCompleted {
                connection_id,
                stats,
                ..
            } => {
                let _ = self.sender.send(SendEvent {
                    message: format!("{} transfer completed {:?}", connection_id, stats),
                });
            }
            provider::Event::TransferAborted { connection_id, .. } => {
                let _ = self.sender.send(SendEvent {
                    message: format!("{} transfer aborted", connection_id),
                });
            }
            _ => {}
        };
    }
}
