use std::fmt;
use std::io::{Read, Write};
use std::ops::Range;
use std::sync::Arc;

use destack_core::{Blob, BlobMemory};

use super::BlobStoreError;

/// Immutable content-addressed byte storage.
pub trait BlobStore: fmt::Debug + Send + Sync {
    /// Store one exact byte stream.
    fn put(&self, input: &mut dyn Read) -> Result<Blob, BlobStoreError>;

    /// Create one incremental Blob writer.
    fn writer(&self) -> Result<Box<dyn BlobWriter + '_>, BlobStoreError>;

    /// Open one complete Blob as retained immutable memory.
    fn open(&self, blob: Blob) -> Result<Arc<BlobMemory>, BlobStoreError>;

    /// Return whether one exact Blob is present.
    fn contains(&self, blob: Blob) -> Result<bool, BlobStoreError>;

    /// Write one exact Blob range.
    fn read(
        &self,
        blob: Blob,
        range: Range<u64>,
        output: &mut dyn Write,
    ) -> Result<(), BlobStoreError>;
}

/// Incremental writer for one unpublished Blob.
pub trait BlobWriter: fmt::Debug + Write + Send {
    /// Publish the complete Blob.
    fn commit(self: Box<Self>) -> Result<Blob, BlobStoreError>;
}
