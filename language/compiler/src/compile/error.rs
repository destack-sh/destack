use destack_dir::{GlobalNodeIdAny, Program};

use crate::{
    AnalyzeError, BindError, ElaborateError, GenerateError, ImportError, LinkError, LowerError,
    OptimizeError, ResolveError, TaskDependency, TaskId, TaskPhase, VerifyError,
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
    /// Error during lower.
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
    // --------------------------------------------------
    /// Internal compiler error (bug).
    Internal(InternalError),
}

/// Internal compiler error (bug in the compiler).
#[derive(Debug, Clone, PartialEq)]
pub enum InternalError {
    /// Task yielded to the same dependency twice in a row.
    SuspiciousYield {
        node: GlobalNodeIdAny,
        task_id: TaskId,
        dependency: TaskDependency,
    },
    /// Task exceeded maximum yield count.
    ExcessiveYield {
        node: GlobalNodeIdAny,
        task_id: TaskId,
        yield_count: u32,
    },
    /// Circular dependency detected in task graph.
    CircularDependency {
        node: GlobalNodeIdAny,
        task_id: TaskId,
        cycle: Vec<TaskId>,
    },
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

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::SuspiciousYield { node, .. }
            | Self::ExcessiveYield { node, .. }
            | Self::CircularDependency { node, .. } => *node,
        }
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
        write!(f, "EC{:03}", self.sub_code())
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
            Self::Lower(_) => Some(TaskPhase::Lower),
            Self::Verify(_) => Some(TaskPhase::Verify),
            Self::Optimize(_) => Some(TaskPhase::Optimize),
            Self::Generate(_) => Some(TaskPhase::Generate),
            Self::Link(_) => Some(TaskPhase::Link),
            Self::Internal(_) => None,
        }
    }

    /// Get the phase letter of the error.
    pub fn phase_letter(&self) -> char {
        match self.phase() {
            Some(phase) => phase.letter(),
            None => 'C', // C for Compiler internal error
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
            Self::Lower(error) => error.sub_code(),
            Self::Verify(error) => error.sub_code(),
            Self::Optimize(error) => error.sub_code(),
            Self::Generate(error) => error.sub_code(),
            Self::Link(error) => error.sub_code(),
            Self::Internal(error) => error.sub_code(),
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Import(error) => error.node(),
            Self::Bind(error) => error.node(),
            Self::Resolve(error) => error.node(),
            Self::Analyze(error) => error.node(),
            Self::Elaborate(error) => error.node(),
            Self::Lower(error) => error.node(),
            Self::Verify(error) => error.node(),
            Self::Optimize(error) => error.node(),
            Self::Generate(error) => error.node(),
            Self::Link(error) => error.node(),
            Self::Internal(error) => error.node(),
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
            Self::Optimize(error) => error.message(program),
            Self::Generate(error) => error.message(program),
            Self::Link(error) => error.message(program),
            Self::Internal(error) => error.message(program),
        }
    }

    /// Get the full code of the error (e.g., `IE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("E{}{:03}", self.phase_letter(), self.sub_code())
    }
}
