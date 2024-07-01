use iroh_blobs::BlobFormat;
use thiserror::Error;

use crate::UniffiCustomTypeConverter;

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

uniffi::custom_type!(IrohError, String);

impl UniffiCustomTypeConverter for IrohError {
    type Builtin = String;

    fn into_custom(val: Self::Builtin) -> uniffi::Result<Self> {
        Ok(match val.as_str() {
            _ => IrohError::Anyhow(anyhow::Error::msg(val)),
        })
    }

    fn from_custom(obj: Self) -> Self::Builtin {
        match obj {
            IrohError::Anyhow(err) => err.to_string(),
            IrohError::InvalidTicket(err) => err.to_string(),
            IrohError::InvalidMetadata => "invalid metadata".to_string(),
            IrohError::UnsupportedFormat(format) => format!("unsupported format: {}", format),
        }
    }
}
