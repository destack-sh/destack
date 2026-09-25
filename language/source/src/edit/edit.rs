use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{File, FileId, Span};

/// Error produced while applying source patches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchApplyError {
    /// The patched file cannot be loaded.
    MissingFile {
        /// The missing file.
        file: FileId,
    },
    /// One patch is attached to a different file.
    FileMismatch {
        /// The file being patched.
        file: FileId,
        /// The file carried by the patch span.
        patch_file: FileId,
    },
    /// Patches overlap after sorting.
    OverlappingPatches {
        /// The patched file.
        file: FileId,
    },
    /// One patch span is outside the file.
    OutsideFile {
        /// The patched file.
        file: FileId,
        /// The patch start byte offset.
        start: u32,
        /// The patch end byte offset.
        end: u32,
        /// The file length in bytes.
        len: usize,
    },
    /// One patch span does not land on UTF-8 boundaries.
    Boundary {
        /// The patched file.
        file: FileId,
        /// The patch start byte offset.
        start: u32,
        /// The patch end byte offset.
        end: u32,
    },
}

impl Display for PatchApplyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFile { file } => {
                write!(formatter, "missing file for source patch {file:?}")
            }
            Self::FileMismatch { file, patch_file } => {
                write!(
                    formatter,
                    "source patch file {patch_file:?} does not match {file:?}"
                )
            }
            Self::OverlappingPatches { file } => {
                write!(formatter, "source patches overlap in file {file:?}")
            }
            Self::OutsideFile {
                file,
                start,
                end,
                len,
            } => write!(
                formatter,
                "source patch span {start}..{end} is outside file {file:?} with length {len}"
            ),
            Self::Boundary { file, start, end } => write!(
                formatter,
                "source patch span {start}..{end} is not on UTF-8 boundaries in file {file:?}"
            ),
        }
    }
}

impl Error for PatchApplyError {}

/// A single patch: replace a span with new text.
///
/// This is the atomic unit of source modification.
/// An empty `new_text` represents deletion; an empty span represents insertion.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Patch {
    /// The span to replace.
    pub span: Span,
    /// The replacement text (empty for deletion).
    pub new_text: String,
}

impl Patch {
    /// Return the stable order key for this patch.
    pub fn order_key(&self) -> (u64, u32, u32, &str) {
        (
            self.span.file.0,
            self.span.start,
            self.span.end,
            &self.new_text,
        )
    }

    /// Create a replacement patch.
    pub fn replace(span: Span, new_text: impl Into<String>) -> Self {
        Self {
            span,
            new_text: new_text.into(),
        }
    }

    /// Create a deletion patch.
    pub fn delete(span: Span) -> Self {
        Self {
            span,
            new_text: String::new(),
        }
    }

    /// Create an insertion patch at a position.
    pub fn insert(file: FileId, position: u32, text: impl Into<String>) -> Self {
        Self {
            span: Span::new(file, position, position),
            new_text: text.into(),
        }
    }

    /// Whether this patch is a pure insertion (zero-width span).
    #[inline]
    pub fn is_insert(&self) -> bool {
        self.span.is_empty()
    }

    /// Whether this patch is a pure deletion (empty replacement).
    #[inline]
    pub fn is_delete(&self) -> bool {
        self.new_text.is_empty() && !self.span.is_empty()
    }

    /// Get the file this patch applies to.
    #[inline]
    pub fn file(&self) -> FileId {
        self.span.file
    }
}

/// Patches for a single file.
///
/// Groups multiple patches together for efficient application.
/// Patches should be non-overlapping and are typically sorted by position.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FilePatch {
    /// The file to patch.
    pub file: FileId,
    /// The patches to apply (should be non-overlapping).
    pub patches: Vec<Patch>,
}

impl FilePatch {
    /// Create a new FilePatch for the given file.
    pub fn new(file: FileId) -> Self {
        Self {
            file,
            patches: Vec::new(),
        }
    }

    /// Create a FilePatch with the given patches.
    pub fn with_patches(file: FileId, patches: Vec<Patch>) -> Self {
        Self { file, patches }
    }

    /// Add a patch.
    pub fn push(&mut self, patch: Patch) {
        debug_assert_eq!(patch.file(), self.file, "patch file mismatch");
        self.patches.push(patch);
    }

    /// Add a replacement patch.
    pub fn replace(&mut self, span: Span, new_text: impl Into<String>) {
        self.push(Patch::replace(span, new_text));
    }

    /// Add a deletion patch.
    pub fn delete(&mut self, span: Span) {
        self.push(Patch::delete(span));
    }

    /// Add an insertion patch.
    pub fn insert(&mut self, position: u32, text: impl Into<String>) {
        self.push(Patch::insert(self.file, position, text));
    }

    /// Whether there are no patches.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.patches.is_empty()
    }

    /// Number of patches.
    #[inline]
    pub fn len(&self) -> usize {
        self.patches.len()
    }

    /// Sort patches by position (start, then end).
    pub fn sort(&mut self) {
        self.patches.sort_by_key(|e| (e.span.start, e.span.end));
    }
}

/// Patches across multiple files.
///
/// Used for refactoring operations that touch multiple files (like rename).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub struct PatchSet {
    /// Per-file patches.
    pub files: Vec<FilePatch>,
}

impl PatchSet {
    /// Sort patches into stable order and drop empty file entries.
    pub fn sort(&mut self) {
        self.files.retain(|file| !file.patches.is_empty());
        for file in &mut self.files {
            file.patches
                .sort_by(|left, right| left.order_key().cmp(&right.order_key()));
        }
        self.files.sort_by_key(|file| file.file);
    }

    /// Create an empty PatchSet.
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    /// Create a PatchSet from file patches.
    pub fn from_files(files: Vec<FilePatch>) -> Self {
        Self { files }
    }

    /// Add a FilePatch.
    pub fn push(&mut self, file_patch: FilePatch) {
        self.files.push(file_patch);
    }

    /// Get or create a FilePatch for the given file.
    pub fn file_mut(&mut self, file: FileId) -> &mut FilePatch {
        // return the existing file patch
        let index = self.files.iter().position(|patch| patch.file == file);
        if let Some(index) = index {
            return &mut self.files[index];
        }

        // create a new file patch at the end
        let index = self.files.len();
        self.files.push(FilePatch::new(file));

        &mut self.files[index]
    }

    /// Add a patch to the appropriate file.
    pub fn add(&mut self, patch: Patch) {
        self.file_mut(patch.file()).push(patch);
    }

    /// Whether there are no patches.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() || self.files.iter().all(|f| f.is_empty())
    }

    /// Total number of patches across all files.
    pub fn total_patches(&self) -> usize {
        self.files.iter().map(|f| f.len()).sum()
    }

    /// Number of files affected.
    #[inline]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Iterate over all patches with their file.
    pub fn iter(&self) -> impl Iterator<Item = &Patch> {
        self.files.iter().flat_map(|f| f.patches.iter())
    }
}

impl From<Patch> for FilePatch {
    /// Convert one patch into its single-file collection.
    fn from(patch: Patch) -> Self {
        Self {
            file: patch.file(),
            patches: vec![patch],
        }
    }
}

impl From<Patch> for PatchSet {
    /// Convert one patch into its complete patch set.
    fn from(patch: Patch) -> Self {
        FilePatch::from(patch).into()
    }
}

impl From<FilePatch> for PatchSet {
    /// Convert one file patch into its complete patch set.
    fn from(file: FilePatch) -> Self {
        Self { files: vec![file] }
    }
}

impl FromIterator<FilePatch> for PatchSet {
    fn from_iter<T: IntoIterator<Item = FilePatch>>(iter: T) -> Self {
        Self {
            files: iter.into_iter().collect(),
        }
    }
}

/// Apply one file patch to source text.
pub fn apply_file_patch(file: &File, file_patch: &FilePatch) -> Result<String, PatchApplyError> {
    let mut patches = file_patch.patches.clone();
    patches.sort_by_key(|patch| (patch.span.start, patch.span.end));

    // validate patch order and ownership before mutating text
    let mut previous_end = 0;
    for patch in &patches {
        // reject patches attached to a different file
        if patch.span.file != file_patch.file {
            return Err(PatchApplyError::FileMismatch {
                file: file_patch.file,
                patch_file: patch.span.file,
            });
        }

        // reject patches that would rewrite the same byte twice
        if patch.span.start < previous_end {
            return Err(PatchApplyError::OverlappingPatches {
                file: file_patch.file,
            });
        }

        previous_end = patch.span.end;
    }

    // apply from the back so byte offsets stay stable
    let mut text = file.text().to_string();
    for patch in patches.iter().rev() {
        let start = patch.span.start as usize;
        let end = patch.span.end as usize;

        // reject byte ranges outside the source text
        if start > end || end > text.len() {
            return Err(PatchApplyError::OutsideFile {
                file: file_patch.file,
                start: patch.span.start,
                end: patch.span.end,
                len: text.len(),
            });
        }

        // reject byte ranges that split unicode scalars
        if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            return Err(PatchApplyError::Boundary {
                file: file_patch.file,
                start: patch.span.start,
                end: patch.span.end,
            });
        }

        // replace after validation
        text.replace_range(start..end, &patch.new_text);
    }

    Ok(text)
}

/// Apply one patch set to loaded source files.
pub fn apply_patch_set<'a, F>(
    patches: &PatchSet,
    file_for_id: F,
) -> Result<HashMap<FileId, String>, PatchApplyError>
where
    F: Fn(FileId) -> Option<&'a File>,
{
    let mut updates = HashMap::new();

    for file_patch in &patches.files {
        // require every patched file to be available
        let file = file_for_id(file_patch.file).ok_or(PatchApplyError::MissingFile {
            file: file_patch.file,
        })?;

        // apply one file patch independently
        let text = apply_file_patch(file, file_patch)?;

        updates.insert(file_patch.file, text);
    }

    Ok(updates)
}
