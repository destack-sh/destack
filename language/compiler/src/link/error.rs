use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_source::PackageId;
use destack_workspace::Program;

/// Errors during the link phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Link)]
pub enum LinkError {
    /// Wait for task dependency.
    #[error(code = "EK000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EK001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Internal error during linking.
    #[error(code = "EK002", message = "internal error")]
    Internal { package: PackageId, message: String },

    /// Missing target.
    #[error(code = "EK003", message = "missing target: {target}")]
    MissingTarget { package: PackageId, target: String },

    /// Invalid target configuration.
    #[error(code = "EK004", message = "invalid target: {target}: {message}")]
    InvalidTarget {
        package: PackageId,
        target: String,
        message: String,
    },
}
