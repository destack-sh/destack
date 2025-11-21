use dyst_dir::{LocalNodeIdAny, Session};

use crate::{CompilePhase, CompileWarning};

/// Warning when linking something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LinkWarning {
    /// Missing target for a symbol.
    MissingTarget { node: LocalNodeIdAny },
    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    WeakSymbol {
        node: LocalNodeIdAny,
        symbol: String,
    },
    /// Large binary / large static data section.
    LargeBinary { node: LocalNodeIdAny, size_mb: u64 },
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

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::MissingTarget { node, .. } => Some(*node),
            Self::WeakSymbol { node, .. } => Some(*node),
            Self::LargeBinary { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
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
                &format!("{}W{:03}", CompilePhase::Link.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<LinkWarning> for CompileWarning {
    fn from(warning: LinkWarning) -> Self {
        CompileWarning::Link(warning)
    }
}
