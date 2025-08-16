//! destack.core.definition.property@2025.08.15.1

#![destack::partial(destack.core.definition.property, file)]

use crate::{ReferenceType, Type, Value, ValueFactory};

#[destack::generated(PropertyDefinition, -, block)]
/// Definition of a builtin Property.
pub struct PropertyDefinition {
    pub id: u8,
    pub r#type: Type,
    pub name: String,
    pub description: String,
    pub tag: Option<u8>,
    pub default_value: Option<Value>,
    pub default_factory: Option<ValueFactory>,
    pub reference_type: Option<ReferenceType>,
    pub is_readonly: bool,
    pub is_repr: bool,
    pub is_hash: bool,
    pub is_eq: bool,
    pub is_managed: bool,
    pub is_static: bool,
    pub is_runtime_only: bool,
    pub is_interned: bool,
}
