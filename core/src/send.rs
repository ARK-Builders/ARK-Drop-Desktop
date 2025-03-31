use std::sync::Arc;

use futures_lite::future::Boxed;
use iroh_blobs::provider::{self, CustomEventSender};
use serde::Serialize;
use tokio::sync::mpsc::Sender;

use crate::metadata::FileTransfer;

#[derive(Debug)]
pub enum Event {
    Files(Vec<FileTransfer>),
    Send(SendEvent),
}

#[derive(Debug, Clone, Serialize)]
pub struct SendEvent {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct SendStatus {
    sender: Arc<Sender<Event>>,
}

impl SendStatus {
    pub fn new(sender: Arc<Sender<Event>>) -> Self {
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
                        .send(Event::Send(SendEvent {
                            message: format!("{} client connected", connection_id),
                        }))
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
                        .send(Event::Send(SendEvent {
                            message: format!(
                                "{} transfer blob completed {} {} {}",
                                connection_id, hash, index, size
                            ),
                        }))
                        .await
                }
                provider::Event::TransferCompleted {
                    connection_id,
                    stats,
                    ..
                } => {
                    sender
                        .send(Event::Send(SendEvent {
                            message: format!("{} transfer completed {:?}", connection_id, stats),
                        }))
                        .await
                }
                provider::Event::TransferAborted { connection_id, .. } => {
                    sender
                        .send(Event::Send(SendEvent {
                            message: format!("{} transfer aborted", connection_id),
                        }))
                        .await
                }
                _ => Ok(()), // For unhandled events, return Ok
            };

            if let Err(e) = result {
                eprintln!("Failed to send event: {}", e);
            }
        }) as Boxed<()>
    }

    fn try_send(&self, _event: provider::Event) {}
}
