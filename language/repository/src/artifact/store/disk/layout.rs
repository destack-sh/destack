use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[cfg(not(target_os = "wasi"))]
use std::fs::File;

use destack_artifact::ArtifactError;
use destack_core::{Blob, BlobId};

use crate::DestackLayout;

/// Artifact store lock file name.
#[cfg(not(target_os = "wasi"))]
const LOCK_FILE: &str = "store.lock";
/// Artifact segment reference file suffix.
const REFERENCE_SUFFIX: &str = ".ref";
/// Segment reference directory below one artifact partition.
const SEGMENT_DIRECTORY: &str = "segments";

/// Filesystem namespace containing artifact segment references.
#[derive(Debug)]
pub(super) struct Layout {
    /// Root shared by artifact partitions for this repository.
    root: PathBuf,
}

/// Held artifact publication lock.
#[cfg(not(target_os = "wasi"))]
#[derive(Debug)]
pub(super) struct Lock {
    /// The native locked file.
    _file: File,
}

impl Layout {
    /// Resolve one artifact layout.
    pub(super) fn new(repository_root: &Path, layout: &DestackLayout) -> Self {
        let root = layout.artifact_directory(repository_root);

        Self { root }
    }

    /// Acquire the exclusive artifact publication lock.
    #[cfg(not(target_os = "wasi"))]
    pub(super) fn lock(&self) -> Result<Lock, ArtifactError> {
        let path = self.root.join(LOCK_FILE);

        Lock::acquire(path)
    }

    /// Return every referenced segment Blob in stable order.
    pub(super) fn segments(&self, partition: &str) -> Result<Vec<Blob>, ArtifactError> {
        let directory = self.root.join(partition).join(SEGMENT_DIRECTORY);
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };

        let mut blobs = Vec::new();
        for entry in entries {
            let entry = entry.map_err(ArtifactError::from)?;
            let file_type = entry.file_type().map_err(ArtifactError::from)?;
            if file_type.is_file() {
                blobs.push(Self::parse_reference(&entry.file_name())?);
            }
        }
        blobs.sort_unstable();

        Ok(blobs)
    }

    /// Publish one segment reachability reference.
    pub(super) fn publish(&self, partition: &str, blob: Blob) -> Result<(), ArtifactError> {
        let path = self.reference(partition, blob);
        let Some(parent) = path.parent() else {
            return Err(ArtifactError::store("artifact reference has no parent"));
        };
        fs::create_dir_all(parent).map_err(ArtifactError::from)?;

        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(file) => file.sync_all().map_err(ArtifactError::from),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    /// Remove one segment reachability reference.
    pub(super) fn remove(&self, partition: &str, blob: Blob) -> Result<(), ArtifactError> {
        let path = self.reference(partition, blob);
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    /// Return the reachability reference path for one segment Blob.
    fn reference(&self, partition: &str, blob: Blob) -> PathBuf {
        let name = format!("{}-{}{REFERENCE_SUFFIX}", blob.id, blob.byte_len);

        self.root.join(partition).join(SEGMENT_DIRECTORY).join(name)
    }

    /// Decode one segment descriptor from its reference file name.
    fn parse_reference(name: &OsStr) -> Result<Blob, ArtifactError> {
        let Some(name) = name.to_str() else {
            return Err(ArtifactError::store("artifact reference name is not UTF-8"));
        };
        let Some(name) = name.strip_suffix(REFERENCE_SUFFIX) else {
            return Err(ArtifactError::store(
                "artifact reference has an invalid extension",
            ));
        };
        let Some((id, byte_len)) = name.split_once('-') else {
            return Err(ArtifactError::store("artifact reference is malformed"));
        };
        let id = id
            .parse::<BlobId>()
            .map_err(|error| ArtifactError::store(error.to_string()))?;
        let byte_len = byte_len
            .parse::<u64>()
            .map_err(|error| ArtifactError::store(error.to_string()))?;

        Ok(Blob::new(id, byte_len))
    }
}

#[cfg(not(target_os = "wasi"))]
impl Lock {
    /// Acquire one artifact publication lock.
    fn acquire(path: PathBuf) -> Result<Self, ArtifactError> {
        let Some(parent) = path.parent() else {
            return Err(ArtifactError::store("artifact lock path has no parent"));
        };
        fs::create_dir_all(parent).map_err(ArtifactError::from)?;

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(ArtifactError::from)?;
        file.lock().map_err(ArtifactError::from)?;

        Ok(Self { _file: file })
    }
}
