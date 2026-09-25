use std::fmt::Debug;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use std::{fs, io};

use cfg_if::cfg_if;

#[cfg(target_os = "linux")]
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};
#[cfg(windows)]
use windows::core::PCWSTR;

/// Next process-local atomic write identity.
static NEXT_WRITE_ID: AtomicU64 = AtomicU64::new(1);

/// Metadata information about a file.
#[derive(Debug, Clone, Copy)]
pub struct FileMetadata {
    /// Whether the file is a regular file.
    pub is_file: bool,
    /// Whether the file is a directory.
    pub is_directory: bool,
    /// Whether the file is a symlink.
    pub is_symlink: bool,
    /// The file size in bytes.
    pub size_bytes: u64,
    /// The last modified timestamp when available.
    pub modified_at: Option<SystemTime>,
}

impl FileMetadata {
    /// Create a new FileMetadata.
    #[must_use]
    pub const fn new(
        is_file: bool,
        is_dir: bool,
        is_symlink: bool,
        size_bytes: u64,
        modified_at: Option<SystemTime>,
    ) -> Self {
        Self {
            is_file,
            is_directory: is_dir,
            is_symlink,
            size_bytes,
            modified_at,
        }
    }
}

#[cfg(target_os = "windows")]
impl From<crate::windows::SymlinkMetadata> for FileMetadata {
    fn from(value: crate::windows::SymlinkMetadata) -> Self {
        Self::new(
            value.is_file,
            value.is_dir,
            value.is_symlink,
            value.size_bytes,
            value.modified_at,
        )
    }
}

impl From<fs::Metadata> for FileMetadata {
    fn from(metadata: fs::Metadata) -> Self {
        Self::new(
            metadata.is_file(),
            metadata.is_dir(),
            metadata.is_symlink(),
            metadata.len(),
            metadata.modified().ok(),
        )
    }
}

#[cfg(target_os = "linux")]
fn system_time_from_unix(secs: i64, nanos: u32) -> Option<SystemTime> {
    // map negative timestamps to None
    if secs < 0 {
        return None;
    }

    // build system time from unix timestamp
    let duration = Duration::new(secs as u64, nanos);
    SystemTime::UNIX_EPOCH.checked_add(duration)
}

/// Abstract file system.
pub trait FileSystem: Send + Sync + Debug {
    /// Create a new file system instance.
    fn new() -> Self
    where
        Self: Sized;

    // read operations

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

    /// Open a file as a byte stream.
    fn open(&self, path: &Path) -> io::Result<Box<dyn io::Read + Send>>;

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

    // mutation operations

    /// Write exact bytes atomically, creating the file when absent.
    fn write(&self, path: &Path, content: &[u8]) -> io::Result<()>;

    /// Create a directory at the given path.
    ///
    /// See [std::fs::create_dir].
    fn create_dir(&self, path: &Path) -> io::Result<()>;

    /// Create a directory and all of its parent directories if they don't exist.
    ///
    /// See [std::fs::create_dir_all].
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Remove a file at the given path.
    ///
    /// See [std::fs::remove_file].
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Remove an empty directory at the given path.
    ///
    /// See [std::fs::remove_dir].
    fn remove_dir(&self, path: &Path) -> io::Result<()>;

    /// Remove a file or directory recursively if it exists.
    fn remove_path(&self, path: &Path) -> io::Result<bool> {
        // fetch metadata without following symlinks
        let metadata = match self.symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(err) => return Err(err),
        };

        // remove symlinks and files directly
        if metadata.is_symlink || metadata.is_file {
            self.remove_file(path)?;
            return Ok(true);
        }

        // remove directories recursively
        if metadata.is_directory {
            self.remove_dir_all(path)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Remove a directory and all of its entries.
    fn remove_dir_all(&self, dir: &Path) -> io::Result<()> {
        // collect directory entries
        let entries = self.read_dir(dir)?;

        // remove child entries
        for entry in entries {
            self.remove_path(&entry)?;
        }

        // remove the directory itself
        self.remove_dir(dir)
    }
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
                let statx = rustix::fs::statx(CWD, path, AtFlags::STATX_DONT_SYNC, StatxFlags::TYPE | StatxFlags::SIZE | StatxFlags::MTIME)?;
                let file_type = FileType::from_raw_mode(statx.stx_mode.into());
                let modified_at = system_time_from_unix(statx.stx_mtime.tv_sec, statx.stx_mtime.tv_nsec as u32);
                Ok(FileMetadata::new(
                    file_type.is_file(),
                    file_type.is_dir(),
                    file_type.is_symlink(),
                    statx.stx_size,
                    modified_at,
                ))
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
                let statx = rustix::fs::statx(CWD, path, AtFlags::SYMLINK_NOFOLLOW, StatxFlags::TYPE | StatxFlags::SIZE | StatxFlags::MTIME)?;
                let file_type = FileType::from_raw_mode(statx.stx_mode.into());
                let modified_at = system_time_from_unix(statx.stx_mtime.tv_sec, statx.stx_mtime.tv_nsec as u32);
                Ok(FileMetadata::new(
                    file_type.is_file(),
                    file_type.is_dir(),
                    file_type.is_symlink(),
                    statx.stx_size,
                    modified_at,
                ))
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

    /// Return the write destination reached by one final symlink.
    fn destination(path: &Path) -> io::Result<PathBuf> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => fs::canonicalize(path),
            Ok(_) => Ok(path.to_path_buf()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path.to_path_buf()),
            Err(error) => Err(error),
        }
    }

    /// Return one unique temporary path beside its destination.
    fn temporary_path(path: &Path) -> PathBuf {
        let identifier = NEXT_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();
        let temporary = format!(".{name}.tspp-{process}-{identifier}.tmp");

        path.with_file_name(temporary)
    }

    /// Preserve existing file permissions when the destination exists.
    fn permissions(path: &Path) -> io::Result<Option<fs::Permissions>> {
        match fs::metadata(path) {
            Ok(metadata) => Ok(Some(metadata.permissions())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Remove one unpublished replacement while retaining every failure.
    fn discard(path: &Path, operation: io::Error) -> io::Error {
        match fs::remove_file(path) {
            Ok(()) => operation,
            Err(cleanup) => io::Error::other(format!(
                "{operation}; removing replacement {} also failed: {cleanup}",
                path.display()
            )),
        }
    }

    /// Atomically publish one replacement file on Unix hosts.
    #[cfg(not(windows))]
    fn publish(source: &Path, destination: &Path) -> io::Result<()> {
        fs::rename(source, destination)
    }

    /// Atomically publish one replacement file on Windows hosts.
    #[cfg(windows)]
    fn publish(source: &Path, destination: &Path) -> io::Result<()> {
        let mut source = source.as_os_str().encode_wide().collect::<Vec<_>>();
        source.push(0);
        let mut destination = destination.as_os_str().encode_wide().collect::<Vec<_>>();
        destination.push(0);
        let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;

        unsafe { MoveFileExW(PCWSTR(source.as_ptr()), PCWSTR(destination.as_ptr()), flags) }
            .map_err(io::Error::other)
    }
}

impl FileSystem for PhysicalFileSystem {
    fn new() -> Self {
        Self
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        match self.metadata(path) {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(err),
        }
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        Self::metadata(path)
    }

    fn resolve_symlink(&self, path: &Path) -> io::Result<PathBuf> {
        Self::read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        Self::canonicalize(path)
    }

    fn open(&self, path: &Path) -> io::Result<Box<dyn io::Read + Send>> {
        Ok(Box::new(fs::File::open(path)?))
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            entries.push(entry?.path());
        }
        Ok(entries)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read(path)?;
        validate_utf8_string(bytes)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        Self::symlink_metadata(path)
    }

    fn write(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        let destination = Self::destination(path)?;
        let permissions = Self::permissions(&destination)?;

        // allocate one unused replacement path beside the destination
        let (temporary, mut file) = loop {
            let temporary = Self::temporary_path(&destination);
            let file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary);
            match file {
                Ok(file) => break (temporary, file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        };

        // remove an incomplete replacement after one failed write
        if let Err(error) = file.write_all(content) {
            drop(file);

            return Err(Self::discard(&temporary, error));
        }

        // retain the destination's existing access permissions
        if let Some(permissions) = permissions
            && let Err(error) = file.set_permissions(permissions)
        {
            drop(file);

            return Err(Self::discard(&temporary, error));
        }
        drop(file);

        // publish the complete file through one filesystem replacement
        let result = Self::publish(&temporary, &destination);
        if let Err(error) = result {
            return Err(Self::discard(&temporary, error));
        }

        Ok(())
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(path)
    }
}
