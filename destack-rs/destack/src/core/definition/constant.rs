//! destack.core.definition.constant@2025.08.15.1

#![destack::partial(destack.core.definition.constant, file)]

use crate::Value;

#[destack::generated(ConstantDefinition, -, block)]
/// Definition of a builtin Constant.
pub struct ConstantDefinition {
    pub id: u8,
    pub name: String,
    pub description: String,
    pub taggings: Vec<u8>,
    pub value: Value,
    pub _is_deferred: bool,
}
