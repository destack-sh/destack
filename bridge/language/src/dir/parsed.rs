use crate::{ArtifactVersion, ModuleId, bridge};

/// Typed projection of one parsed DIR artifact.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirParsed {
    /// Exact parsed artifact version.
    pub version: ArtifactVersion,
    /// Parsed module id.
    pub module: ModuleId,
}

impl DirParsed {
    /// Project one parsed DIR artifact.
    pub fn from_artifact(
        version: destack_artifact::ArtifactVersion,
        module: ModuleId,
        _parsed: &destack_artifact::DirParsed,
    ) -> Self {
        Self {
            version: ArtifactVersion::from_artifact(version),
            module,
        }
    }
}
