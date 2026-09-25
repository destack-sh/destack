use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, io};

use super::{FileMetadata, FileSystem, PhysicalFileSystem};

/// Counter for unique temporary root names.
static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Temporary physical file system scoped to a temp root directory.
#[derive(Debug)]
pub struct TemporaryPhysicalFileSystem {
    /// The root directory for the temporary file system.
    root: PathBuf,
    /// Whether to keep the directory after drop.
    keep_on_drop: bool,
}

impl TemporaryPhysicalFileSystem {
    /// Create a temporary file system with a prefix.
    pub fn new_with_prefix(prefix: &str) -> Self {
        Self::try_new_with_prefix(prefix).unwrap_or_else(|error| {
            panic!("failed to create temporary filesystem: {error}");
        })
    }

    /// Try to create a temporary file system with a prefix.
    pub fn try_new_with_prefix(prefix: &str) -> io::Result<Self> {
        // derive a unique suffix for the temp directory
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| io::Error::other(format!("system time error: {error}")))?
            .as_nanos();
        let name = format!("tspp_{prefix}_{nanos}_{counter}");

        // create the temp root
        let root = env::temp_dir().join(name);
        PhysicalFileSystem.create_dir_all(&root)?;

        // store one canonical root so all generated paths use one stable form
        let root = PhysicalFileSystem.canonicalize(&root).unwrap_or(root);

        // keep temp roots for debugging when requested
        let keep_on_drop = should_keep_temp_root();

        Ok(Self { root, keep_on_drop })
    }

    /// Return the temp root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a path relative to the temp root.
    pub fn path_for(&self, path: impl AsRef<Path>) -> PathBuf {
        self.resolve_path(path.as_ref())
            .unwrap_or_else(|error| panic!("invalid temp path: {error}"))
    }

    /// Write a UTF-8 string under the temp root.
    pub fn write_text(&self, path: impl AsRef<Path>, contents: &str) -> io::Result<PathBuf> {
        // resolve the target path
        let path = self.resolve_path(path.as_ref())?;

        // ensure parent directories exist
        if let Some(parent) = path.parent() {
            PhysicalFileSystem.create_dir_all(parent)?;
        }

        // write the file contents
        PhysicalFileSystem.write(&path, contents.as_bytes())?;
        Ok(path)
    }

    /// Write a UTF-8 string under the temp root or error out.
    pub fn write_text_or_error(&self, path: impl AsRef<Path>, contents: &str) -> PathBuf {
        self.write_text(path, contents).unwrap_or_else(|error| {
            panic!("failed to write temp file: {error}");
        })
    }

    /// Write bytes under the temp root.
    pub fn write_bytes(&self, path: impl AsRef<Path>, contents: &[u8]) -> io::Result<PathBuf> {
        // resolve the target path
        let path = self.resolve_path(path.as_ref())?;

        // ensure parent directories exist
        if let Some(parent) = path.parent() {
            PhysicalFileSystem.create_dir_all(parent)?;
        }

        // write the file contents
        PhysicalFileSystem.write(&path, contents)?;
        Ok(path)
    }

    /// Write bytes under the temp root or error out.
    pub fn write_bytes_or_error(&self, path: impl AsRef<Path>, contents: &[u8]) -> PathBuf {
        self.write_bytes(path, contents).unwrap_or_else(|error| {
            panic!("failed to write temp file: {error}");
        })
    }

    /// Rename a file within the temp root.
    pub fn rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
        // resolve the source and destination paths
        let from = self.resolve_path(from.as_ref())?;
        let to = self.resolve_path(to.as_ref())?;

        // ensure destination directories exist
        if let Some(parent) = to.parent() {
            PhysicalFileSystem.create_dir_all(parent)?;
        }

        std::fs::rename(&from, &to)
    }

    /// Rename a file within the temp root or error out.
    pub fn rename_or_error(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) {
        self.rename(from, to).unwrap_or_else(|error| {
            panic!("failed to rename temp file: {error}");
        })
    }

    /// Decide whether the temp root should be kept after drop.
    pub fn with_keep_on_drop(mut self, keep_on_drop: bool) -> Self {
        self.keep_on_drop = keep_on_drop;
        self
    }

    /// Resolve a path to an absolute path under the temp root.
    fn resolve_path(&self, path: &Path) -> io::Result<PathBuf> {
        // allow absolute paths scoped to the temp root
        if path.is_absolute() {
            if path.starts_with(&self.root) {
                return Ok(path.to_path_buf());
            }
            return Err(path_scope_error(path));
        }

        // deny parent traversal from relative paths
        for component in path.components() {
            if matches!(component, Component::ParentDir) {
                return Err(path_scope_error(path));
            }
        }

        Ok(self.root.join(path))
    }
}

impl Drop for TemporaryPhysicalFileSystem {
    fn drop(&mut self) {
        // keep the root when requested
        if self.keep_on_drop {
            return;
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl FileSystem for TemporaryPhysicalFileSystem {
    fn new() -> Self {
        Self::new_with_prefix("temp")
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.exists(&path)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.metadata(&path)
    }

    fn resolve_symlink(&self, path: &Path) -> io::Result<PathBuf> {
        let path = self.resolve_path(path)?;
        let resolved = PhysicalFileSystem.resolve_symlink(&path)?;
        self.resolve_path(&resolved)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let path = self.resolve_path(path)?;
        let resolved = PhysicalFileSystem.canonicalize(&path)?;
        if !resolved.starts_with(&self.root) {
            return Err(path_scope_error(&resolved));
        }
        Ok(resolved)
    }

    fn open(&self, path: &Path) -> io::Result<Box<dyn io::Read + Send>> {
        let path = self.resolve_path(path)?;

        PhysicalFileSystem.open(&path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.read(&path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.read_dir(&path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.read_to_string(&path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.symlink_metadata(&path)
    }

    fn write(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.write(&path, content)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.create_dir(&path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.create_dir_all(&path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.remove_file(&path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        let path = self.resolve_path(path)?;
        PhysicalFileSystem.remove_dir(&path)
    }
}

/// Decide whether to keep temporary roots based on the environment.
fn should_keep_temp_root() -> bool {
    let Ok(value) = env::var("TSPP_TEST_KEEP_FS") else {
        return false;
    };
    matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES")
}

/// Build a scoped path error for paths outside the root.
fn path_scope_error(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("path outside temporary root: {}", path.display()),
    )
}
