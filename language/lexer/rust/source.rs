pub type SourceId = u32;

/// A file inside the `SourceMap`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile<'a> {
    /// The path of the SourceFile.
    pub path: SourceId,
    /// The content of the source file.
    pub content: &'a str,
    /// The length of the SourceFile in bytes.
    pub len: u32,
}

impl<'a> SourceFile<'a> {
    pub fn new(path: SourceId, content: &'a str, len: u32) -> Self {
        Self { path, content, len }
    }
}

/// Map of files in the current compilation unit.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {}
