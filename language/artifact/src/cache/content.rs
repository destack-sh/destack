use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;

use destack_source::{Content, ContentId};
use serde::{Deserialize, Serialize};

use crate::{CACHE_BLOB_LIMIT_BYTES, CacheStore, CacheStoreError, RepositoryCacheLayout};

/// Cache of canonical content blobs.
#[derive(Debug)]
pub struct ContentCache<'a> {
    /// The cache store backing this content cache.
    store: &'a dyn CacheStore,
    /// The repository cache layout.
    layout: RepositoryCacheLayout,
}

/// One serialized content blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContentBlob {
    /// The exact content identity.
    id: ContentId,
    /// The canonical content payload.
    content: Content,
}

/// Errors that can occur while reading or writing cached content.
#[derive(Debug)]
pub enum ContentCacheError {
    /// The content bytes are malformed or internally inconsistent.
    Corrupt(&'static str),
    /// The content id did not match the decoded content.
    Identity {
        expected: ContentId,
        actual: ContentId,
    },
    /// The content failed to encode or decode.
    Codec(Box<postcard::Error>),
    /// The content blob exceeded the configured size limit.
    Size { limit: u64, actual: u64 },
    /// The cache already has different bytes for the same content id.
    Conflict { content: ContentId },
    /// The content cache failed to read or write.
    Cache(Box<CacheStoreError>),
}

impl<'a> ContentCache<'a> {
    /// Create one content cache.
    pub fn new(store: &'a dyn CacheStore, layout: &RepositoryCacheLayout) -> Self {
        Self {
            store,
            layout: layout.clone(),
        }
    }

    /// Load one exact content blob.
    pub fn load(&self, expected: ContentId) -> Result<Option<Content>, ContentCacheError> {
        // inspect cached content
        let path = self.content_path(expected);
        let Some(byte_len) = self.store.byte_len(&path)? else {
            return Ok(None);
        };
        if byte_len > CACHE_BLOB_LIMIT_BYTES {
            return Err(ContentCacheError::Size {
                limit: CACHE_BLOB_LIMIT_BYTES,
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
    pub fn store(&self, content: &Content) -> Result<ContentId, ContentCacheError> {
        // encode content blob
        let content_id = ContentId::for_content(content);
        let blob = ContentBlob {
            id: content_id,
            content: content.clone(),
        };
        let bytes = serialize_blob(&blob)?;

        // write under cache lock
        self.with_write_lock(|| {
            self.write_content_bytes(content_id, &bytes)?;

            Ok(())
        })?;

        Ok(content_id)
    }

    /// Retain only reachable content blobs.
    pub fn retain_reachable(
        &self,
        reachable: &HashSet<ContentId>,
    ) -> Result<(), ContentCacheError> {
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

    /// Run one write operation under the persistent cache lock.
    fn with_write_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ContentCacheError>,
    ) -> Result<T, ContentCacheError> {
        // acquire cache lock
        let lock_path = self.layout.cache_lock_path();
        let mut operation = Some(operation);
        let mut output = None;

        // run operation once
        self.store.with_exclusive_lock(&lock_path, &mut || {
            let Some(operation) = operation.take() else {
                panic!("cache lock should run exactly once");
            };
            let result = operation();
            output = Some(result);
        })?;

        // return operation result
        match output {
            Some(output) => output,
            None => panic!("cache lock should produce one value"),
        }
    }

    /// Write one exact content blob.
    fn write_content_bytes(
        &self,
        content: ContentId,
        bytes: &[u8],
    ) -> Result<(), ContentCacheError> {
        // accept existing identical bytes
        let path = self.content_path(content);
        if let Some(existing_bytes) = self.store.read(&path)? {
            if existing_bytes != bytes {
                return Err(ContentCacheError::Conflict { content });
            }

            return Ok(());
        }

        // write new bytes
        match self.store.write_once(&path, bytes) {
            Ok(()) => {}
            Err(CacheStoreError::AlreadyExists) => {
                let Some(existing_bytes) = self.store.read(&path)? else {
                    return Err(ContentCacheError::Corrupt(
                        "cache entry disappeared after write conflict",
                    ));
                };
                if existing_bytes != bytes {
                    return Err(ContentCacheError::Conflict { content });
                }
            }
            Err(error) => return Err(error.into()),
        }

        Ok(())
    }

    /// Return the cached content path for one exact content id.
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
fn serialize_blob(blob: &ContentBlob) -> Result<Vec<u8>, ContentCacheError> {
    // encode blob
    let bytes =
        postcard::to_allocvec(blob).map_err(|error| ContentCacheError::Codec(Box::new(error)))?;
    let byte_len = bytes.len() as u64;
    if byte_len > CACHE_BLOB_LIMIT_BYTES {
        return Err(ContentCacheError::Size {
            limit: CACHE_BLOB_LIMIT_BYTES,
            actual: byte_len,
        });
    }

    Ok(bytes)
}

/// Deserialize one content blob.
fn deserialize_blob(bytes: &[u8]) -> Result<ContentBlob, ContentCacheError> {
    postcard::from_bytes(bytes).map_err(|error| ContentCacheError::Codec(Box::new(error)))
}

/// Validate one decoded content blob.
fn validate_blob(
    expected: ContentId,
    blob: ContentBlob,
) -> Result<Option<Content>, ContentCacheError> {
    // validate stored identity
    if blob.id != expected {
        return Err(ContentCacheError::Identity {
            expected,
            actual: blob.id,
        });
    }

    // validate content identity
    let actual = ContentId::for_content(&blob.content);
    if actual != expected {
        return Err(ContentCacheError::Identity { expected, actual });
    }

    Ok(Some(blob.content))
}

impl fmt::Display for ContentCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Corrupt(message) => write!(formatter, "corrupt content cache blob: {message}"),
            Self::Identity { expected, actual } => {
                write!(
                    formatter,
                    "unexpected content identity, expected {expected}, found {actual}"
                )
            }
            Self::Codec(error) => write!(formatter, "content cache codec error: {error}"),
            Self::Size { limit, actual } => {
                write!(
                    formatter,
                    "content cache blob exceeded size limit, limit {limit}, actual {actual}"
                )
            }
            Self::Conflict { content } => {
                write!(formatter, "conflicting content cache bytes for {content}")
            }
            Self::Cache(error) => write!(formatter, "content cache error: {error}"),
        }
    }
}

impl std::error::Error for ContentCacheError {}

impl From<CacheStoreError> for ContentCacheError {
    fn from(error: CacheStoreError) -> Self {
        Self::Cache(Box::new(error))
    }
}
