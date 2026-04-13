use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

/// A TypeUnaryOperator is a type unary operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum TypeUnaryOperator {
    /// Not `!T`.
    Not,
    /// Must 'T!'.
    Must,
    /// `newtype`
    Newtype,
    /// `type`
    Type,
    /// `readonly`
    Readonly,
    /// `typeof`
    Typeof,
    /// `keyof`
    Keyof,
    /// `as comptime`
    AsComptime,
}

/// A TypeBinaryOperator is a type binary operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum TypeBinaryOperator {
    /// `in`
    In,
    /// `is`
    Is,
    /// `instanceof`
    InstanceOf,
    /// `extends`
    Extends,
    /// `implements`
    Implements,
}
