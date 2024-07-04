use std::{error::Error, sync::mpsc::SendError};

use iroh_blobs::BlobFormat;
use thiserror::Error;

use crate::{FileTransfer, UniffiCustomTypeConverter};

pub type IrohResult<T> = Result<T, IrohError>;

pub type AnyhowError = anyhow::Error;
pub type IrohBaseError = iroh_base::ticket::Error;
pub type SendFileError = SendError<Vec<FileTransfer>>;

#[derive(Error, Debug, uniffi::Error)]
pub enum IrohError {
    #[error(transparent)]
    Anyhow(#[from] AnyhowError),

    #[error("invalid ticket")]
    InvalidTicket(#[from] iroh_base::ticket::Error),

    #[error("invalid metadata")]
    InvalidMetadata,

    #[error("unsupported format: {0}")]
    UnsupportedFormat(BlobFormat),

    #[error("send error")]
    SendError(#[from] SendFileError),
}

uniffi::custom_type!(BlobFormat, String);

impl UniffiCustomTypeConverter for BlobFormat {
    type Builtin = String;

    fn into_custom(val: Self::Builtin) -> uniffi::Result<Self> {
        match val.as_str() {
            "hashseq" => Ok(BlobFormat::HashSeq),
            "raw" => Ok(BlobFormat::Raw),
            _ => Err(anyhow::anyhow!("unsupported format: {}", val).into()),
        }
    }

    fn from_custom(obj: Self) -> Self::Builtin {
        match obj {
            BlobFormat::HashSeq => "hashseq".to_string(),
            BlobFormat::Raw => "raw".to_string(),
        }
    }
}

uniffi::custom_type!(AnyhowError, String);

impl UniffiCustomTypeConverter for AnyhowError {
    type Builtin = String;

    fn into_custom(val: Self::Builtin) -> uniffi::Result<Self> {
        Ok(anyhow::Error::msg(val))
    }

    fn from_custom(obj: Self) -> Self::Builtin {
        obj.to_string()
    }
}

uniffi::custom_type!(IrohBaseError, String);

impl UniffiCustomTypeConverter for IrohBaseError {
    type Builtin = String;

    fn into_custom(val: Self::Builtin) -> uniffi::Result<Self> {
        Ok(iroh_base::ticket::Error::Kind { expected: "None" })
    }

    fn from_custom(obj: Self) -> Self::Builtin {
        obj.to_string()
    }
}

uniffi::custom_type!(SendFileError, String);

impl UniffiCustomTypeConverter for SendFileError {
    type Builtin = String;

    fn into_custom(val: Self::Builtin) -> uniffi::Result<Self> {
        Err(anyhow::anyhow!("send error").into())
    }

    fn from_custom(obj: Self) -> Self::Builtin {
        obj.to_string()
    }
}
