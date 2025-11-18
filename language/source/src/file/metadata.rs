use std::fs;

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

    /// Whether the file is a regular file.
    #[must_use]
    pub const fn is_file(self) -> bool {
        self.is_file
    }

    /// Whether the file is a directory.
    #[must_use]
    pub const fn is_dir(self) -> bool {
        self.is_directory
    }

    /// Whether the file is a symlink.
    #[must_use]
    pub const fn is_symlink(self) -> bool {
        self.is_symlink
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
