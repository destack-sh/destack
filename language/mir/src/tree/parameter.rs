use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{TypeId, TypedValue, Value};

/// Typed SSA parameter behavior shared by function and block parameters.
pub trait TypedParameter {
    /// Return the SSA value.
    fn value(&self) -> Value;

    /// Return the parameter type.
    fn ty(&self) -> TypeId;

    /// Return this parameter as a typed value.
    #[inline]
    fn typed_value(&self) -> TypedValue {
        TypedValue::new(self.value(), self.ty())
    }
}

/// One function entry parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionParameter {
    /// The SSA value.
    pub value: Value,
    /// The parameter type.
    pub ty: TypeId,
}

impl FunctionParameter {
    /// Create a function parameter.
    #[inline]
    pub fn new(value: Value, ty: TypeId) -> Self {
        Self { value, ty }
    }

    /// Return this parameter as a typed value.
    #[inline]
    pub fn typed_value(&self) -> TypedValue {
        TypedParameter::typed_value(self)
    }

    /// Return the matching entry block parameter.
    #[inline]
    pub fn block_parameter(&self) -> BlockParameter {
        BlockParameter {
            value: self.value,
            ty: self.ty,
        }
    }

    /// Return the matching callable signature parameter.
    #[inline]
    pub fn signature_parameter(&self) -> SignatureParameter {
        SignatureParameter { ty: self.ty }
    }
}

impl TypedParameter for FunctionParameter {
    #[inline]
    fn value(&self) -> Value {
        self.value
    }

    #[inline]
    fn ty(&self) -> TypeId {
        self.ty
    }
}

/// One block parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct BlockParameter {
    /// The SSA value.
    pub value: Value,
    /// The parameter type.
    pub ty: TypeId,
}

impl BlockParameter {
    /// Return this parameter as a typed value.
    #[inline]
    pub fn typed_value(&self) -> TypedValue {
        TypedParameter::typed_value(self)
    }
}

impl TypedParameter for BlockParameter {
    #[inline]
    fn value(&self) -> Value {
        self.value
    }

    #[inline]
    fn ty(&self) -> TypeId {
        self.ty
    }
}

/// One callable signature parameter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SignatureParameter {
    /// The parameter type.
    pub ty: TypeId,
}

impl SignatureParameter {
    /// Create a signature parameter.
    #[inline]
    pub fn new(ty: TypeId) -> Self {
        Self { ty }
    }
}

impl From<TypeId> for SignatureParameter {
    #[inline]
    fn from(ty: TypeId) -> Self {
        Self::new(ty)
    }
}
