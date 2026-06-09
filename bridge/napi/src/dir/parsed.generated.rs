// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{ArtifactVersion, ModuleId};

/// Typed projection of one parsed DIR artifact.
#[derive(Debug)]
#[napi(object)]
pub struct DirParsed {
    /// Exact parsed artifact version.
    pub version: ArtifactVersion,
    /// Parsed module id.
    pub module: ModuleId,
}

impl DirParsed {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::DirParsed) -> Self {
        Self {
            version: ArtifactVersion::from_bridge(value.version),
            module: ModuleId::from_bridge(value.module),
        }
    }
}
