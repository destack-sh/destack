use destack_dir::{GlobalNodeIdAny, Program};

use crate::{
    AnalyzeWarning, BindWarning, BuildWarning, ElaborateWarning, ExecuteWarning, ImportWarning,
    LinkWarning, LowerWarning, OptimizeWarning, TaskPhase, ResolveWarning, ValidateWarning,
};

/// Warning during compilation.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskWarning {
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
    /// Warning during analysis.
    Analyze(AnalyzeWarning),
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

impl TaskWarning {
    /// Get the phase of the warning.
    pub fn phase(&self) -> TaskPhase {
        match self {
            Self::Import(_) => TaskPhase::Import,
            Self::Bind(_) => TaskPhase::Bind,
            Self::Resolve(_) => TaskPhase::Resolve,
            Self::Validate(_) => TaskPhase::Validate,
            Self::Elaborate(_) => TaskPhase::Elaborate,
            Self::Lower(_) => TaskPhase::Lower,
            Self::Analyze(_) => TaskPhase::Analyze,
            Self::Optimize(_) => TaskPhase::Optimize,
            Self::Execute(_) => TaskPhase::Execute,
            Self::Build(_) => TaskPhase::Build,
            Self::Link(_) => TaskPhase::Link,
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
            Self::Analyze(warning) => warning.sub_code(),
            Self::Optimize(warning) => warning.sub_code(),
            Self::Execute(warning) => warning.sub_code(),
            Self::Build(warning) => warning.sub_code(),
            Self::Link(warning) => warning.sub_code(),
        }
    }

    /// Get the full code of the error (e.g., `IE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("W{}{:03}", self.phase_letter(), self.sub_code())
    }

    /// Get the node of the warning.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Import(warning) => warning.node(),
            Self::Bind(warning) => warning.node(),
            Self::Resolve(warning) => warning.node(),
            Self::Validate(warning) => warning.node(),
            Self::Elaborate(warning) => warning.node(),
            Self::Lower(warning) => warning.node(),
            Self::Analyze(warning) => warning.node(),
            Self::Execute(warning) => warning.node(),
            Self::Optimize(warning) => warning.node(),
            Self::Build(warning) => warning.node(),
            Self::Link(warning) => warning.node(),
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Import(warning) => warning.message(program),
            Self::Bind(warning) => warning.message(program),
            Self::Resolve(warning) => warning.message(program),
            Self::Validate(warning) => warning.message(program),
            Self::Elaborate(warning) => warning.message(program),
            Self::Lower(warning) => warning.message(program),
            Self::Analyze(warning) => warning.message(program),
            Self::Execute(warning) => warning.message(program),
            Self::Optimize(warning) => warning.message(program),
            Self::Build(warning) => warning.message(program),
            Self::Link(warning) => warning.message(program),
        }
    }
}
