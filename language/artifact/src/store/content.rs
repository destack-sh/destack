use destack_serde::Schema;
use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;

use destack_source::{Content, ContentId};
use serde::{Deserialize, Serialize};

use crate::{BlobStore, BlobStoreError, MAX_BLOB_BYTES, RepositoryStoreLayout};

/// Store of canonical content blobs.
#[derive(Debug)]
pub struct ContentStore<'a> {
    /// The blob store backing this content store.
    store: &'a dyn BlobStore,
    /// The repository store layout.
    layout: RepositoryStoreLayout,
}

/// One serialized content blob.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
struct ContentBlob {
    /// The exact content identity.
    id: ContentId,
    /// The canonical content payload.
    content: Content,
}

/// Errors that can occur while reading or writing content blobs.
#[derive(Debug)]
pub enum ContentStoreError {
    /// The content bytes are malformed or internally inconsistent.
    Corrupt(&'static str),
    /// The content id did not match the decoded content.
    Identity {
        /// The requested content id.
        expected: ContentId,
        /// The decoded or computed content id.
        actual: ContentId,
    },
    /// The content failed to encode or decode.
    Codec(Box<destack_serde::Error>),
    /// The content blob exceeded the configured size limit.
    Size {
        /// The configured size limit.
        limit: u64,
        /// The actual encoded byte length.
        actual: u64,
    },
    /// The store already has different bytes for the same content id.
    Conflict {
        /// The conflicting exact content id.
        content: ContentId,
    },
    /// The blob store failed to read or write.
    Blob(Box<BlobStoreError>),
}

impl<'a> ContentStore<'a> {
    /// Create one content store.
    pub fn new(store: &'a dyn BlobStore, layout: &RepositoryStoreLayout) -> Self {
        Self {
            store,
            layout: layout.clone(),
        }
    }

    /// Load one exact content blob.
    pub fn load(&self, expected: ContentId) -> Result<Option<Content>, ContentStoreError> {
        // inspect stored content
        let path = self.content_path(expected);
        let Some(byte_len) = self.store.byte_len(&path)? else {
            return Ok(None);
        };
        if byte_len > MAX_BLOB_BYTES {
            return Err(ContentStoreError::Size {
                limit: MAX_BLOB_BYTES,
                actual: byte_len,
            });
        }

        // read and validate bytes
        let Some(bytes) = self.store.read(&path)? else {
            return Ok(None);
        };
        let blob = deserialize_blob(&bytes)?;
        validate_blob(expected, blob)
    }

    /// Store one exact content blob.
    pub fn store(&self, content: &Content) -> Result<ContentId, ContentStoreError> {
        // encode content blob
        let content_id = ContentId::for_content(content);
        let blob = ContentBlob {
            id: content_id,
            content: content.clone(),
        };
        let bytes = serialize_blob(&blob)?;

        // publish immutable content without serializing independent writers
        self.write_content_bytes(content_id, &bytes)?;

        Ok(content_id)
    }

    /// Retain only reachable content blobs.
    pub fn retain_reachable(
        &self,
        reachable: &HashSet<ContentId>,
    ) -> Result<(), ContentStoreError> {
        // build retained path set
        let root = self.layout.content_root();
        let retained_paths = reachable
            .iter()
            .map(|content| self.content_path(*content))
            .collect::<HashSet<_>>();

        // remove unreachable entries
        self.with_write_lock(|| {
            for path in self.store.entries(&root)? {
                if !retained_paths.contains(&path) {
                    self.store.remove(&path)?;
                }
            }

            Ok(())
        })
    }

    /// Run one write operation under the persistent store lock.
    fn with_write_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ContentStoreError>,
    ) -> Result<T, ContentStoreError> {
        // acquire store lock
        let lock_path = self.layout.store_lock_path();
        let mut operation = Some(operation);
        let mut output = None;

        // run operation once
        self.store.with_exclusive_lock(&lock_path, &mut || {
            let Some(operation) = operation.take() else {
                unreachable!("blob store lock should run the operation exactly once");
            };
            let result = operation();
            output = Some(result);
        })?;

        // return operation result
        match output {
            Some(output) => output,
            None => unreachable!("blob store lock should run the operation before returning"),
        }
    }

    /// Write one exact content blob.
    fn write_content_bytes(
        &self,
        content: ContentId,
        bytes: &[u8],
    ) -> Result<(), ContentStoreError> {
        // accept existing identical bytes
        let path = self.content_path(content);
        if let Some(existing_bytes) = self.store.read(&path)? {
            if existing_bytes != bytes {
                return Err(ContentStoreError::Conflict { content });
            }

            return Ok(());
        }

        // write new bytes
        match self.store.write_once(&path, bytes) {
            Ok(()) => {}
            Err(BlobStoreError::AlreadyExists) => {
                let Some(existing_bytes) = self.store.read(&path)? else {
                    return Err(ContentStoreError::Corrupt(
                        "blob disappeared after write conflict",
                    ));
                };
                if existing_bytes != bytes {
                    return Err(ContentStoreError::Conflict { content });
                }
            }
            Err(error) => return Err(error.into()),
        }

        Ok(())
    }

    /// Return the stored content path for one exact content id.
    fn content_path(&self, content: ContentId) -> PathBuf {
        let content_token = format!("{:032x}", content.0);
        let shard = &content_token[0..2];

        self.layout
            .content_root()
            .join(shard)
            .join(format!("{content_token}.bin"))
    }
}

/// Serialize one content blob.
fn serialize_blob(blob: &ContentBlob) -> Result<Vec<u8>, ContentStoreError> {
    // encode blob
    let bytes =
        destack_serde::to_vec(blob).map_err(|error| ContentStoreError::Codec(Box::new(error)))?;
    let byte_len = bytes.len() as u64;
    if byte_len > MAX_BLOB_BYTES {
        return Err(ContentStoreError::Size {
            limit: MAX_BLOB_BYTES,
            actual: byte_len,
        });
    }

    Ok(bytes)
}

/// Deserialize one content blob.
fn deserialize_blob(bytes: &[u8]) -> Result<ContentBlob, ContentStoreError> {
    destack_serde::from_slice(bytes).map_err(|error| ContentStoreError::Codec(Box::new(error)))
}

/// Validate one decoded content blob.
fn validate_blob(
    expected: ContentId,
    blob: ContentBlob,
) -> Result<Option<Content>, ContentStoreError> {
    // validate stored identity
    if blob.id != expected {
        return Err(ContentStoreError::Identity {
            expected,
            actual: blob.id,
        });
    }

    // validate content identity
    let actual = ContentId::for_content(&blob.content);
    if actual != expected {
        return Err(ContentStoreError::Identity { expected, actual });
    }

    Ok(Some(blob.content))
}

impl fmt::Display for ContentStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Corrupt(message) => write!(formatter, "corrupt content blob: {message}"),
            Self::Identity { expected, actual } => {
                write!(
                    formatter,
                    "unexpected content identity, expected {expected}, found {actual}"
                )
            }
            Self::Codec(error) => write!(formatter, "content store codec error: {error}"),
            Self::Size { limit, actual } => {
                write!(
                    formatter,
                    "content blob exceeded size limit, limit {limit}, actual {actual}"
                )
            }
            Self::Conflict { content } => {
                write!(formatter, "conflicting content bytes for {content}")
            }
            Self::Blob(error) => write!(formatter, "content blob store error: {error}"),
        }
    }
}

impl std::error::Error for ContentStoreError {}

impl From<BlobStoreError> for ContentStoreError {
    fn from(error: BlobStoreError) -> Self {
        Self::Blob(Box::new(error))
    }
}
