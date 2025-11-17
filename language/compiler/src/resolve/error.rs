use crate::{CompileError, CompilerStage};
use dyst_dir::{ModuleId, NodeIdAny, ScopeId, Session, StringId, SymbolId};

/// Error when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ResolveError {
    /// Dependent nodes are not ready to be resolved. May be retried.
    NotReady {
        module: ModuleId,
        node: NodeIdAny,
        depends_on: Vec<NodeIdAny>,
    } = 1,

    /// Circular dependency.
    CircularDependency {
        module: ModuleId,
        node: NodeIdAny,
        depends_on: Vec<NodeIdAny>,
    } = 2,

    /// Use of undeclared symbol.
    UndeclaredSymbol {
        module: ModuleId,
        node: NodeIdAny,
        scope: ScopeId,
        name: StringId,
    } = 3,
    /// Use of missing symbol.
    MissingSymbol {
        module: ModuleId,
        node: NodeIdAny,
        scope: ScopeId,
        name: StringId,
    } = 4,
    /// Use of ambiguous symbol.
    AmbiguousSymbol {
        module: ModuleId,
        node: NodeIdAny,
        scope: ScopeId,
        symbol: SymbolId,
        name: StringId,
    } = 5,
}

impl ResolveError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::NotReady { .. } => 1,
            Self::CircularDependency { .. } => 2,
            Self::UndeclaredSymbol { .. } => 3,
            Self::MissingSymbol { .. } => 4,
            Self::AmbiguousSymbol { .. } => 5,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<NodeIdAny> {
        match self {
            Self::NotReady { node, .. } => Some(*node),
            Self::CircularDependency { node, .. } => Some(*node),
            Self::UndeclaredSymbol { node, .. } => Some(*node),
            Self::MissingSymbol { node, .. } => Some(*node),
            Self::AmbiguousSymbol { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::NotReady { .. } => "dependent nodes are not ready to be resolved".to_string(),
            Self::CircularDependency { .. } => "circular dependency".to_string(),
            Self::UndeclaredSymbol { .. } => "use of undeclared symbol".to_string(),
            Self::MissingSymbol { .. } => "missing symbol".to_string(),
            Self::AmbiguousSymbol { .. } => "ambiguous symbol".to_string(),
        }
    }
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolveError")
            .field(
                "code",
                &format!("{}E{:03}", CompilerStage::Resolve.letter(), self.sub_code()),
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
