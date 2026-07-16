use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Completed lint analysis for one module in one target.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct ModuleLinted;

/// Completed lint analysis for one target program.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct ProgramLinted;
