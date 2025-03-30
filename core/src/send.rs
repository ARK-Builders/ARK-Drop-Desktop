use std::sync::Arc;

use futures_lite::future::Boxed;
use iroh_blobs::provider::{self, CustomEventSender};
use tokio::sync::mpsc::Sender;

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
        let sender = self.sender.clone();
        Box::pin(async move {
            let result = match event {
                provider::Event::ClientConnected { connection_id } => {
                    sender
                        .send(SendEvent {
                            message: format!("{} client connected", connection_id),
                        })
                        .await
                }
                provider::Event::TransferBlobCompleted {
                    connection_id,
                    hash,
                    index,
                    size,
                    ..
                } => {
                    sender
                        .send(SendEvent {
                            message: format!(
                                "{} transfer blob completed {} {} {}",
                                connection_id, hash, index, size
                            ),
                        })
                        .await
                }
                provider::Event::TransferCompleted {
                    connection_id,
                    stats,
                    ..
                } => {
                    sender
                        .send(SendEvent {
                            message: format!("{} transfer completed {:?}", connection_id, stats),
                        })
                        .await
                }
                provider::Event::TransferAborted { connection_id, .. } => {
                    sender
                        .send(SendEvent {
                            message: format!("{} transfer aborted", connection_id),
                        })
                        .await
                }
                _ => Ok(()), // For unhandled events, return Ok
            };

            if let Err(e) = result {
                eprintln!("Failed to send event: {}", e);
            }
        }) as Boxed<()>
    }

    fn try_send(&self, event: provider::Event) {}
}
