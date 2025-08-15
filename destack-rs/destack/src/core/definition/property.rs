//! destack.core.definition.property@2025.08.15.1

#![destack::partial(destack.core.definition.property, file)]

use crate::ReferenceType;
use crate::Type;
use crate::Value;
use crate::ValueFactory;

#[destack::generated(PropertyDefinition, , block)]
/// Definition of a builtin Property.
pub struct PropertyDefinition {
    id: u8,
    r#type: Type,
    name: String,
    description: String,
    tag: Option<u8>,
    default_value: Option<Value>,
    default_factory: Option<ValueFactory>,
    reference_type: Option<ReferenceType>,
    is_readonly: bool,
    is_repr: bool,
    is_hash: bool,
    is_eq: bool,
    is_managed: bool,
    is_static: bool,
    is_runtime_only: bool,
    is_interned: bool,
}
