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

impl IntType {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            IntType::Int8 => "int8",
            IntType::Int16 => "int16",
            IntType::Int32 => "int32",
            IntType::Int64 => "int64",
            IntType::Int128 => "int128",
        }
    }
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

impl UintType {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            UintType::Uint8 => "uint8",
            UintType::Uint16 => "uint16",
            UintType::Uint32 => "uint32",
            UintType::Uint64 => "uint64",
            UintType::Uint128 => "uint128",
        }
    }
}

/// A FloatType is a floating-point type.
#[derive(Debug, Clone, PartialEq)]
pub enum FloatType {
    Float32,
    Float64,
}

impl FloatType {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            FloatType::Float32 => "float32",
            FloatType::Float64 => "float64",
        }
    }
}

impl<'a> Parser<'a> {}
