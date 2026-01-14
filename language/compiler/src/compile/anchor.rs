use destack_source::{FileId, ModuleId, PackageId, Span};
use destack_workspace::Program;
use {destack_dir as dir, destack_mir as mir};

use crate::{
    AnalyzeError, AnalyzeWarning, ElaborateError, ElaborateWarning, EmitError, EmitWarning,
    ExecuteError, ExecuteWarning, GenerateError, GenerateWarning, ImportError, ImportWarning,
    LinkError, LinkWarning, LowerError, LowerWarning, OptimizeError, OptimizeWarning, ResolveError,
    ResolveWarning,
};

/// Static metadata about a diagnostic variant.
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticDefinition {
    /// The full code (e.g., "ER003").
    pub code: &'static str,
    /// The variant name (e.g., "UndeclaredSymbol").
    pub name: &'static str,
    /// The doc comment description.
    pub description: &'static str,
    /// The numeric sub-code (e.g., 3 for "ER003").
    pub sub_code: u16,
}

/// Where a Diagnostic is anchored in the source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticAnchor {
    /// Anchored to a specific DIR node (with profile provenance).
    DirNode(dir::AnchoredGlobalNodeId),
    /// Anchored to a specific MIR node (with target provenance).
    MirNode(mir::AnchoredGlobalNodeId),
    /// Anchored to a module (no specific node).
    Module(ModuleId),
    /// Anchored to a file (e.g., config file, entry point).
    File(FileId),
    /// Anchored to a specific span in a file.
    /// Use this for errors in non-AST files (JSON, TOML, YAML) where we have line/column info.
    Span(Span),
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
            Self::DirNode(anchored) => {
                let module = program.modules.get(anchored.module_id());
                let module = module.read();
                let ast = module.ast_maybe()?;
                let local_id = anchored.local_id().id;

                // look in profile-specific DIR if we have a profile
                if let Some(profile_id) = anchored.profile_id
                    && let Some(dir) = module.dir_maybe(profile_id)
                {
                    let tree = dir.tree.read();
                    if tree.has_node_id(local_id) {
                        let source_id = tree.get_source(local_id);
                        let span = ast.tree.get_span_by_id(source_id);
                        return Some((module.file_id, span));
                    }
                }

                // fall back to base DIR
                let dir = module.dir_base_maybe()?;
                let tree = dir.tree.read();
                if !tree.has_node_id(local_id) {
                    return None;
                }
                let source_id = tree.get_source(local_id);
                let span = ast.tree.get_span_by_id(source_id);

                Some((module.file_id, span))
            }
            Self::MirNode(anchored) => {
                let module = program.modules.get(anchored.module_id());
                let module = module.read();
                let ast = module.ast_maybe()?;
                let mir = module.mir(&anchored.target_id);
                let mir_tree = mir.tree.read();

                // get source DIR node from MIR
                let dir_node_id = mir_tree.get_source(anchored.local_id().id)?;

                // look up span from base DIR
                let dir = module.dir_base_maybe()?;
                let tree = dir.tree.read();
                if !tree.has_node_id(dir_node_id) {
                    return None;
                }
                let source_id = tree.get_source(dir_node_id);
                let span = ast.tree.get_span_by_id(source_id);

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
            Self::Span(span) => {
                // use the exact span provided
                Some((span.file, *span))
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
                            .manifest
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
            Self::DirNode(anchored) => Some(anchored.module_id()),
            Self::MirNode(anchored) => Some(anchored.module_id()),
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

impl From<dir::AnchoredGlobalNodeId> for DiagnosticAnchor {
    fn from(anchored: dir::AnchoredGlobalNodeId) -> Self {
        Self::DirNode(anchored)
    }
}

impl From<dir::GlobalNodeIdAny> for DiagnosticAnchor {
    fn from(node: dir::GlobalNodeIdAny) -> Self {
        // assume base DIR (no profile) for un-anchored nodes
        Self::DirNode(dir::AnchoredGlobalNodeId::base(node))
    }
}

impl From<mir::AnchoredGlobalNodeId> for DiagnosticAnchor {
    fn from(anchored: mir::AnchoredGlobalNodeId) -> Self {
        Self::MirNode(anchored)
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

impl From<Span> for DiagnosticAnchor {
    fn from(span: Span) -> Self {
        Self::Span(span)
    }
}

impl From<PackageId> for DiagnosticAnchor {
    fn from(package: PackageId) -> Self {
        Self::Package(package)
    }
}

/// Registry of all compiler diagnostic codes.
///
/// Provides compile-time access to all valid error and warning codes,
/// organized by phase. Use this for validation at CLI and test boundaries.
#[derive(Debug)]
pub struct DiagnosticRegistry;

impl DiagnosticRegistry {
    /// All error definitions from all phases.
    pub const ALL_ERRORS: &'static [&'static [DiagnosticDefinition]] = &[
        ImportError::ALL,
        ResolveError::ALL,
        AnalyzeError::ALL,
        ElaborateError::ALL,
        ExecuteError::ALL,
        LowerError::ALL,
        OptimizeError::ALL,
        GenerateError::ALL,
        LinkError::ALL,
        EmitError::ALL,
    ];

    /// All warning definitions from all phases.
    pub const ALL_WARNINGS: &'static [&'static [DiagnosticDefinition]] = &[
        ImportWarning::ALL,
        ResolveWarning::ALL,
        AnalyzeWarning::ALL,
        ElaborateWarning::ALL,
        ExecuteWarning::ALL,
        LowerWarning::ALL,
        OptimizeWarning::ALL,
        GenerateWarning::ALL,
        LinkWarning::ALL,
        EmitWarning::ALL,
    ];

    /// Check if an error code is valid.
    pub fn is_valid_error_code(code: &str) -> bool {
        ImportError::is_valid_code(code)
            || ResolveError::is_valid_code(code)
            || AnalyzeError::is_valid_code(code)
            || ElaborateError::is_valid_code(code)
            || ExecuteError::is_valid_code(code)
            || LowerError::is_valid_code(code)
            || OptimizeError::is_valid_code(code)
            || GenerateError::is_valid_code(code)
            || LinkError::is_valid_code(code)
            || EmitError::is_valid_code(code)
    }

    /// Check if a warning code is valid.
    pub fn is_valid_warning_code(code: &str) -> bool {
        ImportWarning::is_valid_code(code)
            || ResolveWarning::is_valid_code(code)
            || AnalyzeWarning::is_valid_code(code)
            || ElaborateWarning::is_valid_code(code)
            || ExecuteWarning::is_valid_code(code)
            || LowerWarning::is_valid_code(code)
            || OptimizeWarning::is_valid_code(code)
            || GenerateWarning::is_valid_code(code)
            || LinkWarning::is_valid_code(code)
            || EmitWarning::is_valid_code(code)
    }

    /// Check if a diagnostic code (error or warning) is valid.
    #[inline]
    pub fn is_valid_code(code: &str) -> bool {
        Self::is_valid_error_code(code) || Self::is_valid_warning_code(code)
    }

    /// Look up a diagnostic definition by code.
    pub fn definition_for_code(code: &str) -> Option<&'static DiagnosticDefinition> {
        ImportError::def_for_code(code)
            .or_else(|| ResolveError::def_for_code(code))
            .or_else(|| AnalyzeError::def_for_code(code))
            .or_else(|| ElaborateError::def_for_code(code))
            .or_else(|| ExecuteError::def_for_code(code))
            .or_else(|| LowerError::def_for_code(code))
            .or_else(|| OptimizeError::def_for_code(code))
            .or_else(|| GenerateError::def_for_code(code))
            .or_else(|| LinkError::def_for_code(code))
            .or_else(|| EmitError::def_for_code(code))
            .or_else(|| ImportWarning::def_for_code(code))
            .or_else(|| ResolveWarning::def_for_code(code))
            .or_else(|| AnalyzeWarning::def_for_code(code))
            .or_else(|| ElaborateWarning::def_for_code(code))
            .or_else(|| ExecuteWarning::def_for_code(code))
            .or_else(|| LowerWarning::def_for_code(code))
            .or_else(|| OptimizeWarning::def_for_code(code))
            .or_else(|| GenerateWarning::def_for_code(code))
            .or_else(|| LinkWarning::def_for_code(code))
            .or_else(|| EmitWarning::def_for_code(code))
    }

    /// Look up a diagnostic definition by variant name.
    pub fn definition_for_name(name: &str) -> Option<&'static DiagnosticDefinition> {
        let matches_name = |definition: &&DiagnosticDefinition| definition.name == name;

        Self::ALL_ERRORS
            .iter()
            .flat_map(|defs| defs.iter())
            .find(matches_name)
            .or_else(|| {
                Self::ALL_WARNINGS
                    .iter()
                    .flat_map(|defs| defs.iter())
                    .find(matches_name)
            })
    }

    /// Get all error codes for a given phase letter.
    pub fn error_codes_for_phase(letter: char) -> &'static [&'static str] {
        match letter {
            'I' => ImportError::ALL_CODES,
            'R' => ResolveError::ALL_CODES,
            'A' => AnalyzeError::ALL_CODES,
            'E' => ElaborateError::ALL_CODES,
            'X' => ExecuteError::ALL_CODES,
            'M' => LowerError::ALL_CODES,
            'O' => OptimizeError::ALL_CODES,
            'G' => GenerateError::ALL_CODES,
            'K' => LinkError::ALL_CODES,
            'W' => EmitError::ALL_CODES,
            _ => &[],
        }
    }

    /// Get all warning codes for a given phase letter.
    pub fn warning_codes_for_phase(letter: char) -> &'static [&'static str] {
        match letter {
            'I' => ImportWarning::ALL_CODES,
            'R' => ResolveWarning::ALL_CODES,
            'A' => AnalyzeWarning::ALL_CODES,
            'E' => ElaborateWarning::ALL_CODES,
            'X' => ExecuteWarning::ALL_CODES,
            'M' => LowerWarning::ALL_CODES,
            'O' => OptimizeWarning::ALL_CODES,
            'G' => GenerateWarning::ALL_CODES,
            'K' => LinkWarning::ALL_CODES,
            'W' => EmitWarning::ALL_CODES,
            _ => &[],
        }
    }

    /// Validate a list of codes and return any invalid ones.
    pub fn validate_codes(codes: &[String]) -> Vec<&str> {
        codes
            .iter()
            .filter(|c| !Self::is_valid_code(c))
            .map(|c| c.as_str())
            .collect()
    }
}
