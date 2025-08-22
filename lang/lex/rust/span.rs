use std::collections::HashMap;

/// A position range in a `SourceFile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

/// The identifier type used by AST nodes.
pub type NodeId = u32;

/// A file inside the `SourceMap`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub id: NodeId,
    pub path: String,
}

/// Keep track of files participating in the AST.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    pub source_files: HashMap<NodeId, SourceFile>,
}
