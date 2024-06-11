use iroh_blobs::BlobFormat;
use thiserror::Error;

pub type IrohResult<T> = Result<T, IrohError>;

#[derive(Error, Debug)]
pub enum IrohError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error("invalid ticket")]
    InvalidTicket(#[from] iroh_base::ticket::Error),

    #[error("invalid metadata")]
    InvalidMetadata,

    #[error("unsupported format: {0}")]
    UnsupportedFormat(BlobFormat),
}
