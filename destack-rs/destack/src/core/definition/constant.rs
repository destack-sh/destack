//! destack.core.definition.constant

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
