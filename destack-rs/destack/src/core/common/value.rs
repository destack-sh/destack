//! destack.core.common.value@2025.08.15.1

#![destack::partial(destack.core.common.value, file)]

use crate::Type;

#[destack::generated(Value, , block)]
/// A generic Value of any Type.
/// Values are used to represent any generic data.
pub struct Value {
    r#type: Type,
    value: Option<() /* TODO */>,
}

#[destack::generated(NamedValue, , block)]
/// A named Value.
pub struct NamedValue {
    name: String,
    value: Value,
}
