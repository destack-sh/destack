mod ast;
mod dir;
mod regex;

pub use ast::*;
pub use dir::*;
pub use regex::*;

/// Represents a constant value that lints can reason about.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstValue {
    /// A boolean literal.
    Boolean(bool),
    /// An integer literal.
    Integer(i64),
    /// A bigint literal.
    Bigint(i64),
    /// A float literal.
    Float(f64),
    /// A null literal.
    Null,
    /// An undefined literal.
    Undefined,
}

impl ConstValue {
    /// Convert the value into boolean semantics used by the linter.
    pub fn to_bool(self) -> bool {
        match self {
            ConstValue::Boolean(value) => value,
            ConstValue::Integer(value) => value != 0,
            ConstValue::Bigint(value) => value != 0,
            ConstValue::Float(value) => value != 0.0,
            ConstValue::Null | ConstValue::Undefined => false,
        }
    }
}
