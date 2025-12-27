use destack_dir::GlobalNodeIdAny;
use destack_source::{FileId, ModuleId, PackageId, Span};
use destack_workspace::Program;

use crate::{
    AnalyzeError, AnalyzeWarning, BindError, BindWarning, ElaborateError, ElaborateWarning,
    EmitError, EmitWarning, ExecuteError, ExecuteWarning, GenerateError, GenerateWarning,
    ImportError, ImportWarning, LinkError, LinkWarning, LowerError, LowerWarning, OptimizeError,
    OptimizeWarning, ResolveError, ResolveWarning, VerifyError, VerifyWarning,
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
    pub sub_code: u8,
}

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
                let profile = program.default_profile_id_for_module(module.id);
                let dir = module
                    .dir_maybe(profile)
                    .or_else(|| module.dir_base_maybe())?;
                let ast = module.ast.as_ref()?;
                let source_id = dir.tree.read().get_source(node_id.local_id.id);
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
        BindError::ALL,
        ResolveError::ALL,
        AnalyzeError::ALL,
        ElaborateError::ALL,
        LowerError::ALL,
        VerifyError::ALL,
        ExecuteError::ALL,
        OptimizeError::ALL,
        GenerateError::ALL,
        LinkError::ALL,
        EmitError::ALL,
    ];

    /// All warning definitions from all phases.
    pub const ALL_WARNINGS: &'static [&'static [DiagnosticDefinition]] = &[
        ImportWarning::ALL,
        BindWarning::ALL,
        ResolveWarning::ALL,
        AnalyzeWarning::ALL,
        ElaborateWarning::ALL,
        LowerWarning::ALL,
        VerifyWarning::ALL,
        ExecuteWarning::ALL,
        OptimizeWarning::ALL,
        GenerateWarning::ALL,
        LinkWarning::ALL,
        EmitWarning::ALL,
    ];

    /// Check if an error code is valid.
    pub fn is_valid_error_code(code: &str) -> bool {
        ImportError::is_valid_code(code)
            || BindError::is_valid_code(code)
            || ResolveError::is_valid_code(code)
            || AnalyzeError::is_valid_code(code)
            || ElaborateError::is_valid_code(code)
            || LowerError::is_valid_code(code)
            || VerifyError::is_valid_code(code)
            || ExecuteError::is_valid_code(code)
            || OptimizeError::is_valid_code(code)
            || GenerateError::is_valid_code(code)
            || LinkError::is_valid_code(code)
            || EmitError::is_valid_code(code)
    }

    /// Check if a warning code is valid.
    pub fn is_valid_warning_code(code: &str) -> bool {
        ImportWarning::is_valid_code(code)
            || BindWarning::is_valid_code(code)
            || ResolveWarning::is_valid_code(code)
            || AnalyzeWarning::is_valid_code(code)
            || ElaborateWarning::is_valid_code(code)
            || LowerWarning::is_valid_code(code)
            || VerifyWarning::is_valid_code(code)
            || ExecuteWarning::is_valid_code(code)
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

    /// Get all error codes for a given phase letter.
    pub fn error_codes_for_phase(letter: char) -> &'static [&'static str] {
        match letter {
            'I' => ImportError::ALL_CODES,
            'B' => BindError::ALL_CODES,
            'R' => ResolveError::ALL_CODES,
            'A' => AnalyzeError::ALL_CODES,
            'E' => ElaborateError::ALL_CODES,
            'M' => LowerError::ALL_CODES,
            'V' => VerifyError::ALL_CODES,
            'X' => ExecuteError::ALL_CODES,
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
            'B' => BindWarning::ALL_CODES,
            'R' => ResolveWarning::ALL_CODES,
            'A' => AnalyzeWarning::ALL_CODES,
            'E' => ElaborateWarning::ALL_CODES,
            'M' => LowerWarning::ALL_CODES,
            'V' => VerifyWarning::ALL_CODES,
            'X' => ExecuteWarning::ALL_CODES,
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
