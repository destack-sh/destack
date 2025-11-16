use dyst_dir::NodeIdAny;

use crate::CompileError;

/// Error when building something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum BuildError {
    /// Building is impossible for this node.
    BuildingImpossible { node_id: NodeIdAny } = 1,
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildError")
            .field("code", &format!("BE{:03}", self.sub_code()))
            .finish()
    }
}

pub type BuildResult<T> = Result<T, BuildError>;

impl From<BuildError> for CompileError {
    #[inline]
    fn from(error: BuildError) -> Self {
        CompileError::Build(error)
    }
}

impl BuildError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::BuildingImpossible { .. } => 1,
        }
    }

    /// Get the message of the error.
    pub fn message(&self) -> &'static str {
        match self {
            Self::BuildingImpossible { .. } => "building is impossible",
        }
    }
}

/// Warning when building something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum BuildWarning {
    /// Building is impossible for this node.
    BuildingImpossible { node: NodeIdAny } = 1,
}

impl BuildWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::BuildingImpossible { .. } => 1,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self) -> &'static str {
        match self {
            Self::BuildingImpossible { .. } => "building is impossible for this node",
        }
    }
}

impl std::fmt::Display for BuildWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildWarning")
            .field("code", &format!("BW{:03}", self.sub_code()))
            .finish()
    }
}
