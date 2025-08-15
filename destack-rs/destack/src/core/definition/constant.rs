//! destack.core.definition.constant@2025.08.15.1

#![destack::partial(destack.core.definition.constant, file)]

use crate::Value;

#[destack::generated(ConstantDefinition, struct, block)]
/// Definition of a builtin Constant.
pub struct ConstantDefinition {
    id: u8,
    name: String,
    description: String,
    taggings: Vec<u8>,
    value: Value,
    _is_deferred: bool
}