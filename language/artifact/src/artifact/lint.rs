use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// The realized module lint surface for one module profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct ModuleLinted;

/// The realized package lint surface for one package.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct PackageLinted;

/// The realized workspace lint surface.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct WorkspaceLinted;
