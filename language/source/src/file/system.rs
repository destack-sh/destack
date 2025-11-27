use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::{fs, io};

use cfg_if::cfg_if;

/// Metadata information about a file.
#[derive(Debug, Clone, Copy)]
pub struct FileMetadata {
    /// Whether the file is a regular file.
    pub is_file: bool,
    /// Whether the file is a directory.
    pub is_directory: bool,
    /// Whether the file is a symlink.
    pub is_symlink: bool,
}

impl FileMetadata {
    /// Create a new FileMetadata.
    #[must_use]
    pub const fn new(is_file: bool, is_dir: bool, is_symlink: bool) -> Self {
        Self {
            is_file,
            is_directory: is_dir,
            is_symlink,
        }
    }
}

#[cfg(target_os = "windows")]
impl From<crate::windows::SymlinkMetadata> for FileMetadata {
    fn from(value: crate::windows::SymlinkMetadata) -> Self {
        Self::new(value.is_file, value.is_dir, value.is_symlink)
    }
}

impl From<fs::Metadata> for FileMetadata {
    fn from(metadata: fs::Metadata) -> Self {
        Self::new(metadata.is_file(), metadata.is_dir(), metadata.is_symlink())
    }
}

/// Abstract file system.
pub trait FileSystem: Send + Sync + Debug {
    /// Create a new file system instance.
    fn new() -> Self
    where
        Self: Sized;

    /// Check whether the path points to an existing entry.
    ///
    /// See [std::path::Path::exists].
    fn exists(&self, path: &Path) -> io::Result<bool>;

    /// Get the metadata of a file, following symlinks if present.
    ///
    /// See [std::fs::metadata].
    fn metadata(&self, path: &Path) -> io::Result<FileMetadata>;

    /// Get the target path of a symbolic link.
    ///
    /// See [std::fs::read_link].
    fn resolve_symlink(&self, path: &Path) -> io::Result<PathBuf>;

    /// Get the canonical, absolute form of a path with all intermediate components normalized.
    ///
    /// See [std::fs::canonicalize].
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// Read the contents of a file into a vector of bytes.
    ///
    /// See [std::fs::read].
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Read the entries within a directory.
    ///
    /// See [std::fs::read_dir].
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;

    /// Read the contents of a file into a string.
    ///
    /// See [std::fs::read_to_string].
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// Get the metadata of a file without following symlinks.
    ///
    /// See [std::fs::symlink_metadata].
    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata>;
}

#[inline]
pub fn validate_utf8_string(bytes: Vec<u8>) -> io::Result<String> {
    // `simdutf8` is faster than `std::str::from_utf8` which `fs::read_to_string` uses internally
    if simdutf8::basic::from_utf8(&bytes).is_err() {
        // Same error as `fs::read_to_string` produces (`io::Error::INVALID_UTF8`)
        #[cold]
        fn invalid_utf8_error() -> io::Error {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            )
        }
        return Err(invalid_utf8_error());
    }
    // SAFETY: `simdutf8` has ensured it's a valid UTF-8 string
    Ok(unsafe { String::from_utf8_unchecked(bytes) })
}

/// Physical file system implementation (backed by the current target OS).
#[derive(Debug)]
pub struct PhysicalFileSystem;

impl PhysicalFileSystem {
    pub fn read_to_string(path: &Path) -> io::Result<String> {
        let bytes = std::fs::read(path)?;
        validate_utf8_string(bytes)
    }

    #[inline]
    pub fn metadata(path: &Path) -> io::Result<FileMetadata> {
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                let result = crate::windows::symlink_metadata(path)?;
                if result.is_symlink {
                    return fs::metadata(path).map(FileMetadata::from);
                }
                Ok(result.into())
            } else if #[cfg(target_os = "linux")] {
                use rustix::fs::{AtFlags, CWD, FileType, StatxFlags};
                let statx = rustix::fs::statx(CWD, path, AtFlags::STATX_DONT_SYNC, StatxFlags::TYPE)?;
                let file_type = FileType::from_raw_mode(statx.stx_mode.into());
                Ok(FileMetadata::new(file_type.is_file(), file_type.is_dir(), file_type.is_symlink()))
            } else {
                fs::metadata(path).map(FileMetadata::from)
            }
        }
    }

    #[inline]
    pub fn symlink_metadata(path: &Path) -> io::Result<FileMetadata> {
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                Ok(crate::windows::symlink_metadata(path)?.into())
            } else if #[cfg(target_os = "linux")] {
                use rustix::fs::{AtFlags, CWD, FileType, StatxFlags};
                let statx = rustix::fs::statx(CWD, path, AtFlags::SYMLINK_NOFOLLOW, StatxFlags::TYPE)?;
                let file_type = FileType::from_raw_mode(statx.stx_mode.into());
                Ok(FileMetadata::new(file_type.is_file(), file_type.is_dir(), file_type.is_symlink()))
            } else {
                fs::symlink_metadata(path).map(FileMetadata::from)
            }
        }
    }

    #[inline]
    pub fn read_link(path: &Path) -> io::Result<PathBuf> {
        let path = fs::read_link(path)?;
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                crate::windows::strip_windows_prefix(path)
            } else {
                Ok(path)
            }
        }
    }

    #[inline]
    pub fn canonicalize(path: &Path) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }
}

impl FileSystem for PhysicalFileSystem {
    fn new() -> Self {
        Self
    }

    #[tracing::instrument(name = "fs.exists", level = "trace", skip(self))]
    fn exists(&self, path: &Path) -> io::Result<bool> {
        tracing::trace!(?path, "fs.exists");
        match self.metadata(path) {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(err),
        }
    }

    #[tracing::instrument(name = "fs.metadata", level = "trace", skip(self))]
    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        tracing::trace!(?path, "fs.metadata");
        Self::metadata(path)
    }

    #[tracing::instrument(name = "fs.resolve_symlink", level = "trace", skip(self))]
    fn resolve_symlink(&self, path: &Path) -> io::Result<PathBuf> {
        tracing::trace!(?path, "fs.resolve_symlink");
        Self::read_link(path)
    }

    #[tracing::instrument(name = "fs.canonicalize", level = "trace", skip(self))]
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        tracing::trace!(?path, "fs.canonicalize");
        Self::canonicalize(path)
    }

    #[tracing::instrument(name = "fs.read", level = "trace", skip(self))]
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        tracing::trace!(?path, "fs.read");
        fs::read(path)
    }

    #[tracing::instrument(name = "fs.read_dir", level = "trace", skip(self))]
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        tracing::trace!(?path, "fs.read_dir");
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            entries.push(entry?.path());
        }
        Ok(entries)
    }

    #[tracing::instrument(name = "fs.read_to_string", level = "trace", skip(self))]
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        tracing::trace!(?path, "fs.read_to_string");
        let bytes = self.read(path)?;
        validate_utf8_string(bytes)
    }

    #[tracing::instrument(name = "fs.symlink_metadata", level = "trace", skip(self))]
    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        tracing::trace!(?path, "fs.symlink_metadata");
        Self::symlink_metadata(path)
    }
}
