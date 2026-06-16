use serde::{Deserialize, Serialize};

use crate::{TypeId, TypedValue, Value};

/// One block or function parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    /// The SSA value.
    pub value: Value,
    /// The parameter type.
    pub ty: TypeId,
}

impl Parameter {
    /// Return this parameter as a typed value.
    #[inline]
    pub fn typed_value(self) -> TypedValue {
        TypedValue::new(self.value, self.ty)
    }
}
