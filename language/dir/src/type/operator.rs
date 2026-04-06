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
    /// `as const`
    AsConst,
}

/// A TypeBinaryOperator is a type binary operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum TypeBinaryOperator {
    /// `as`
    Cast,
    /// `in`
    In,
    /// `is`
    Is,
    /// `instanceof`
    InstanceOf,
    /// `satisfies`
    Satisfies,
    /// `extends`
    Extends,
    /// `implements`
    Implements,
}
