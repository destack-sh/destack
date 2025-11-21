use crate::{CompileError, CompilePhase};
use dyst_dir::{
    LocalNodeIdAny, LocalScopeId, LocalSymbolId, ModuleId, Session, StringId, Visibility,
};

/// Error when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ResolveError {
    /// Unsupported node.
    UnsupportedNode { node: LocalNodeIdAny },
    /// Dependent nodes are not ready to be resolved. May be retried.
    NotReady {
        module: ModuleId,
        node: LocalNodeIdAny,
        depends_on: Vec<LocalNodeIdAny>,
    },

    /// Circular dependency.
    CircularDependency {
        module: ModuleId,
        node: LocalNodeIdAny,
        depends_on: Vec<LocalNodeIdAny>,
    },

    /// Use of undeclared symbol.
    UndeclaredSymbol {
        module: ModuleId,
        node: LocalNodeIdAny,
        scope: LocalScopeId,
        name: StringId,
    },
    /// Use of missing symbol.
    MissingSymbol {
        module: ModuleId,
        node: LocalNodeIdAny,
        scope: LocalScopeId,
        name: StringId,
    },
    /// Use of ambiguous symbol.
    AmbiguousSymbol {
        module: ModuleId,
        node: LocalNodeIdAny,
        scope: LocalScopeId,
        symbol: LocalSymbolId,
        name: StringId,
    },
    /// Unresolved module.
    UnresolvedModule {
        module: ModuleId,
        node: LocalNodeIdAny,
        target: StringId,
    },
    /// Unresolved member.
    UnresolvedMember {
        module: ModuleId,
        node: LocalNodeIdAny,
        member: StringId,
    },
    /// Visibility violation (private/internal/module boundaries).
    InaccessibleSymbol {
        module: ModuleId,
        node: LocalNodeIdAny,
        visibility: Visibility,
        symbol: LocalSymbolId,
    },
    /// Conflicting declarations in the same scope.
    ConflictingDeclaration {
        module: ModuleId,
        node: LocalNodeIdAny,
        name: StringId,
    },
    /// Duplicate export name in the same module.
    DuplicateExport {
        module: ModuleId,
        node: LocalNodeIdAny,
        name: StringId,
    },
}

impl ResolveError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnsupportedNode { .. } => 1,
            Self::NotReady { .. } => 2,
            Self::CircularDependency { .. } => 3,
            Self::UndeclaredSymbol { .. } => 4,
            Self::MissingSymbol { .. } => 5,
            Self::AmbiguousSymbol { .. } => 6,
            Self::UnresolvedModule { .. } => 7,
            Self::UnresolvedMember { .. } => 8,
            Self::InaccessibleSymbol { .. } => 9,
            Self::ConflictingDeclaration { .. } => 10,
            Self::DuplicateExport { .. } => 11,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::UnsupportedNode { node, .. } => Some(*node),
            Self::NotReady { node, .. } => Some(*node),
            Self::CircularDependency { node, .. } => Some(*node),
            Self::UndeclaredSymbol { node, .. } => Some(*node),
            Self::MissingSymbol { node, .. } => Some(*node),
            Self::AmbiguousSymbol { node, .. } => Some(*node),
            Self::UnresolvedModule { node, .. } => Some(*node),
            Self::UnresolvedMember { node, .. } => Some(*node),
            Self::InaccessibleSymbol { node, .. } => Some(*node),
            Self::ConflictingDeclaration { node, .. } => Some(*node),
            Self::DuplicateExport { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
            Self::NotReady { .. } => "dependent nodes are not ready to be resolved".to_string(),
            Self::CircularDependency { .. } => "circular dependency".to_string(),
            Self::UndeclaredSymbol { .. } => "use of undeclared symbol".to_string(),
            Self::MissingSymbol { .. } => "missing symbol".to_string(),
            Self::AmbiguousSymbol { .. } => "ambiguous symbol".to_string(),
            Self::UnresolvedModule { .. } => "unresolved module".to_string(),
            Self::UnresolvedMember { .. } => "unresolved member".to_string(),
            Self::InaccessibleSymbol { .. } => "inaccessible symbol".to_string(),
            Self::ConflictingDeclaration { .. } => "conflicting declaration".to_string(),
            Self::DuplicateExport { .. } => "duplicate export".to_string(),
        }
    }
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolveError")
            .field(
                "code",
                &format!("{}E{:03}", CompilePhase::Resolve.letter(), self.sub_code()),
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
