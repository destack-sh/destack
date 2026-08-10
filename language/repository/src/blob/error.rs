use std::{error, fmt, io};

use destack_core::Blob;

/// Failure while storing or opening immutable Blobs.
#[derive(Debug)]
pub enum BlobStoreError {
    /// One Blob was not present.
    Missing {
        /// The missing Blob.
        blob: Blob,
    },
    /// Stored bytes did not match their expected description.
    Corrupt {
        /// The expected Blob.
        expected: Blob,
        /// The observed Blob.
        actual: Blob,
    },
    /// Stored bytes have the wrong length.
    Length {
        /// The expected Blob.
        blob: Blob,
        /// The observed byte length.
        actual: u64,
    },
    /// The backing I/O operation failed.
    Io(Box<io::Error>),
}

impl fmt::Display for BlobStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { blob } => write!(formatter, "missing Blob {}", blob.id),
            Self::Corrupt { expected, actual } => write!(
                formatter,
                "corrupt Blob {}: expected {} bytes, observed Blob {} with {} bytes",
                expected.id, expected.byte_len, actual.id, actual.byte_len
            ),
            Self::Length { blob, actual } => write!(
                formatter,
                "corrupt Blob {}: expected {} bytes, observed {actual} bytes",
                blob.id, blob.byte_len
            ),
            Self::Io(error) => write!(formatter, "Blob store I/O failed: {error}"),
        }
    }
}

impl error::Error for BlobStoreError {}

impl From<io::Error> for BlobStoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(Box::new(error))
    }
}
