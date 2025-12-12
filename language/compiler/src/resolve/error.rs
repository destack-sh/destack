use crate::{DiagnosticAnchor, TaskDependency, TaskDependencyError, TaskError, TaskPhase};
use destack_dir::{GlobalNodeIdAny, GlobalScopeId, GlobalSymbolId, StaticKey, StringId};
use destack_source::ModuleId;
use destack_workspace::Program;

/// Error when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ResolveError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    UnsatisfiedDependency { dependency: TaskDependency },
    /// Unsupported node.
    UnsupportedConstruct { node: GlobalNodeIdAny },
    /// Circular dependency.
    CircularDependency {
        node: GlobalNodeIdAny,
        depends_on: Vec<GlobalNodeIdAny>,
    },
    /// Use of undeclared symbol.
    UndeclaredSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        key: StaticKey,
    },
    /// Use of missing symbol.
    MissingSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        via_module: Option<ModuleId>,
        key: StaticKey,
    },
    /// Use of ambiguous symbol.
    AmbiguousSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        key: StaticKey,
    },
    /// Cyclic symbol reference (re-export chain forms a cycle).
    CyclicSymbol {
        node: GlobalNodeIdAny,
        symbol: GlobalSymbolId,
    },
    /// Unresolved module.
    UnresolvedModule {
        node: GlobalNodeIdAny,
        target: StringId,
    },
    /// Self type used outside of a type context.
    MissingSelf { node: GlobalNodeIdAny },
    /// Missing target for a control flow expression.
    MissingTarget {
        node: GlobalNodeIdAny,
        target: Option<StringId>,
    },
    /// Invalid target for a control flow expression.
    InvalidTarget {
        node: GlobalNodeIdAny,
        target: Option<StringId>,
        target_node: GlobalNodeIdAny,
    },
}

impl From<TaskDependencyError> for ResolveError {
    fn from(e: TaskDependencyError) -> Self {
        match e {
            TaskDependencyError::NotReady { dependency } => Self::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                Self::UnsatisfiedDependency { dependency }
            }
        }
    }
}

impl TryFrom<ResolveError> for TaskDependency {
    type Error = ResolveError;

    fn try_from(error: ResolveError) -> Result<Self, Self::Error> {
        match error {
            ResolveError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl ResolveError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::UnsatisfiedDependency { .. } => 1,
            Self::UnsupportedConstruct { .. } => 2,
            Self::CircularDependency { .. } => 3,
            Self::UndeclaredSymbol { .. } => 4,
            Self::MissingSymbol { .. } => 5,
            Self::AmbiguousSymbol { .. } => 6,
            Self::CyclicSymbol { .. } => 8,
            Self::UnresolvedModule { .. } => 7,
            Self::MissingSelf { .. } => 9,
            Self::MissingTarget { .. } => 10,
            Self::InvalidTarget { .. } => 11,
        }
    }

    /// Get the anchor of the error.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Yield { dependency } => dependency.anchor(),
            Self::UnsatisfiedDependency { dependency } => dependency.anchor(),
            Self::UnsupportedConstruct { node, .. } => DiagnosticAnchor::Node(*node),
            Self::CircularDependency { node, .. } => DiagnosticAnchor::Node(*node),
            Self::UndeclaredSymbol { node, .. } => DiagnosticAnchor::Node(*node),
            Self::MissingSymbol { node, .. } => DiagnosticAnchor::Node(*node),
            Self::AmbiguousSymbol { node, .. } => DiagnosticAnchor::Node(*node),
            Self::CyclicSymbol { node, .. } => DiagnosticAnchor::Node(*node),
            Self::UnresolvedModule { node, .. } => DiagnosticAnchor::Node(*node),
            Self::MissingSelf { node, .. } => DiagnosticAnchor::Node(*node),
            Self::MissingTarget { node, .. } => DiagnosticAnchor::Node(*node),
            Self::InvalidTarget { node, .. } => DiagnosticAnchor::Node(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::UnsatisfiedDependency { .. } => "unsatisfied dependency".to_string(),
            Self::UnsupportedConstruct { node, .. } => {
                format!("unsupported {}", node.local_id.ty.name())
            }
            Self::CircularDependency { .. } => "circular dependency".to_string(),
            Self::UndeclaredSymbol { key, .. } => {
                let key = key.debug_string(&program.strings);
                format!("missing symbol {key}")
            }
            Self::MissingSymbol {
                key, via_module, ..
            } => {
                let key = key.debug_string(&program.strings);
                if let &Some(via_module) = via_module {
                    let via_module = program.modules.get(via_module).read().uri.to_string();
                    format!("missing symbol {key} in '{via_module}'")
                } else {
                    format!("missing symbol {key}")
                }
            }
            Self::AmbiguousSymbol { key, .. } => {
                let key = key.debug_string(&program.strings);
                format!("ambiguous symbol {key}")
            }
            Self::CyclicSymbol { .. } => "cyclic symbol reference".to_string(),
            Self::UnresolvedModule { target, .. } => {
                let target = program.strings.get(*target).to_string();
                format!("unresolved module '{target}'")
            }
            Self::MissingSelf { .. } => {
                "`Self` type can only be used inside a class, struct, or enum".to_string()
            }
            Self::MissingTarget { target, .. } => {
                let target = target
                    .map(|t| program.strings.get(t).to_string())
                    .unwrap_or_default();
                format!("missing target {target}")
            }
            Self::InvalidTarget {
                target,
                target_node: _,
                ..
            } => {
                let target = target
                    .map(|t| program.strings.get(t).to_string())
                    .unwrap_or_default();
                format!("invalid target {target}")
            }
        }
    }
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolveError")
            .field(
                "code",
                &format!("E{}{:03}", TaskPhase::Resolve.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ResolveError> for TaskError {
    #[inline]
    fn from(error: ResolveError) -> Self {
        TaskError::Resolve(error)
    }
}

pub type ResolveResult<T> = Result<T, ResolveError>;
