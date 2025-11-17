use dyst_dir::{ModuleId, NodeIdAny, Session};

use crate::CompilerStage;

/// Warning when building something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum BuildWarning {
    /// Missing configuration for a file.
    MissingConfiguration { module: ModuleId } = 1,
}

impl BuildWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::MissingConfiguration { .. } => 1,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<NodeIdAny> {
        match self {
            Self::MissingConfiguration { .. } => None,
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::MissingConfiguration { .. } => "missing configuration for a file".to_string(),
        }
    }
}

impl std::fmt::Display for BuildWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildWarning")
            .field(
                "code",
                &format!("{}W{:03}", CompilerStage::Build.letter(), self.sub_code()),
            )
            .finish()
    }
}
