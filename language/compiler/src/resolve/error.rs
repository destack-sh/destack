use crate::{CompileError, CompilePhase, CompileTask};
use dyst_dir::{GlobalNodeIdAny, GlobalScopeId, GlobalSymbolId, Program, StringId, SymbolKey};

/// Error when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ResolveError {
    /// Wait for other tasks.
    Wait {
        nodes: Vec<GlobalNodeIdAny>,
        tasks: Vec<CompileTask>,
    },

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
        scope: GlobalScopeId,
        target: StringId,
    },
    /// Conflicting declarations in the same scope.
    ConflictingDeclaration {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        name: StringId,
    },
    /// Duplicate export name in the same module.
    DuplicateExport {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        name: StringId,
    },
}

impl ResolveError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Wait { .. } => 0,
            Self::UnsupportedNode { .. } => 2,
            Self::CircularDependency { .. } => 2,
            Self::UndeclaredSymbol { .. } => 3,
            Self::MissingSymbol { .. } => 4,
            Self::AmbiguousSymbol { .. } => 5,
            Self::UnresolvedModule { .. } => 7,
            Self::ConflictingDeclaration { .. } => 8,
            Self::DuplicateExport { .. } => 9,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Wait { nodes, .. } => nodes.first().copied(),
            Self::UnsupportedNode { node, .. } => Some(*node),
            Self::CircularDependency { node, .. } => Some(*node),
            Self::UndeclaredSymbol { node, .. } => Some(*node),
            Self::MissingSymbol { node, .. } => Some(*node),
            Self::AmbiguousSymbol { node, .. } => Some(*node),
            Self::UnresolvedModule { node, .. } => Some(*node),
            Self::ConflictingDeclaration { node, .. } => Some(*node),
            Self::DuplicateExport { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, program: &'a Program<'a>) -> String {
        match self {
            Self::Wait { tasks, .. } => {
                format!("wait for {} tasks", tasks.len())
            }
            Self::UnsupportedNode { node, .. } => {
                format!("unsupported {}", node.local_id.ty.name())
            }
            Self::CircularDependency { .. } => "circular dependency".to_string(),
            Self::UndeclaredSymbol { key, .. } => {
                let key = key.debug_string(program);
                format!("undeclared symbol {key}")
            }
            Self::MissingSymbol { key, .. } => {
                let key = key.debug_string(program);
                format!("missing symbol {key}")
            }
            Self::AmbiguousSymbol { key, .. } => {
                let key = key.debug_string(program);
                format!("ambiguous symbol {key}")
            }
            Self::UnresolvedModule { target, .. } => {
                let target = program.strings.get(*target).to_string();
                format!("unresolved module '{target}'")
            }
            Self::ConflictingDeclaration { name, .. } => {
                let name = program.strings.get(*name).to_string();
                format!("conflicting declaration {name}")
            }
            Self::DuplicateExport { name, .. } => {
                let name = program.strings.get(*name).to_string();
                format!("duplicate export {name}")
            }
        }
    }
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolveError")
            .field(
                "code",
                &format!("E{}{:03}", CompilePhase::Resolve.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ResolveError> for CompileError {
    #[inline]
    fn from(error: ResolveError) -> Self {
        CompileError::Resolve(error)
    }
}

pub type ResolveResult<T> = Result<T, ResolveError>;
