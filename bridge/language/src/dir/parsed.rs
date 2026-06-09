use destack_artifact as artifact;

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
        version: artifact::ArtifactVersion,
        module: ModuleId,
        _parsed: &artifact::DirParsed,
    ) -> Self {
        Self {
            version: ArtifactVersion::from_artifact(version),
            module,
        }
    }
}
