use crate::{Phase, TaskDependency, TaskError};
use destack_dir::{
    GlobalNodeIdAny, GlobalScopeId, GlobalSymbolId, ModuleId, Program, StringId, SymbolKey,
};

/// Error when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ResolveError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    YieldFailed { dependency: TaskDependency },
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
    /// Circular dependency.
    CircularDependency {
        node: GlobalNodeIdAny,
        depends_on: Vec<GlobalNodeIdAny>,
    },
    /// Use of undeclared symbol.
    UndeclaredSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        key: SymbolKey,
    },
    /// Use of missing symbol.
    MissingSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        via_module: Option<ModuleId>,
        key: SymbolKey,
    },
    /// Use of ambiguous symbol.
    AmbiguousSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        key: SymbolKey,
    },
    /// Unresolved module.
    UnresolvedModule {
        node: GlobalNodeIdAny,
        target: StringId,
    },
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
            Self::YieldFailed { .. } => 1,
            Self::UnsupportedNode { .. } => 2,
            Self::CircularDependency { .. } => 3,
            Self::UndeclaredSymbol { .. } => 4,
            Self::MissingSymbol { .. } => 5,
            Self::AmbiguousSymbol { .. } => 6,
            Self::UnresolvedModule { .. } => 7,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::YieldFailed { dependency } => dependency.node(),
            Self::UnsupportedNode { node, .. } => *node,
            Self::CircularDependency { node, .. } => *node,
            Self::UndeclaredSymbol { node, .. } => *node,
            Self::MissingSymbol { node, .. } => *node,
            Self::AmbiguousSymbol { node, .. } => *node,
            Self::UnresolvedModule { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::YieldFailed { .. } => "unsatisfied dependency".to_string(),
            Self::UnsupportedNode { node, .. } => {
                format!("unsupported {}", node.local_id.ty.name())
            }
            Self::CircularDependency { .. } => "circular dependency".to_string(),
            Self::UndeclaredSymbol { key, .. } => {
                let key = key.debug_string(program);
                format!("missing symbol {key}")
            }
            Self::MissingSymbol {
                key, via_module, ..
            } => {
                let key = key.debug_string(program);
                if let &Some(via_module) = via_module {
                    let via_module = program.modules.get(via_module).read().uri.to_string();
                    format!("missing symbol {key} in '{via_module}'")
                } else {
                    format!("missing symbol {key}")
                }
            }
            Self::AmbiguousSymbol { key, .. } => {
                let key = key.debug_string(program);
                format!("ambiguous symbol {key}")
            }
            Self::UnresolvedModule { target, .. } => {
                let target = program.strings.get(*target).to_string();
                format!("unresolved module '{target}'")
            }
        }
    }
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolveError")
            .field(
                "code",
                &format!("E{}{:03}", Phase::Resolve.letter(), self.sub_code()),
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
