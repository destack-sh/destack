use std::io::Write;
use std::ops::Range;

use serde::{Deserialize, Serialize};
use tspp_core::{Blob, BlobStore, BlobStoreError};
use tspp_rpc::{Code, Request, RequestStream, Response, ResponseSender, Status};
use tspp_serde::Reflect;

use crate::DaemonError;
use crate::service::BYTE_STREAM_CHUNK_BYTE_LEN;

/// RPC operations on shared Blobs.
#[tspp_rpc::service(name = "tspp.blob.Blob")]
pub trait BlobService {
    // =============================================================================
    // Transfer
    // =============================================================================

    /// Store one streamed Blob.
    #[rpc(
        name = "Put",
        request_stream(Vec<u8>),
        idempotency = "idempotent"
    )]
    fn put(request: ()) -> Blob;

    /// Read one streamed Blob range.
    #[rpc(
        name = "Read",
        response_stream(Vec<u8>),
        idempotency = "no_side_effects"
    )]
    fn read(request: ReadBlobRequest) -> ();

    // =============================================================================
    // Query
    // =============================================================================

    /// Return whether one exact Blob is present.
    #[rpc(name = "Contains", idempotency = "no_side_effects")]
    fn contains(request: Blob) -> bool;
}

/// Request to read one Blob range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadBlobRequest {
    /// Blob to read.
    pub blob: Blob,
    /// First byte to read.
    pub offset: u64,
    /// Number of bytes to read, or every remaining byte.
    pub byte_len: Option<u64>,
}

impl ReadBlobRequest {
    /// Resolve the requested byte range for this host.
    pub(crate) fn range(self) -> Result<Range<usize>, Status> {
        let end = match self.byte_len {
            Some(byte_len) => self
                .offset
                .checked_add(byte_len)
                .ok_or_else(|| Status::new(Code::OutOfRange, "Blob read range exceeds u64"))?,
            None => self.blob.byte_len,
        };

        // require one range within the exact Blob
        if self.offset > end || end > self.blob.byte_len {
            return Err(Status::new(
                Code::OutOfRange,
                format!(
                    "Blob read range {}..{end} exceeds {} bytes",
                    self.offset, self.blob.byte_len
                ),
            ));
        }

        // convert the validated range to this host's address width
        let start = usize::try_from(self.offset)
            .map_err(|_| Status::new(Code::OutOfRange, "Blob read offset exceeds usize"))?;
        let end = usize::try_from(end)
            .map_err(|_| Status::new(Code::OutOfRange, "Blob read end exceeds usize"))?;

        Ok(start..end)
    }
}

impl BlobService for BlobStore {
    /// Store one streamed Blob.
    async fn put(
        &self,
        _request: Request<()>,
        mut requests: RequestStream<Vec<u8>>,
    ) -> Result<Response<Blob>, Status> {
        let mut writer = self.writer();

        // write each bounded input chunk directly into unpublished storage
        while let Some(bytes) = requests
            .receive()
            .await
            .map_err(|error| error.into_status())?
        {
            if bytes.len() > BYTE_STREAM_CHUNK_BYTE_LEN {
                return Err(Status::new(
                    Code::InvalidArgument,
                    format!(
                        "Blob stream item exceeds {BYTE_STREAM_CHUNK_BYTE_LEN} bytes: {}",
                        bytes.len()
                    ),
                ));
            }
            writer
                .write_all(&bytes)
                .map_err(BlobStoreError::from)
                .map_err(DaemonError::from)?;
        }

        // atomically publish the completed Blob
        let blob = writer.commit().map_err(DaemonError::from)?;

        Ok(Response::new(blob))
    }

    /// Read one streamed Blob range.
    async fn read(
        &self,
        request: Request<ReadBlobRequest>,
        mut responses: ResponseSender<Vec<u8>>,
    ) -> Result<Response<()>, Status> {
        let range = request.value.range()?;
        let memory = self.open(request.value.blob).map_err(DaemonError::from)?;

        // send the exact range as bounded output chunks
        for bytes in memory.bytes()[range].chunks(BYTE_STREAM_CHUNK_BYTE_LEN) {
            responses
                .send(&bytes.to_vec())
                .await
                .map_err(|error| error.into_status())?;
        }

        Ok(Response::new(()))
    }

    /// Return whether one exact Blob is present.
    async fn contains(&self, request: Request<Blob>) -> Result<Response<bool>, Status> {
        let is_present = BlobStore::contains(self, request.value).map_err(DaemonError::from)?;

        Ok(Response::new(is_present))
    }
}
