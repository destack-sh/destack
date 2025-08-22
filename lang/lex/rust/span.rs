use std::collections::HashMap;

use crate::Token;

type SourceId = u32;

/// A position range in a `SourceFile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// The start position of the Span in bytes.
    pub start: u32,
    /// The end position of the Span in bytes.
    pub end: u32,
}

/// A semantic Token with a Span.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SemanticToken {
    /// The Token.
    pub token: Token,
    /// The Span of the Token in its SourceFile.
    pub span: Span,
}

/// A file inside the `SourceMap`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// The path of the SourceFile.
    pub path: SourceId,
    /// The length of the SourceFile in bytes.
    pub len: u32,
}

/// Map of files in the current compilation unit.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    pub source_files: HashMap<SourceId, SourceFile>,
}
