use dyst_dir::{LocalNodeIdAny, Session};

use crate::{
    BindWarning, BuildWarning, CompileError, CompileStage, ElaborateWarning, ExecuteWarning,
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
    /// Warning during lower.
    Lower(LowerWarning),
    /// Warning during execution.
    Execute(ExecuteWarning),
    /// Warning during optimization.
    Optimize(OptimizeWarning),
    /// Warning during building.
    Build(BuildWarning),
    /// Warning during linking.
    Link(LinkWarning),
    /// Suppressed compiler error.
    Suppressed(CompileError),
}

impl CompileWarning {
    /// Get the stage of the warning.
    pub fn stage(&self) -> CompileStage {
        match self {
            Self::Import(_) => CompileStage::Import,
            Self::Bind(_) => CompileStage::Bind,
            Self::Resolve(_) => CompileStage::Resolve,
            Self::Validate(_) => CompileStage::Validate,
            Self::Elaborate(_) => CompileStage::Elaborate,
            Self::Lower(_) => CompileStage::Lower,
            Self::Execute(_) => CompileStage::Execute,
            Self::Optimize(_) => CompileStage::Optimize,
            Self::Build(_) => CompileStage::Build,
            Self::Link(_) => CompileStage::Link,
            Self::Suppressed(error) => error.stage(),
        }
    }

    /// Get the stage letter of the warning.
    pub fn stage_letter(&self) -> char {
        self.stage().letter()
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
            Self::Execute(warning) => warning.sub_code(),
            Self::Optimize(warning) => warning.sub_code(),
            Self::Build(warning) => warning.sub_code(),
            Self::Link(warning) => warning.sub_code(),
            Self::Suppressed(error) => error.sub_code(),
        }
    }

    /// Get the full code of the error (e.g., `IE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        match self {
            Self::Suppressed(error) => error.full_code(),
            _ => format!("{}W{:03}", self.stage_letter(), self.sub_code()),
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::Import(warning) => warning.node_id(),
            Self::Bind(warning) => warning.node_id(),
            Self::Resolve(warning) => warning.node_id(),
            Self::Validate(warning) => warning.node_id(),
            Self::Elaborate(warning) => warning.node_id(),
            Self::Lower(warning) => warning.node_id(),
            Self::Execute(warning) => warning.node_id(),
            Self::Optimize(warning) => warning.node_id(),
            Self::Build(warning) => warning.node_id(),
            Self::Link(warning) => warning.node_id(),
            Self::Suppressed(error) => error.node_id(),
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
            Self::Execute(warning) => warning.message(session),
            Self::Optimize(warning) => warning.message(session),
            Self::Build(warning) => warning.message(session),
            Self::Link(warning) => warning.message(session),
            Self::Suppressed(error) => error.message(session),
        }
    }
}
