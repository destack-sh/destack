/// An IntType is a signed integer type.
#[derive(Debug, Clone, PartialEq)]
pub enum IntType {
    /// The size of the pointer type.
    Isize,
    I8,
    I16,
    I32,
    I64,
    I128,
}

/// A UintType is an unsigned integer type.
#[derive(Debug, Clone, PartialEq)]
pub enum UintType {
    /// The size of the pointer type.
    Usize,
    U8,
    U16,
    U32,
    U64,
    U128,
}

/// A FloatType is a floating-point type.
#[derive(Debug, Clone, PartialEq)]
pub enum FloatType {
    F32,
    F64,
}
