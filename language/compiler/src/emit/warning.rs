use std::path::PathBuf;

use crate::{DiagnosticAnchor, DiagnosticDefinition};
use destack_compiler_macros::DefineWarning;
use destack_workspace::Program;

/// Warnings during the emit phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Emit)]
#[standalone]
pub enum EmitWarning {
    /// Overwriting an existing file.
    #[warning(code = "WM001", message = "overwriting existing file")]
    OverwritingFile { path: PathBuf },

    /// Output file is unchanged from previous emit.
    #[warning(code = "WM002", message = "file unchanged")]
    FileUnchanged { path: PathBuf },
}
