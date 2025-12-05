use destack_dir::{GlobalNodeIdAny};

use crate::{TaskPhase, TaskWarning};

use destack_workspace::Program;

/// Warning when linking something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LinkWarning {
    /// Missing target for a symbol.
    MissingTarget { node: GlobalNodeIdAny },
    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    WeakSymbol {
        node: GlobalNodeIdAny,
        symbol: String,
    },
    /// Large binary / large static data section.
    LargeBinary { node: GlobalNodeIdAny, size_mb: u64 },
}

impl LinkWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::MissingTarget { .. } => 1,
            Self::WeakSymbol { .. } => 2,
            Self::LargeBinary { .. } => 3,
        }
    }

    /// Get the node of the warning.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::MissingTarget { node, .. } => *node,
            Self::WeakSymbol { node, .. } => *node,
            Self::LargeBinary { node, .. } => *node,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::MissingTarget { .. } => "missing target for a symbol".to_string(),
            Self::WeakSymbol { .. } => "weak symbol".to_string(),
            Self::LargeBinary { .. } => "large binary".to_string(),
        }
    }
}

impl std::fmt::Display for LinkWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkWarning")
            .field(
                "code",
                &format!("W{}{:03}", TaskPhase::Link.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<LinkWarning> for TaskWarning {
    fn from(warning: LinkWarning) -> Self {
        TaskWarning::Link(warning)
    }
}
