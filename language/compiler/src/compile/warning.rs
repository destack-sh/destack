use dyst_dir::{GlobalNodeIdAny, Session};

use crate::{
    BindWarning, BuildWarning, CompilePhase, ElaborateWarning, ExecuteWarning, FlowWarning,
    ImportWarning, LinkWarning, LowerWarning, OptimizeWarning, ResolveWarning, ValidateWarning,
};

/// Warning during compilation.
#[derive(Debug, Clone)]
pub enum CompileWarning {
    /// Warning during importing.
    Import(ImportWarning),
    /// Warning during binding.
    Bind(BindWarning),
    /// Warning during resolution.
    Resolve(ResolveWarning),
    /// Warning during validation.
    Validate(ValidateWarning),
    /// Warning during elaboration.
    Elaborate(ElaborateWarning),
    // --------------------------------------------------
    /// Warning during lower.
    Lower(LowerWarning),
    /// Warning during flow-checking.
    Flow(FlowWarning),
    /// Warning during optimization.
    Optimize(OptimizeWarning),
    // --------------------------------------------------
    /// Warning during execution.
    Execute(ExecuteWarning),
    /// Warning during building.
    Build(BuildWarning),
    /// Warning during linking.
    Link(LinkWarning),
}

impl CompileWarning {
    /// Get the phase of the warning.
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

    /// Get the phase letter of the warning.
    pub fn phase_letter(&self) -> char {
        self.phase().letter()
    }

    /// Get the numeric sub-code of the warning (e.g., `1` for `IE001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Import(warning) => warning.sub_code(),
            Self::Bind(warning) => warning.sub_code(),
            Self::Resolve(warning) => warning.sub_code(),
            Self::Validate(warning) => warning.sub_code(),
            Self::Elaborate(warning) => warning.sub_code(),
            Self::Lower(warning) => warning.sub_code(),
            Self::Flow(warning) => warning.sub_code(),
            Self::Optimize(warning) => warning.sub_code(),
            Self::Execute(warning) => warning.sub_code(),
            Self::Build(warning) => warning.sub_code(),
            Self::Link(warning) => warning.sub_code(),
        }
    }

    /// Get the full code of the error (e.g., `IE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("{}W{:03}", self.phase_letter(), self.sub_code())
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Import(warning) => warning.node_id(),
            Self::Bind(warning) => warning.node_id(),
            Self::Resolve(warning) => warning.node_id(),
            Self::Validate(warning) => warning.node_id(),
            Self::Elaborate(warning) => warning.node_id(),
            Self::Lower(warning) => warning.node_id(),
            Self::Flow(warning) => warning.node_id(),
            Self::Execute(warning) => warning.node_id(),
            Self::Optimize(warning) => warning.node_id(),
            Self::Build(warning) => warning.node_id(),
            Self::Link(warning) => warning.node_id(),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, session: &'a Session<'a>) -> String {
        match self {
            Self::Import(warning) => warning.message(session),
            Self::Bind(warning) => warning.message(session),
            Self::Resolve(warning) => warning.message(session),
            Self::Validate(warning) => warning.message(session),
            Self::Elaborate(warning) => warning.message(session),
            Self::Lower(warning) => warning.message(session),
            Self::Flow(warning) => warning.message(session),
            Self::Execute(warning) => warning.message(session),
            Self::Optimize(warning) => warning.message(session),
            Self::Build(warning) => warning.message(session),
            Self::Link(warning) => warning.message(session),
        }
    }
}
