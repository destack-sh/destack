use destack_workspace::Program;

use crate::{
    AnalyzeWarning, BindWarning, DiagnosticAnchor, ElaborateWarning, ExecuteWarning,
    GenerateWarning, ImportWarning, LinkWarning, LowerWarning, OptimizeWarning, ResolveWarning,
    TaskPhase, VerifyWarning,
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
    /// Warning during analysis.
    Analyze(AnalyzeWarning),
    /// Warning during elaboration.
    Elaborate(ElaborateWarning),
    // --------------------------------------------------
    /// Warning during execution.
    Execute(ExecuteWarning),
    /// Warning during lowering.
    Lower(LowerWarning),
    /// Warning during verification.
    Verify(VerifyWarning),
    /// Warning during optimization.
    Optimize(OptimizeWarning),
    // --------------------------------------------------
    /// Warning during generating.
    Generate(GenerateWarning),
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
            Self::Analyze(_) => TaskPhase::Analyze,
            Self::Elaborate(_) => TaskPhase::Elaborate,
            Self::Execute(_) => TaskPhase::Execute,
            Self::Lower(_) => TaskPhase::Lower,
            Self::Verify(_) => TaskPhase::Verify,
            Self::Optimize(_) => TaskPhase::Optimize,
            Self::Generate(_) => TaskPhase::Generate,
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
            Self::Analyze(warning) => warning.sub_code(),
            Self::Elaborate(warning) => warning.sub_code(),
            Self::Execute(warning) => warning.sub_code(),
            Self::Lower(warning) => warning.sub_code(),
            Self::Verify(warning) => warning.sub_code(),
            Self::Optimize(warning) => warning.sub_code(),
            Self::Generate(warning) => warning.sub_code(),
            Self::Link(warning) => warning.sub_code(),
        }
    }

    /// Get the full code of the error (e.g., `IE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("W{}{:03}", self.phase_letter(), self.sub_code())
    }

    /// Get the anchor of the warning.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Import(warning) => warning.anchor(),
            Self::Bind(warning) => warning.anchor(),
            Self::Resolve(warning) => warning.anchor(),
            Self::Analyze(warning) => warning.anchor(),
            Self::Elaborate(warning) => warning.anchor(),
            Self::Execute(warning) => warning.anchor(),
            Self::Lower(warning) => warning.anchor(),
            Self::Verify(warning) => warning.anchor(),
            Self::Optimize(warning) => warning.anchor(),
            Self::Generate(warning) => warning.anchor(),
            Self::Link(warning) => warning.anchor(),
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Import(warning) => warning.message(program),
            Self::Bind(warning) => warning.message(program),
            Self::Resolve(warning) => warning.message(program),
            Self::Analyze(warning) => warning.message(program),
            Self::Elaborate(warning) => warning.message(program),
            Self::Execute(warning) => warning.message(program),
            Self::Lower(warning) => warning.message(program),
            Self::Verify(warning) => warning.message(program),
            Self::Optimize(warning) => warning.message(program),
            Self::Generate(warning) => warning.message(program),
            Self::Link(warning) => warning.message(program),
        }
    }
}
