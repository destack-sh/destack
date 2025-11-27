use dyst_dir::{GlobalNodeIdAny, ModuleId, Program};

use crate::Phase;

/// Warning when importing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ImportWarning {
    /// Huge file.
    VeryLargeFile {
        node: GlobalNodeIdAny,
        module: ModuleId,
        len: usize,
    },
}

impl ImportWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::VeryLargeFile { .. } => 1,
        }
    }

    /// Get the node of the warning.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::VeryLargeFile { node, .. } => *node,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::VeryLargeFile { len, .. } => format!("very large file ({len} bytes)"),
        }
    }
}

impl std::fmt::Display for ImportWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportWarning")
            .field(
                "code",
                &format!("W{}{:03}", Phase::Import.letter(), self.sub_code()),
            )
            .finish()
    }
}
