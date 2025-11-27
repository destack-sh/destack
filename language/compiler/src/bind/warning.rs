use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{CompileWarning, Phase};

/// Warning when binding something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum BindWarning {
    /// Use of deprecated target / CPU / ABI.
    DeprecatedTarget { node: GlobalNodeIdAny },
    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    WeakSymbol {
        node: GlobalNodeIdAny,
        symbol: String,
    },
    /// Large binary / large static data section ("binary size exceeded X MB").
    LargeBinary { node: GlobalNodeIdAny, size_mb: u64 },
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

    /// Get the node of the warning.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::DeprecatedTarget { node, .. } => *node,
            Self::WeakSymbol { node, .. } => *node,
            Self::LargeBinary { node, .. } => *node,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
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
                &format!("W{}{:03}", Phase::Bind.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<BindWarning> for CompileWarning {
    fn from(warning: BindWarning) -> Self {
        CompileWarning::Bind(warning)
    }
}
