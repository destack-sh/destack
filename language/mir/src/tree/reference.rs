use serde::{Deserialize, Serialize};

use crate::{Block, Function, Global, Local, LocalNodeId, Type, TypedValue, Value};

/// One value reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueReference {
    /// One concrete SSA value.
    Value(Value),
    /// One required value that was omitted.
    Missing,
    /// One malformed value fragment.
    Error,
}

impl From<Value> for ValueReference {
    fn from(value: Value) -> Self {
        Self::Value(value)
    }
}

impl From<&Value> for ValueReference {
    fn from(value: &Value) -> Self {
        Self::Value(*value)
    }
}

impl ValueReference {
    /// Return the concrete value when present.
    #[inline]
    pub fn value(self) -> Option<Value> {
        match self {
            Self::Value(value) => Some(value),
            Self::Missing | Self::Error => None,
        }
    }

    /// Replace this reference when it points at one value.
    #[inline]
    pub fn replace_value(&mut self, from: Value, to: Value) {
        if *self == Self::Value(from) {
            *self = Self::Value(to);
        }
    }
}

/// One type reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeReference {
    /// One concrete type.
    Type(LocalNodeId<Type>),
    /// One required type that was omitted.
    Missing,
    /// One malformed type fragment.
    Error,
}

impl From<LocalNodeId<Type>> for TypeReference {
    fn from(ty: LocalNodeId<Type>) -> Self {
        Self::Type(ty)
    }
}

impl From<&LocalNodeId<Type>> for TypeReference {
    fn from(ty: &LocalNodeId<Type>) -> Self {
        Self::Type(*ty)
    }
}

impl TypeReference {
    /// Return the concrete type when present.
    #[inline]
    pub fn ty(self) -> Option<LocalNodeId<Type>> {
        match self {
            Self::Type(ty) => Some(ty),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One block reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockReference {
    /// One concrete block.
    Block(LocalNodeId<Block>),
    /// One required block that was omitted.
    Missing,
    /// One malformed block fragment.
    Error,
}

impl From<LocalNodeId<Block>> for BlockReference {
    fn from(block: LocalNodeId<Block>) -> Self {
        Self::Block(block)
    }
}

impl From<&LocalNodeId<Block>> for BlockReference {
    fn from(block: &LocalNodeId<Block>) -> Self {
        Self::Block(*block)
    }
}

impl BlockReference {
    /// Return the concrete block when present.
    #[inline]
    pub fn block(self) -> Option<LocalNodeId<Block>> {
        match self {
            Self::Block(block) => Some(block),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One function reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionReference {
    /// One concrete function.
    Function(LocalNodeId<Function>),
    /// One required function that was omitted.
    Missing,
    /// One malformed function fragment.
    Error,
}

impl From<LocalNodeId<Function>> for FunctionReference {
    fn from(function: LocalNodeId<Function>) -> Self {
        Self::Function(function)
    }
}

impl From<&LocalNodeId<Function>> for FunctionReference {
    fn from(function: &LocalNodeId<Function>) -> Self {
        Self::Function(*function)
    }
}

impl FunctionReference {
    /// Return the concrete function when present.
    #[inline]
    pub fn function(self) -> Option<LocalNodeId<Function>> {
        match self {
            Self::Function(function) => Some(function),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One local reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocalReference {
    /// One concrete local.
    Local(LocalNodeId<Local>),
    /// One required local that was omitted.
    Missing,
    /// One malformed local fragment.
    Error,
}

impl From<LocalNodeId<Local>> for LocalReference {
    fn from(local: LocalNodeId<Local>) -> Self {
        Self::Local(local)
    }
}

impl From<&LocalNodeId<Local>> for LocalReference {
    fn from(local: &LocalNodeId<Local>) -> Self {
        Self::Local(*local)
    }
}

impl LocalReference {
    /// Return the concrete local when present.
    #[inline]
    pub fn local(self) -> Option<LocalNodeId<Local>> {
        match self {
            Self::Local(local) => Some(local),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One global reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GlobalReference {
    /// One concrete global.
    Global(LocalNodeId<Global>),
    /// One required global that was omitted.
    Missing,
    /// One malformed global fragment.
    Error,
}

impl From<LocalNodeId<Global>> for GlobalReference {
    fn from(global: LocalNodeId<Global>) -> Self {
        Self::Global(global)
    }
}

impl From<&LocalNodeId<Global>> for GlobalReference {
    fn from(global: &LocalNodeId<Global>) -> Self {
        Self::Global(*global)
    }
}

impl From<i128> for IntegerReference {
    fn from(value: i128) -> Self {
        Self::Integer(value)
    }
}

impl From<&i128> for IntegerReference {
    fn from(value: &i128) -> Self {
        Self::Integer(*value)
    }
}

impl GlobalReference {
    /// Return the concrete global when present.
    #[inline]
    pub fn global(self) -> Option<LocalNodeId<Global>> {
        match self {
            Self::Global(global) => Some(global),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One integer reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntegerReference {
    /// One concrete integer.
    Integer(i128),
    /// One required integer that was omitted.
    Missing,
    /// One malformed integer fragment.
    Error,
}

impl IntegerReference {
    /// Return the concrete integer when present.
    #[inline]
    pub fn integer(self) -> Option<i128> {
        match self {
            Self::Integer(value) => Some(value),
            Self::Missing | Self::Error => None,
        }
    }
}

/// One parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    /// The SSA value.
    pub value: ValueReference,
    /// The parameter type.
    pub ty: TypeReference,
}

impl Parameter {
    /// Return the concrete typed value when present.
    #[inline]
    pub fn typed_value(self) -> Option<TypedValue> {
        let value = self.value.value()?;
        let ty = self.ty.ty()?;

        Some(TypedValue::new(value, ty))
    }
}
