use crate::PackageId;

/// The special intent of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileMode {
    /// Generic source file (`.ds`)
    Source,
    /// Generic source declaration file (`.d.ds`)
    SourceDeclaration,
    /// Generic data file (`.dst`, `.dsb`)
    Data,
}

/// A source File (might be on disk, might also be virtual or in-memory).
#[derive(Debug, Clone)]
pub struct File {
    /// The ID of this source.
    pub id: SourceId,
    /// The ID of the containing package.
    pub package_id: PackageId,
    /// The name of the file.
    pub name: String,
    /// The URI of the SourceFile.
    pub uri: Uri,
    /// The format of the file.
    pub format: SourceFormat,
    /// The mode of the file.
    pub mode: FileMode,
    /// Whether the file is currently open (in editor context).
    pub is_open: bool,
}