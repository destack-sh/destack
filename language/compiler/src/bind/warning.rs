use dyst_dir::{LocalNodeIdAny, Session};

use crate::{CompileStage, CompileWarning};

/// Warning when binding something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum BindWarning {
    /// Use of deprecated target / CPU / ABI.
    DeprecatedTarget { node: LocalNodeIdAny },
    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    WeakSymbol {
        node: LocalNodeIdAny,
        symbol: String,
    },
    /// Large binary / large static data section ("binary size exceeded X MB").
    LargeBinary { node: LocalNodeIdAny, size_mb: u64 },
}

impl BindWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::DeprecatedTarget { .. } => 1,
            Self::WeakSymbol { .. } => 2,
            Self::LargeBinary { .. } => 3,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::DeprecatedTarget { node, .. } => Some(*node),
            Self::WeakSymbol { node, .. } => Some(*node),
            Self::LargeBinary { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::DeprecatedTarget { .. } => "deprecated target".to_string(),
            Self::WeakSymbol { .. } => "weak symbol".to_string(),
            Self::LargeBinary { .. } => "large binary".to_string(),
        }
    }
}

impl std::fmt::Display for BindWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BindWarning")
            .field(
                "code",
                &format!("{}W{:03}", CompileStage::Bind.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<BindWarning> for CompileWarning {
    fn from(warning: BindWarning) -> Self {
        CompileWarning::Bind(warning)
    }
}
