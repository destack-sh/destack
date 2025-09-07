#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceId(u32);

impl SourceId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A "source" inside the `SourceMap` (Like a file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// The path of the SourceFile.
    pub path: SourceId,
    /// The content of the source file.
    pub content: String,
    /// The length of the SourceFile in bytes.
    pub len: u32,
}

impl Source {
    pub fn new(path: SourceId, content: String) -> Self {
        let len = content.len() as u32;
        Self { path, content, len }
    }
}
