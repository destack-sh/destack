use destack_source::{FileContentId, FileId, ModuleId, PackageId, Span};
use {destack_ast as ast, destack_dir as dir, destack_mir as mir};

use crate::{ArtifactKey, ArtifactVersion, DiagnosticError};

/// Provider-side source anchor for one diagnostic label.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticAnchor {
    /// A concrete span in an exact file image.
    Source {
        /// The exact file content containing the span.
        content: FileContentId,
        /// The source span inside the file content.
        span: Span,
    },
    /// A whole exact file image.
    File {
        /// The exact file content.
        content: FileContentId,
        /// The file containing the content.
        file: FileId,
    },
    /// An AST node in an exact AST artifact.
    Ast {
        /// The exact AST artifact containing the node.
        artifact: ArtifactVersion,
        /// The local AST node id.
        node: ast::LocalNodeIdAny,
    },
    /// A DIR node in an exact DIR artifact.
    Dir {
        /// The exact DIR artifact containing the node.
        artifact: ArtifactVersion,
        /// The global node id within the artifact tree.
        node: dir::GlobalNodeIdAny,
    },
    /// A MIR node in an exact MIR artifact.
    Mir {
        /// The exact MIR artifact containing the node.
        artifact: ArtifactVersion,
        /// The global node id within the artifact tree.
        node: mir::GlobalNodeIdAny,
    },
}

impl DiagnosticAnchor {
    /// Return the module id carried by this anchor when available.
    pub fn module_id(&self) -> Option<ModuleId> {
        match self {
            Self::Ast { artifact, .. } => artifact.module_id(),
            Self::Dir { node, .. } => Some(node.module_id),
            Self::Mir { node, .. } => Some(node.module_id),
            Self::Source { .. } | Self::File { .. } => None,
        }
    }

    /// Return the file id carried by this anchor when available without repository access.
    pub fn file_id(&self) -> Option<FileId> {
        match self {
            Self::Source { span, .. } => Some(span.file),
            Self::File { file, .. } => Some(*file),
            Self::Ast { .. } | Self::Dir { .. } | Self::Mir { .. } => None,
        }
    }

    /// Return the file content id carried by this anchor when available.
    pub fn file_content_id(&self) -> Option<FileContentId> {
        match self {
            Self::Source { content, .. } | Self::File { content, .. } => Some(*content),
            Self::Ast { .. } | Self::Dir { .. } | Self::Mir { .. } => None,
        }
    }
}

/// Provider-side location seed for one diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticSite {
    /// An already exact diagnostic anchor.
    Anchor(DiagnosticAnchor),
    /// A concrete span in the provider revision.
    Span(Span),
    /// A whole source file in the provider revision.
    File(FileId),
    /// An AST node in the provider revision.
    Ast {
        /// The AST artifact key containing the node.
        key: ArtifactKey,
        /// The local AST node id.
        node: ast::LocalNodeIdAny,
    },
    /// A DIR node in the provider revision.
    Dir {
        /// The DIR artifact key containing the node.
        key: ArtifactKey,
        /// The global DIR node id.
        node: dir::GlobalNodeIdAny,
    },
    /// A MIR node in the provider revision.
    Mir {
        /// The MIR artifact key containing the node.
        key: ArtifactKey,
        /// The global MIR node id.
        node: mir::GlobalNodeIdAny,
    },
    /// A module in the provider revision.
    Module(ModuleId),
    /// A package in the provider revision.
    Package(PackageId),
}

impl DiagnosticSite {
    /// Create a diagnostic site for a declared DIR node.
    pub fn dir_declared(node: dir::AnchoredGlobalNodeId) -> Result<Self, DiagnosticError> {
        let Some(profile) = node.profile_id else {
            return Err(DiagnosticError::InvalidSite {
                message: "DIR diagnostic site requires a profile id".to_string(),
            });
        };
        let key = ArtifactKey::dir_declared(node.node_id.module_id, profile);

        Ok(Self::Dir {
            key,
            node: node.node_id,
        })
    }

    /// Create a diagnostic site for an optimized MIR node.
    pub fn mir_optimized(node: mir::AnchoredGlobalNodeId) -> Result<Self, DiagnosticError> {
        let Some(profile) = node.profile_id else {
            return Err(DiagnosticError::InvalidSite {
                message: "MIR diagnostic site requires a profile id".to_string(),
            });
        };
        let key = ArtifactKey::mir_optimized(node.module_id(), profile, node.target_id);

        Ok(Self::Mir {
            key,
            node: node.node_id,
        })
    }
}

impl From<DiagnosticAnchor> for DiagnosticSite {
    /// Create a diagnostic site from an exact anchor.
    fn from(anchor: DiagnosticAnchor) -> Self {
        Self::Anchor(anchor)
    }
}

impl From<Span> for DiagnosticSite {
    /// Create a diagnostic site from a source span.
    fn from(span: Span) -> Self {
        Self::Span(span)
    }
}

impl From<FileId> for DiagnosticSite {
    /// Create a diagnostic site from a source file.
    fn from(file: FileId) -> Self {
        Self::File(file)
    }
}

impl From<ModuleId> for DiagnosticSite {
    /// Create a diagnostic site from a source module.
    fn from(module: ModuleId) -> Self {
        Self::Module(module)
    }
}

impl From<PackageId> for DiagnosticSite {
    /// Create a diagnostic site from a source package.
    fn from(package: PackageId) -> Self {
        Self::Package(package)
    }
}
