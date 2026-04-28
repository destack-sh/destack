use serde::{Deserialize, Serialize};

/// The realized module lint surface for one module profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleLinted;

/// The realized package lint surface for one package.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageLinted;

/// The realized workspace lint surface.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceLinted;
