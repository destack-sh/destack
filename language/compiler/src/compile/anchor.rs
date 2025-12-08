use destack_dir::GlobalNodeIdAny;
use destack_source::{FileId, ModuleId, PackageId, Span};

use destack_workspace::Program;

/// Where a Diagnostic is anchored in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticAnchor {
    /// Anchored to a specific node in the source.
    Node(GlobalNodeIdAny),
    /// Anchored to a module (no specific node).
    Module(ModuleId),
    /// Anchored to a file (e.g., config file, entry point).
    File(FileId),
    /// Anchored to a package.
    Package(PackageId),
    /// No specific anchor (global/CLI errors).
    Global,
}

impl DiagnosticAnchor {
    /// Get the file and span for this anchor.
    ///
    /// Returns `None` for `Global` anchors or if the file cannot be determined.
    pub fn to_file_span(&self, program: &Program) -> Option<(FileId, Span)> {
        match self {
            Self::Node(node_id) => {
                let module = program.modules.get(node_id.module_id);
                let module = module.read();
                let source_id = module.dir.tree.read().get_source(node_id.local_id.id);
                let span = module.ast.tree.get_span_by_id(source_id);
                Some((module.file_id, span))
            }
            Self::Module(module_id) => {
                let module = program.modules.get(*module_id);
                let module = module.read();
                // point to file start
                Some((module.file_id, Span::empty(module.file_id)))
            }
            Self::File(file_id) => {
                // point to file start
                Some((*file_id, Span::empty(*file_id)))
            }
            Self::Package(package_id) => {
                // point to dsconfig.json or package.json if available
                let package = program.packages.get(*package_id);
                let package = package.read();
                package
                    .dsconfig
                    .as_ref()
                    .map(|c| (c.file_id, Span::empty(c.file_id)))
                    .or_else(|| {
                        package
                            .package_config
                            .as_ref()
                            .map(|c| (c.file_id, Span::empty(c.file_id)))
                    })
            }
            Self::Global => None,
        }
    }

    /// Get the module id for this anchor, if applicable.
    pub fn module_id(&self) -> Option<ModuleId> {
        match self {
            Self::Node(node_id) => Some(node_id.module_id),
            Self::Module(module_id) => Some(*module_id),
            _ => None,
        }
    }

    /// Get the file id for this anchor, if available without program access.
    pub fn file_id(&self) -> Option<FileId> {
        match self {
            Self::File(file_id) => Some(*file_id),
            _ => None,
        }
    }

    /// Get the package id for this anchor, if applicable.
    pub fn package_id(&self) -> Option<PackageId> {
        match self {
            Self::Package(package_id) => Some(*package_id),
            _ => None,
        }
    }

    /// Check if this is a global anchor (no specific location).
    pub fn is_global(&self) -> bool {
        matches!(self, Self::Global)
    }
}

impl From<GlobalNodeIdAny> for DiagnosticAnchor {
    fn from(node: GlobalNodeIdAny) -> Self {
        Self::Node(node)
    }
}

impl From<ModuleId> for DiagnosticAnchor {
    fn from(module: ModuleId) -> Self {
        Self::Module(module)
    }
}

impl From<FileId> for DiagnosticAnchor {
    fn from(file: FileId) -> Self {
        Self::File(file)
    }
}

impl From<PackageId> for DiagnosticAnchor {
    fn from(package: PackageId) -> Self {
        Self::Package(package)
    }
}
