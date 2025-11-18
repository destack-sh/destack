use std::path::{Path, PathBuf};
use std::{fs, io};

use cfg_if::cfg_if;

use crate::FileMetadata;

/// File system.
pub trait FileSystem: Send + Sync {
    fn new() -> Self;

    /// See [std::fs::read]
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// See [std::fs::read_to_string]
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// See [std::fs::metadata]
    fn metadata(&self, path: &Path) -> io::Result<FileMetadata>;

    /// See [std::fs::symlink_metadata]
    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata>;

    /// Returns the resolution of a symbolic link.
    fn read_link(&self, path: &Path) -> io::Result<PathBuf>;

    /// Returns the canonical, absolute form of a path with all intermediate components normalized.
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
}

/// Physical file system implementation (backed by the current target OS).
#[derive(Debug)]
pub struct PhysicalFileSystem;

impl PhysicalFileSystem {
    #[inline]
    pub fn validate_string(bytes: Vec<u8>) -> io::Result<String> {
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

    pub fn read_to_string(path: &Path) -> io::Result<String> {
        let bytes = std::fs::read(path)?;
        Self::validate_string(bytes)
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

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read(path)?;
        Self::validate_string(bytes)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        Self::metadata(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        Self::symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        Self::read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        Self::canonicalize(path)
    }
}
