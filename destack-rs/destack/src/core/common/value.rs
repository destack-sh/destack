//! destack.core.common.value

#![destack::partial(destack.core.common.value, file)]

use crate::Type;

#[destack::generated(Value, -, block)]
/// A generic Value of any Type.
/// Values are used to represent any generic data.
pub struct Value {
    pub r#type: Type,
    pub value: Option<()>,
}

#[destack::generated(NamedValue, -, block)]
/// A named Value.
pub struct NamedValue {
    pub name: String,
    pub value: Value,
}
