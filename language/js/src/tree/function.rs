use crate::{Asynchrony, LocalNodeId, Parameter, Pattern};

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// The signature of a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FunctionSignature {
    /// The asynchrony of the function.
    pub asynchrony: Asynchrony,
    /// The runtime parameters of the function.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// The final rest parameter.
    pub rest: Option<LocalNodeId<Pattern>>,
    /// Whether the function is a generator.
    pub is_generator: bool,
}
