use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::ArtifactVersion;

use super::{
    ARTIFACT_BLOB_HEADER_LENGTH_BYTES, ARTIFACT_BLOB_MAGIC, ArtifactStoreError, MAX_BLOB_BYTES,
};

/// Common header for one serialized artifact payload blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ArtifactPayloadHeader {
    /// The magic prefix used to identify artifact payload blobs.
    magic: [u8; 4],
    /// The exact artifact version.
    version: ArtifactVersion,
}

impl ArtifactPayloadHeader {
    /// Create a new artifact payload header.
    fn new(version: ArtifactVersion) -> Self {
        Self {
            magic: ARTIFACT_BLOB_MAGIC,
            version,
        }
    }

    /// Split one serialized blob into the decoded header and payload bytes.
    fn split_from_bytes(
        bytes: &[u8],
    ) -> Result<(ArtifactPayloadHeader, &[u8]), ArtifactStoreError> {
        let (header, payload_offset) = Self::decode_prefixed(bytes)?;
        let payload_bytes = &bytes[payload_offset..];

        Ok((header, payload_bytes))
    }

    /// Decode one serialized payload header from the leading bytes.
    fn decode_prefixed(bytes: &[u8]) -> Result<(ArtifactPayloadHeader, usize), ArtifactStoreError> {
        if bytes.len() < ARTIFACT_BLOB_HEADER_LENGTH_BYTES {
            return Err(ArtifactStoreError::Corrupt(
                "missing artifact payload header length prefix",
            ));
        }

        let length_bytes: [u8; ARTIFACT_BLOB_HEADER_LENGTH_BYTES] = bytes[0..4]
            .try_into()
            .map_err(|_| ArtifactStoreError::Corrupt("invalid artifact payload header length"))?;
        let header_length = u32::from_le_bytes(length_bytes) as usize;
        let payload_offset = ARTIFACT_BLOB_HEADER_LENGTH_BYTES + header_length;
        if bytes.len() < payload_offset {
            return Err(ArtifactStoreError::Corrupt(
                "truncated artifact payload header bytes",
            ));
        }

        let header_bytes = &bytes[ARTIFACT_BLOB_HEADER_LENGTH_BYTES..payload_offset];
        let header =
            deserialize_payload_with_limit::<ArtifactPayloadHeader>(header_bytes, MAX_BLOB_BYTES)?;

        Ok((header, payload_offset))
    }
}

/// One versioned artifact payload blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactPayloadBlob<T> {
    /// The exact artifact version.
    version: ArtifactVersion,
    /// The serialized payload.
    pub payload: T,
}

impl<T> ArtifactPayloadBlob<T> {
    /// Create one artifact payload blob.
    pub fn new(version: ArtifactVersion, payload: T) -> Self {
        Self { version, payload }
    }

    /// Return the exact artifact version.
    pub fn version(&self) -> ArtifactVersion {
        self.version
    }
}

impl<T> ArtifactPayloadBlob<T>
where
    T: Serialize,
{
    /// Serialize this artifact payload blob to bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, ArtifactStoreError> {
        self.serialize_with_limit(MAX_BLOB_BYTES)
    }

    /// Serialize this artifact payload blob to bytes with one explicit size limit.
    pub(crate) fn serialize_with_limit(&self, limit: u64) -> Result<Vec<u8>, ArtifactStoreError> {
        let header = ArtifactPayloadHeader::new(self.version);
        let header_bytes = serialize_payload_with_limit(&header, limit)?;
        let header_length =
            u32::try_from(header_bytes.len()).map_err(|_| ArtifactStoreError::Size {
                limit,
                actual: header_bytes.len() as u64,
            })?;
        let payload_bytes = serialize_payload_with_limit(&self.payload, limit)?;
        let total_bytes = ARTIFACT_BLOB_HEADER_LENGTH_BYTES as u64
            + header_bytes.len() as u64
            + payload_bytes.len() as u64;
        if total_bytes > limit {
            return Err(ArtifactStoreError::Size {
                limit,
                actual: total_bytes,
            });
        }

        let mut bytes = Vec::with_capacity(total_bytes as usize);
        bytes.extend_from_slice(&header_length.to_le_bytes());
        bytes.extend_from_slice(&header_bytes);
        bytes.extend_from_slice(&payload_bytes);

        Ok(bytes)
    }
}

impl<T> ArtifactPayloadBlob<T>
where
    T: DeserializeOwned,
{
    /// Deserialize one artifact payload blob from bytes.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, ArtifactStoreError> {
        let (header, payload_bytes) = ArtifactPayloadHeader::split_from_bytes(bytes)?;
        if header.magic != ARTIFACT_BLOB_MAGIC {
            return Err(ArtifactStoreError::Corrupt(
                "invalid artifact payload magic",
            ));
        }

        let payload = deserialize_payload_with_limit::<T>(payload_bytes, MAX_BLOB_BYTES)?;

        Ok(Self {
            version: header.version,
            payload,
        })
    }
}

/// Serialize one payload with a size limit.
fn serialize_payload_with_limit<T: Serialize>(
    payload: &T,
    limit: u64,
) -> Result<Vec<u8>, ArtifactStoreError> {
    let bytes = destack_serde::to_vec(payload)
        .map_err(|error| ArtifactStoreError::Codec(Box::new(error)))?;
    let actual = bytes.len() as u64;
    if actual > limit {
        return Err(ArtifactStoreError::Size { limit, actual });
    }

    Ok(bytes)
}

/// Deserialize one payload with a size limit.
fn deserialize_payload_with_limit<T: DeserializeOwned>(
    bytes: &[u8],
    limit: u64,
) -> Result<T, ArtifactStoreError> {
    let actual = bytes.len() as u64;
    if actual > limit {
        return Err(ArtifactStoreError::Size { limit, actual });
    }

    destack_serde::from_slice(bytes).map_err(|error| ArtifactStoreError::Codec(Box::new(error)))
}
