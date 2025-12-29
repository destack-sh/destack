use destack_workspace::Program;

use crate::{
    AnalyzeError, BindError, DiagnosticAnchor, ElaborateError, EmitError, ExecuteError,
    GenerateError, ImportError, LinkError, LintError, LowerError, OptimizeError, ResolveError,
    TaskDependency, TaskId, TaskPhase, VerifyError,
};
/// Error during compilation.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskError {
    /// Error during importing.
    Import(ImportError),
    /// Error during binding.
    Bind(BindError),
    /// Error during resolution.
    Resolve(ResolveError),
    /// Error during analysis.
    Analyze(AnalyzeError),
    /// Error during elaboration.
    Elaborate(ElaborateError),
    // --------------------------------------------------
    /// Error during execution.
    Execute(ExecuteError),
    /// Error during lowering.
    Lower(LowerError),
    /// Error during verification.
    Verify(VerifyError),
    /// Error during optimization.
    Optimize(OptimizeError),
    // --------------------------------------------------
    /// Error during generating.
    Generate(GenerateError),
    /// Error during linking.
    Link(LinkError),
    /// Error during emitting.
    Emit(EmitError),
    // --------------------------------------------------
    /// Error during linting.
    Lint(LintError),
    // --------------------------------------------------
    /// Internal compiler error (bug).
    Internal(InternalError),
}

/// Internal compiler error (bug in the compiler).
#[derive(Debug, Clone, PartialEq)]
pub enum InternalError {
    /// Task yielded to the same dependency twice in a row.
    SuspiciousYield {
        task_id: TaskId,
        dependency: TaskDependency,
    },
    /// Task exceeded maximum yield count.
    ExcessiveYield { task_id: TaskId, yield_count: u32 },
    /// Circular dependency detected in task graph.
    CircularDependency { task_id: TaskId, cycle: Vec<TaskId> },
}

impl InternalError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::SuspiciousYield { .. } => 1,
            Self::ExcessiveYield { .. } => 2,
            Self::CircularDependency { .. } => 3,
        }
    }

    /// Get the anchor of the error.
    pub fn anchor(&self) -> DiagnosticAnchor {
        // internal errors are global, not tied to specific source
        DiagnosticAnchor::Global
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::SuspiciousYield { task_id, .. } => {
                format!("internal error: task {task_id} yielded to the same dependency twice")
            }
            Self::ExcessiveYield {
                task_id,
                yield_count,
                ..
            } => {
                format!("internal error: task {task_id} yielded {yield_count} times")
            }
            Self::CircularDependency { task_id, cycle, .. } => {
                let cycle_str = cycle
                    .iter()
                    .map(|id| format!("{id}"))
                    .collect::<Vec<_>>()
                    .join(" -> ");
                format!("internal error: circular dependency involving {task_id}: {cycle_str}")
            }
        }
    }
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EZ{:03}", self.sub_code())
    }
}

impl From<InternalError> for TaskError {
    #[inline]
    fn from(error: InternalError) -> Self {
        TaskError::Internal(error)
    }
}

impl TaskError {
    /// Get the phase of the error, if applicable.
    pub fn phase(&self) -> Option<TaskPhase> {
        match self {
            Self::Import(_) => Some(TaskPhase::Import),
            Self::Bind(_) => Some(TaskPhase::Bind),
            Self::Resolve(_) => Some(TaskPhase::Resolve),
            Self::Analyze(_) => Some(TaskPhase::Analyze),
            Self::Elaborate(_) => Some(TaskPhase::Elaborate),
            Self::Execute(_) => Some(TaskPhase::Execute),
            Self::Lower(_) => Some(TaskPhase::Lower),
            Self::Verify(_) => Some(TaskPhase::Verify),
            Self::Optimize(_) => Some(TaskPhase::Optimize),
            Self::Generate(_) => Some(TaskPhase::Generate),
            Self::Link(_) => Some(TaskPhase::Link),
            Self::Emit(_) => Some(TaskPhase::Emit),
            Self::Lint(_) => Some(TaskPhase::Lint),
            Self::Internal(_) => None,
        }
    }

    /// Get the phase letter of the error.
    pub fn phase_letter(&self) -> char {
        match self.phase() {
            Some(phase) => phase.letter(),
            None => 'Z', // Z for internal compiler error
        }
    }

    /// Get the numeric sub-code of the error (e.g., `1` for `IE001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Import(error) => error.sub_code(),
            Self::Bind(error) => error.sub_code(),
            Self::Resolve(error) => error.sub_code(),
            Self::Analyze(error) => error.sub_code(),
            Self::Elaborate(error) => error.sub_code(),
            Self::Execute(error) => error.sub_code(),
            Self::Lower(error) => error.sub_code(),
            Self::Verify(error) => error.sub_code(),
            Self::Optimize(error) => error.sub_code(),
            Self::Generate(error) => error.sub_code(),
            Self::Link(error) => error.sub_code(),
            Self::Emit(error) => error.sub_code(),
            Self::Lint(error) => error.sub_code(),
            Self::Internal(error) => error.sub_code(),
        }
    }

    /// Get the anchor of the error.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Import(error) => error.anchor(),
            Self::Bind(error) => error.anchor(),
            Self::Resolve(error) => error.anchor(),
            Self::Analyze(error) => error.anchor(),
            Self::Elaborate(error) => error.anchor(),
            Self::Lower(error) => error.anchor(),
            Self::Verify(error) => error.anchor(),
            Self::Execute(error) => error.anchor(),
            Self::Optimize(error) => error.anchor(),
            Self::Generate(error) => error.anchor(),
            Self::Link(error) => error.anchor(),
            Self::Emit(error) => error.anchor(),
            Self::Lint(error) => error.anchor(),
            Self::Internal(error) => error.anchor(),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Import(error) => error.message(program),
            Self::Bind(error) => error.message(program),
            Self::Resolve(error) => error.message(program),
            Self::Analyze(error) => error.message(program),
            Self::Elaborate(error) => error.message(program),
            Self::Lower(error) => error.message(program),
            Self::Verify(error) => error.message(program),
            Self::Execute(error) => error.message(program),
            Self::Optimize(error) => error.message(program),
            Self::Generate(error) => error.message(program),
            Self::Link(error) => error.message(program),
            Self::Emit(error) => error.message(program),
            Self::Lint(error) => error.message(program),
            Self::Internal(error) => error.message(program),
        }
    }

    /// Get the full code of the error (e.g., `IE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("E{}{:03}", self.phase_letter(), self.sub_code())
    }
}
