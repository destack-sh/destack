use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_source::ModuleId;
use destack_workspace::Program;

/// Warnings during the import phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Import)]
pub enum ImportWarning {
    /// Huge file.
    #[warning(code = "WI001", message = "oversized file ({len} bytes)")]
    OversizedFile { module: ModuleId, len: usize },
}
