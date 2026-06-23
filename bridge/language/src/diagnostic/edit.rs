use crate::{FileId, SourceIdParseError, Span, bridge};

/// One source patch crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Patch {
    /// Source span to replace.
    pub span: Span,
    /// Patch text.
    pub new_text: String,
}

/// Patches for a single file.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FilePatch {
    /// Edited file.
    pub file: FileId,
    /// Source patches.
    pub patches: Vec<Patch>,
}

/// Patches across multiple files.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatchSet {
    /// Per-file patches.
    pub files: Vec<FilePatch>,
}

impl Patch {
    /// Convert one source patch into one bridge patch.
    pub fn from_source(patch: destack_source::Patch) -> Self {
        Self {
            span: patch.span.into(),
            new_text: patch.new_text,
        }
    }

    /// Convert this bridge patch into one source patch.
    pub fn into_source(self) -> Result<destack_source::Patch, SourceIdParseError> {
        Ok(destack_source::Patch::replace(
            self.span.into_source()?,
            self.new_text,
        ))
    }
}

impl FilePatch {
    /// Convert one source file patch into one bridge file patch.
    pub fn from_source(patch: destack_source::FilePatch) -> Self {
        Self {
            file: patch.file.into(),
            patches: patch.patches.into_iter().map(Patch::from_source).collect(),
        }
    }

    /// Convert this bridge file patch into one source file patch.
    pub fn into_source(self) -> Result<destack_source::FilePatch, SourceIdParseError> {
        let file = self.file.into_source()?;
        let patches = self
            .patches
            .into_iter()
            .map(Patch::into_source)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(destack_source::FilePatch::with_patches(file, patches))
    }
}

impl PatchSet {
    /// Convert one source patch set into one bridge patch set.
    pub fn from_source(patch: destack_source::PatchSet) -> Self {
        Self {
            files: patch
                .files
                .into_iter()
                .map(FilePatch::from_source)
                .collect(),
        }
    }

    /// Convert this bridge patch set into one source patch set.
    pub fn into_source(self) -> Result<destack_source::PatchSet, SourceIdParseError> {
        let files = self
            .files
            .into_iter()
            .map(FilePatch::into_source)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(destack_source::PatchSet::from_files(files))
    }
}

impl From<destack_source::Patch> for Patch {
    /// Convert one source patch into one bridge patch.
    fn from(patch: destack_source::Patch) -> Self {
        Self::from_source(patch)
    }
}

impl TryFrom<Patch> for destack_source::Patch {
    type Error = SourceIdParseError;

    /// Convert one bridge patch into one source patch.
    fn try_from(patch: Patch) -> Result<Self, Self::Error> {
        patch.into_source()
    }
}

impl From<destack_source::FilePatch> for FilePatch {
    /// Convert one source file patch into one bridge file patch.
    fn from(patch: destack_source::FilePatch) -> Self {
        Self::from_source(patch)
    }
}

impl TryFrom<FilePatch> for destack_source::FilePatch {
    type Error = SourceIdParseError;

    /// Convert one bridge file patch into one source file patch.
    fn try_from(patch: FilePatch) -> Result<Self, Self::Error> {
        patch.into_source()
    }
}

impl From<destack_source::PatchSet> for PatchSet {
    /// Convert one source patch set into one bridge patch set.
    fn from(patch: destack_source::PatchSet) -> Self {
        Self::from_source(patch)
    }
}

impl TryFrom<PatchSet> for destack_source::PatchSet {
    type Error = SourceIdParseError;

    /// Convert one bridge patch set into one source patch set.
    fn try_from(patch: PatchSet) -> Result<Self, Self::Error> {
        patch.into_source()
    }
}
