use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_core::{Blob, BlobId, BlobMemory};

#[cfg(not(target_os = "wasi"))]
use memmap2::MmapOptions;
#[cfg(target_os = "wasi")]
use parking_lot::Mutex;

use super::{BlobStore, BlobStoreError};

/// Process-local sequence for temporary Blob files.
static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
/// Process-local serialization for WASI Blob publication.
#[cfg(target_os = "wasi")]
static PUBLICATION: Mutex<()> = Mutex::new(());

/// BlobStore backed by immutable files.
#[derive(Debug, Clone)]
pub struct DiskBlobStore {
    /// Root containing content-addressed Blob files.
    root: PathBuf,
}

impl DiskBlobStore {
    /// Open one disk BlobStore rooted at the given path.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Return the content-addressed path for one BlobId.
    fn path(&self, id: BlobId) -> PathBuf {
        let name = id.to_string();
        let shard = format!("{:02x}", id.bytes()[0]);

        self.root.join(shard).join(format!("{name}.blob"))
    }

    /// Create one unpublished temporary Blob file.
    fn temporary(&self) -> Result<(PathBuf, File), BlobStoreError> {
        fs::create_dir_all(&self.root)?;

        loop {
            let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let process = process::id();
            let path = self.root.join(format!(".{process}-{sequence}.tmp"));
            let file = OpenOptions::new().write(true).create_new(true).open(&path);

            match file {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
        }
    }

    /// Remove one temporary Blob file if present.
    fn discard(temporary: &Path) -> Result<(), BlobStoreError> {
        match fs::remove_file(temporary) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    /// Publish one staged Blob without replacing existing bytes.
    fn publish(&self, temporary: &Path, blob: Blob) -> Result<(), BlobStoreError> {
        let path = self.path(blob.id);
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("Blob path has no parent"))?;
        fs::create_dir_all(parent)?;

        // publish without replacement on native file systems
        #[cfg(not(target_os = "wasi"))]
        return match fs::hard_link(temporary, &path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                self.open(blob).map(|_memory| ())
            }
            Err(error) => Err(error.into()),
        };

        // serialize the WASI existence check and atomic rename within this process
        #[cfg(target_os = "wasi")]
        {
            let _publication = PUBLICATION.lock();
            match fs::metadata(&path) {
                Ok(_metadata) => self.open(blob).map(|_memory| ()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    fs::rename(temporary, path)?;

                    Ok(())
                }
                Err(error) => Err(error.into()),
            }
        }
    }
}

impl BlobStore for DiskBlobStore {
    fn put(&self, input: &mut dyn Read) -> Result<Blob, BlobStoreError> {
        let (temporary_path, mut temporary) = self.temporary()?;
        let mut input = HashingReader::new(input);
        let byte_len = match io::copy(&mut input, &mut temporary) {
            Ok(byte_len) => byte_len,
            Err(error) => {
                drop(temporary);
                Self::discard(&temporary_path)?;

                return Err(error.into());
            }
        };

        // durably finish the staged bytes
        let synced = temporary.sync_all();
        drop(temporary);
        if let Err(error) = synced {
            Self::discard(&temporary_path)?;

            return Err(error.into());
        }

        // identify the complete staged bytes
        let blob = Blob::new(BlobId::new(input.finish()), byte_len);

        // publish the exact Blob and always remove its staging file
        let published = self.publish(&temporary_path, blob);
        Self::discard(&temporary_path)?;
        published?;

        Ok(blob)
    }

    fn open(&self, blob: Blob) -> Result<Arc<BlobMemory>, BlobStoreError> {
        let path = self.path(blob.id);
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(BlobStoreError::Missing { blob });
            }
            Err(error) => return Err(error.into()),
        };
        let actual = file.metadata()?.len();
        if actual != blob.byte_len {
            return Err(BlobStoreError::Length { blob, actual });
        }

        // retain the exact bytes using this platform's filesystem memory
        #[cfg(not(target_os = "wasi"))]
        let memory = if blob.byte_len == 0 {
            BlobMemory::from_bytes(Vec::new())
        } else {
            // safety: Blob files are immutable after atomic publication
            let memory = unsafe { MmapOptions::new().map(&file)? };

            BlobMemory::from_shared(Arc::new(memory))
        };
        #[cfg(target_os = "wasi")]
        let memory = BlobMemory::from_bytes(fs::read(path)?);

        // verify the complete retained bytes
        if memory.blob() != blob {
            return Err(BlobStoreError::Corrupt {
                expected: blob,
                actual: memory.blob(),
            });
        }

        Ok(Arc::new(memory))
    }

    fn contains(&self, blob: Blob) -> Result<bool, BlobStoreError> {
        match self.open(blob) {
            Ok(_memory) => Ok(true),
            Err(BlobStoreError::Missing { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn read(
        &self,
        blob: Blob,
        range: Range<u64>,
        output: &mut dyn Write,
    ) -> Result<(), BlobStoreError> {
        if range.start > range.end || range.end > blob.byte_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Blob range {}..{} exceeds {} bytes",
                    range.start, range.end, blob.byte_len
                ),
            )
            .into());
        }

        // open and verify the complete stored Blob
        let path = self.path(blob.id);
        let mut input = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(BlobStoreError::Missing { blob });
            }
            Err(error) => return Err(error.into()),
        };
        let actual = hash(&mut input)?;
        if actual.byte_len != blob.byte_len {
            return Err(BlobStoreError::Length {
                blob,
                actual: actual.byte_len,
            });
        }
        if actual.id != blob.id {
            return Err(BlobStoreError::Corrupt {
                expected: blob,
                actual,
            });
        }

        // stream the selected range
        input.seek(SeekFrom::Start(range.start))?;
        let mut input = input.take(range.end - range.start);
        let written = io::copy(&mut input, output)?;
        if written != range.end - range.start {
            return Err(BlobStoreError::Length {
                blob,
                actual: written,
            });
        }

        Ok(())
    }
}

/// Reader that hashes every consumed byte.
struct HashingReader<'a> {
    /// Source byte stream.
    input: &'a mut dyn Read,
    /// Incremental BLAKE3 state.
    hasher: blake3::Hasher,
}

impl<'a> HashingReader<'a> {
    /// Wrap one byte stream.
    fn new(input: &'a mut dyn Read) -> Self {
        Self {
            input,
            hasher: blake3::Hasher::new(),
        }
    }

    /// Finish the complete BLAKE3 digest.
    fn finish(&self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }
}

impl Read for HashingReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let byte_len = self.input.read(buffer)?;
        self.hasher.update(&buffer[..byte_len]);

        Ok(byte_len)
    }
}

/// Hash one complete byte stream.
fn hash(input: &mut dyn Read) -> Result<Blob, BlobStoreError> {
    let mut input = HashingReader::new(input);
    let byte_len = io::copy(&mut input, &mut io::sink())?;

    Ok(Blob::new(BlobId::new(input.finish()), byte_len))
}
