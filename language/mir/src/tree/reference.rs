use serde::{Deserialize, Serialize};

use crate::{Block, Function, Global, Local, LocalNodeId, Type, Value};

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

/// One integer reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntegerReference {
    /// One concrete integer.
    Integer(i64),
    /// One required integer that was omitted.
    Missing,
    /// One malformed integer fragment.
    Error,
}

/// One parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    /// The SSA value.
    pub value: ValueReference,
    /// The parameter type.
    pub ty: TypeReference,
}
