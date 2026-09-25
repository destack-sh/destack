use std::io;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{File, FileId, FilePatch, FileSystem, FileType, Patch, Span, Uri, apply_file_patch};

/// One source file mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Edit {
    /// Replace or create one text file.
    SetText {
        /// Repository relative path.
        path: PathBuf,
        /// Full text content.
        text: String,
    },
    /// Apply text replacements to one tracked text file.
    EditText {
        /// Repository relative path.
        path: PathBuf,
        /// Text replacements.
        patches: Vec<TextPatch>,
    },
    /// Replace or create one binary file.
    SetBytes {
        /// Repository relative path.
        path: PathBuf,
        /// Full binary content.
        bytes: Vec<u8>,
    },
    /// Remove one file.
    Remove {
        /// Repository relative path.
        path: PathBuf,
    },
    /// Move one file.
    Move {
        /// Source repository relative path.
        from: PathBuf,
        /// Destination repository relative path.
        to: PathBuf,
    },
}

impl Edit {
    /// Apply this edit beneath one file system root.
    pub fn apply(self, root: &Path, file_system: &dyn FileSystem) -> io::Result<()> {
        match self {
            Self::SetText { path, text } => {
                let path = Self::resolve(root, &path)?;

                Self::write_text(&path, &text, file_system)
            }
            Self::EditText { path, patches } => {
                let physical_path = Self::resolve(root, &path)?;

                Self::patch_text(&physical_path, &path, patches, file_system)
            }
            Self::SetBytes { path, bytes } => {
                let path = Self::resolve(root, &path)?;

                Self::write_bytes(&path, &bytes, file_system)
            }
            Self::Remove { path } => {
                let path = Self::resolve(root, &path)?;
                file_system.remove_path(&path)?;

                Ok(())
            }
            Self::Move { from, to } => {
                let from = Self::resolve(root, &from)?;
                let to = Self::resolve(root, &to)?;
                let bytes = file_system.read(&from)?;

                Self::write_bytes(&to, &bytes, file_system)?;
                file_system.remove_path(&from)?;

                Ok(())
            }
        }
    }

    /// Return the single file path affected by this edit.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::SetText { path, .. }
            | Self::EditText { path, .. }
            | Self::SetBytes { path, .. }
            | Self::Remove { path } => Some(path.as_path()),
            Self::Move { .. } => None,
        }
    }

    /// Return whether this edit removes its target file.
    pub fn is_remove(&self) -> bool {
        matches!(self, Self::Remove { .. })
    }

    /// Return full text content when this edit sets text directly.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::SetText { text, .. } => Some(text),
            _ => None,
        }
    }

    /// Write one text file.
    fn write_text(path: &Path, text: &str, file_system: &dyn FileSystem) -> io::Result<()> {
        Self::create_parent_directory(path, file_system)?;

        file_system.write(path, text.as_bytes())
    }

    /// Write one binary file.
    fn write_bytes(path: &Path, bytes: &[u8], file_system: &dyn FileSystem) -> io::Result<()> {
        Self::create_parent_directory(path, file_system)?;

        file_system.write(path, bytes)
    }

    /// Apply text patches to one text file.
    fn patch_text(
        path: &Path,
        logical_path: &Path,
        patches: Vec<TextPatch>,
        file_system: &dyn FileSystem,
    ) -> io::Result<()> {
        let text = file_system.read_to_string(path)?;
        let file_id = FileId::from_logical_path(logical_path);
        let name_and_type = path
            .file_name()
            .and_then(|name| name.to_str())
            .zip(FileType::from_path(path));
        let Some((name, file_type)) = name_and_type else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("text edit path must name a UTF-8 file: {}", path.display()),
            ));
        };

        // build one indexed source file for validated patch application
        let file = File::from_text(
            file_id,
            name.to_string(),
            Uri::from_path(path),
            Some(path.to_path_buf()),
            file_type,
            text,
        )
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        // lower path level text patches into source patches
        let patches = patches
            .into_iter()
            .map(|patch| {
                Patch::replace(
                    Span::new(file_id, patch.range.start, patch.range.end),
                    patch.text,
                )
            })
            .collect();
        let patch = FilePatch::with_patches(file_id, patches);
        let text = apply_file_patch(&file, &patch)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

        Self::write_text(path, &text, file_system)
    }

    /// Resolve one repository-relative edit path beneath its file system root.
    fn resolve(root: &Path, path: &Path) -> io::Result<PathBuf> {
        let has_name = path
            .components()
            .any(|component| matches!(component, Component::Normal(_)));
        let is_relative = path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
        if !has_name || !is_relative {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "source edit path must name a relative file: {}",
                    path.display()
                ),
            ));
        }

        Ok(root.join(path))
    }

    /// Create the parent directory for one file path.
    fn create_parent_directory(path: &Path, file_system: &dyn FileSystem) -> io::Result<()> {
        let Some(parent) = path.parent() else {
            return Ok(());
        };

        file_system.create_dir_all(parent)
    }
}

/// One byte range in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ByteRange {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

/// One source text replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TextPatch {
    /// Replaced byte range.
    pub range: ByteRange,
    /// Replacement text.
    pub text: String,
}
