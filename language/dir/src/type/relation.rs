use serde::{Deserialize, Serialize};

use crate::{LocalInstantiationId, LocalTypeId};

/// One solved type relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeRelation {
    /// The related type after static evaluation.
    pub ty: LocalTypeId,
    /// The instantiation selected for the relationship, when one exists.
    pub instantiation: Option<LocalInstantiationId>,
}

impl TypeRelation {
    /// Create a solved type relationship.
    pub fn new(ty: LocalTypeId, instantiation: Option<LocalInstantiationId>) -> Self {
        Self { ty, instantiation }
    }
}
