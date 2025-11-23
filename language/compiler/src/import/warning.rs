use dyst_dir::{GlobalNodeIdAny, ModuleId, Program};

use crate::CompilePhase;

/// Warning when importing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ImportWarning {
    /// Huge file.
    HugeFile { module: ModuleId, len: usize },
}

impl ImportWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::HugeFile { .. } => 1,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::HugeFile { .. } => None,
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            Self::HugeFile { .. } => "huge file".to_string(),
        }
    }
}

impl std::fmt::Display for ImportWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportWarning")
            .field(
                "code",
                &format!("{}W{:03}", CompilePhase::Import.letter(), self.sub_code()),
            )
            .finish()
    }
}
