use std::path::PathBuf;

use crate::{DiagnosticAnchor, DiagnosticDefinition};
use destack_compiler_macros::DefineWarning;
use destack_workspace::Program;

/// Warnings during the emit phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Emit)]
#[standalone]
pub enum EmitWarning {
    // -------------------------------------------------------------------------
    // 1xx: File issues
    // -------------------------------------------------------------------------
    /// Overwriting an existing file.
    #[warning(code = "WW100", message = "overwriting existing file")]
    OverwritingFile { path: PathBuf },

    /// Output file is unchanged from previous emit.
    #[warning(code = "WW101", message = "file unchanged")]
    FileUnchanged { path: PathBuf },
}
