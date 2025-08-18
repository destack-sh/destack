//! destack.core.common.value

#![destack::generated(destack.core.common.value, file)]

use crate::{NamedValue, Value};

#[destack::generated(Value, Debug, block)]
impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value")
    }
}

#[destack::generated(NamedValue, Debug, block)]
impl std::fmt::Debug for NamedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NamedValue")
    }
}
