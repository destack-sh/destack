use dyst_javascript_ast as ast;
use dyst_source::{FileContent, FileId, FileType};

#[derive(Debug, Clone)]
pub struct TranspilerArtifact {
    /// The file id of the generated artifact.
    pub id: FileId,
    /// The file type of the generated artifact.
    pub ty: FileType,
    /// The AST of the transpiled file.
    pub ast: ast::NodeTree,
    /// The source files.
    pub sources: Vec<FileId>,
    /// The content of the generated artifact.
    pub content: FileContent,
}

impl TranspilerArtifact {}
