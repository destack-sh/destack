use std::str::FromStr;

use crate::{Node, NodeId, NodeType};

impl IntType {
    /// 8-bit signed integer
    pub const INT8: IntType = IntType {
        width: 8,
        is_signed: true,
    };
    /// 16-bit signed integer
    pub const INT16: IntType = IntType {
        width: 16,
        is_signed: true,
    };
    /// 32-bit signed integer
    pub const INT32: IntType = IntType {
        width: 32,
        is_signed: true,
    };
    /// 64-bit signed integer
    pub const INT64: IntType = IntType {
        width: 64,
        is_signed: true,
    };
    /// 128-bit signed integer
    pub const INT128: IntType = IntType {
        width: 128,
        is_signed: true,
    };

    /// 8-bit unsigned integer
    pub const UINT8: IntType = IntType {
        width: 8,
        is_signed: false,
    };
    /// 16-bit unsigned integer
    pub const UINT16: IntType = IntType {
        width: 16,
        is_signed: false,
    };
    /// 32-bit unsigned integer
    pub const UINT32: IntType = IntType {
        width: 32,
        is_signed: false,
    };
    /// 64-bit unsigned integer
    pub const UINT64: IntType = IntType {
        width: 64,
        is_signed: false,
    };
    /// 128-bit unsigned integer
    pub const UINT128: IntType = IntType {
        width: 128,
        is_signed: false,
    };
}

impl IntType {
    #[inline]
    pub fn as_str(self) -> String {
        let mut as_str = if self.is_signed {
            "int".to_string()
        } else {
            "uint".to_string()
        };
        as_str.push_str(&self.width.to_string());
        as_str
    }
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

impl FromStr for FloatType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "float32" => Ok(FloatType::Float32),
            "float64" => Ok(FloatType::Float64),
            _ => Err(()),
        }
    }
}

/// An (unresolved) Type declaration node.
///
/// Type references don't support static evaluation directly for simplicity.
/// They can refer to Paths that are themselves any static Expressions
///  (which enables the same feature set in a more structured way).
///
/// Examples:
/// ```
/// void
/// null
/// int32
/// boolean
/// boolean | *int32
/// float32[]
/// float64[3]
/// (int32, int32)
/// &T // reference to T
/// &var T // mutable reference to T
/// &var(x, y) // mutable reference to tuple of x and y
/// &T[] // reference to slice of T
/// &T[5] // reference to array of T
/// &T[] // slice of references to T
/// &T[5] // array of references to T
/// $T // virtual type T
/// T<int32>
/// T<Validate: false>
/// MyEnum
/// simulation.geometry.Vector2
///
/// A | B // implicit anonymous union
/// A & B // implicit anonymous intersection
/// struct MyResponse { x: int32, y: int32 }
/// enum { Good, Bad }
/// union { A(int), B(float) } // explicit anonymous union
/// function (int32) => int32
/// function () => Result<int32, struct Error { message: string }>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Infer placeholder `_`.
    Infer,
    /// Maybe '?T'.
    Maybe(NodeId<Type>),
    /// Not `!T`.
    Not(NodeId<Type>),
    /// Never `!`.
    Never,
    /// Self type (only inside associated scopes for types).
    Self_,
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Path to a type like `MyModule.MyType` or `MyModule.MyType<T1, T2, ...>`.
    Path {
        // path: PathId,
        // static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Reference `&T` to a `T`. Or `&var T` for a mutable reference.
    Reference {
        // mutability: ScopedMutability,
        target: NodeId<Type>,
    },
    /// Virtual type `$T`. Somewhat like Any<T>.
    Virtual(NodeId<Type>),
    /// Variadic type `..T`. Behaves like a slice.
    Variadic(NodeId<Type>),
    /// Array type `T[N]`. Must have static length.
    Array {
        element: NodeId<Type>,
        // count: NodeId<Expression>,
    },
    /// Slice type `T[]`. Unknown length (dynamically sized).
    Slice { element: NodeId<Type> },
    /// Tuple type `(T1, T2, ...)` (no tuple keyword).
    // Tuple(NodeId<Tuple>),
    /// Intersection type `A & B & C`.
    Intersection(Vec<NodeId<Type>>),
}

impl Node for Type {
    const KIND: NodeType = NodeType::Type;
}

/// An IntType represents arbitrary width integer with signedness.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IntType {
    /// Bit width.
    pub width: u16,
    /// Whether the integer is signed (`int*` or `uint*`).
    pub is_signed: bool,
}

/// A FloatType represents IEEE-754 float.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FloatType {
    /// 32-bit IEEE-754 float.
    Float32,
    /// 64-bit IEEE-754 float.
    Float64,
}

/// A PrimitiveType represents primitive types.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PrimitiveType {
    /// Unknown / uninitialized type.
    Undefined,
    /// Void / empty / unit type.
    Void,
    /// Null type.
    Null,
    /// Boolean type.
    Boolean,
    /// Character type.
    Character,
    /// Integer type with arbitrary width.
    Int(IntType),
    /// Floating point number type.
    Float(FloatType),
}
