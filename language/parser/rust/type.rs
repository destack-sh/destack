use crate::Parser;

/// An IntType is a signed integer type.
#[derive(Debug, Clone, PartialEq)]
pub enum IntType {
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
}

/// A UintType is an unsigned integer type.
#[derive(Debug, Clone, PartialEq)]
pub enum UintType {
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Uint128,
}

/// A FloatType is a floating-point type.
#[derive(Debug, Clone, PartialEq)]
pub enum FloatType {
    Float32,
    Float64,
}

impl Parser {}
