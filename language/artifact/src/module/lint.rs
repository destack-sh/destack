use serde::{Deserialize, Serialize};

/// The realized module lint surface for one module profile.
#[derive(destack_artifact_macros::Image, Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleLinted;

/// The realized package lint surface for one package.
#[derive(destack_artifact_macros::Image, Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageLinted;

/// The realized workspace lint surface.
#[derive(destack_artifact_macros::Image, Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceLinted;
