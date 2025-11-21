use dyst_dir::{LocalNodeIdAny, Session};

use crate::{
    BindError, BuildError, CompilePhase, ElaborateError, ExecuteError, FlowError, ImportError,
    LinkError, LowerError, OptimizeError, ResolveError, ValidateError,
};

/// Error during compilation.
#[derive(Debug, Clone)]
pub enum CompileError {
    /// Error during importing.
    Import(ImportError),
    /// Error during binding.
    Bind(BindError),
    /// Error during resolution.
    Resolve(ResolveError),
    /// Error during validation.
    Validate(ValidateError),
    /// Error during elaboration.
    Elaborate(ElaborateError),
    // --------------------------------------------------
    /// Error during lower.
    Lower(LowerError),
    /// Error during flow-checking.
    Flow(FlowError),
    /// Error during optimization.
    Optimize(OptimizeError),
    // --------------------------------------------------
    /// Error during execution.
    Execute(ExecuteError),
    /// Error during building.
    Build(BuildError),
    /// Error during linking.
    Link(LinkError),
}

impl CompileError {
    /// Get the phase of the error.
    pub fn phase(&self) -> CompilePhase {
        match self {
            Self::Import(_) => CompilePhase::Import,
            Self::Bind(_) => CompilePhase::Bind,
            Self::Resolve(_) => CompilePhase::Resolve,
            Self::Validate(_) => CompilePhase::Validate,
            Self::Elaborate(_) => CompilePhase::Elaborate,
            Self::Lower(_) => CompilePhase::Lower,
            Self::Flow(_) => CompilePhase::Flow,
            Self::Optimize(_) => CompilePhase::Optimize,
            Self::Execute(_) => CompilePhase::Execute,
            Self::Build(_) => CompilePhase::Build,
            Self::Link(_) => CompilePhase::Link,
        }
    }

    /// Get the phase letter of the error.
    pub fn phase_letter(&self) -> char {
        self.phase().letter()
    }

    /// Get the numeric sub-code of the error (e.g., `1` for `IE001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Import(error) => error.sub_code(),
            Self::Bind(error) => error.sub_code(),
            Self::Resolve(error) => error.sub_code(),
            Self::Validate(error) => error.sub_code(),
            Self::Elaborate(error) => error.sub_code(),
            Self::Lower(error) => error.sub_code(),
            Self::Flow(error) => error.sub_code(),
            Self::Optimize(error) => error.sub_code(),
            Self::Execute(error) => error.sub_code(),
            Self::Build(error) => error.sub_code(),
            Self::Link(error) => error.sub_code(),
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::Import(error) => error.node_id(),
            Self::Bind(error) => error.node_id(),
            Self::Resolve(error) => error.node_id(),
            Self::Validate(error) => error.node_id(),
            Self::Elaborate(error) => error.node_id(),
            Self::Lower(error) => error.node_id(),
            Self::Flow(error) => error.node_id(),
            Self::Optimize(error) => error.node_id(),
            Self::Execute(error) => error.node_id(),
            Self::Build(error) => error.node_id(),
            Self::Link(error) => error.node_id(),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, session: &'a Session<'a>) -> String {
        match self {
            Self::Import(error) => error.message(session),
            Self::Bind(error) => error.message(session),
            Self::Resolve(error) => error.message(session),
            Self::Validate(error) => error.message(session),
            Self::Elaborate(error) => error.message(session),
            Self::Lower(error) => error.message(session),
            Self::Flow(error) => error.message(session),
            Self::Optimize(error) => error.message(session),
            Self::Execute(error) => error.message(session),
            Self::Build(error) => error.message(session),
            Self::Link(error) => error.message(session),
        }
    }

    /// Get the full code of the error (e.g., `IE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("{}E{:03}", self.phase_letter(), self.sub_code())
    }
}

pub type CompileResult<T> = Result<T, CompileError>;
