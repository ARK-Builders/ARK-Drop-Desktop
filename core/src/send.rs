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
        self.try_send(event);
        Box::pin(std::future::ready(()))
    }

    fn try_send(&self, event: provider::Event) {
        match event {
            provider::Event::ClientConnected { connection_id } => {
                println!("[SEND] Client Connecyed");
                // self.sender
                //     .blocking_send(SendEvent {
                //         message: format!("{} client connected", connection_id),
                //     })
                //     .unwrap();
            }
            provider::Event::TransferBlobCompleted {
                connection_id,
                hash,
                index,
                size,
                ..
            } => {
                println!("[SEND] TransferBlobCompleted");

                // let _ = self
                //     .sender
                //     .blocking_send(SendEvent {
                //         message: format!(
                //             "{} transfer blob completed {} {} {}",
                //             connection_id, hash, index, size
                //         ),
                //     })
                //     .unwrap();
            }
            provider::Event::TransferCompleted {
                connection_id,
                stats,
                ..
            } => {
                println!("[SEND] TransferCompleted");

                // let _ = self
                //     .sender
                //     .blocking_send(SendEvent {
                //         message: format!("{} transfer completed {:?}", connection_id, stats),
                //     })
                //     .unwrap();
            }
            provider::Event::TransferAborted { connection_id, .. } => {
                println!("[SEND] TransferAborted");

                // let _ = self
                //     .sender
                //     .blocking_send(SendEvent {
                //         message: format!("{} transfer aborted", connection_id),
                //     })
                //     .unwrap();
            }
            _ => {}
        };
    }
}
