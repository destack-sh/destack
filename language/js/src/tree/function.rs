use crate::{Asynchrony, Keyword, LocalNodeId, Parameter};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// The role of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum FunctionRole {
    /// Getter method.
    Getter,
    /// Setter method.
    Setter,
}

impl FunctionRole {
    /// Return the JavaScript keyword for this role.
    #[inline]
    pub fn keyword(self) -> Keyword {
        match self {
            Self::Getter => Keyword::Get,
            Self::Setter => Keyword::Set,
        }
    }
}

/// The signature of a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FunctionSignature {
    /// The asynchrony of the function.
    pub asynchrony: Asynchrony,
    /// The runtime parameters of the function.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// Whether the function is a generator.
    pub is_generator: bool,
}
