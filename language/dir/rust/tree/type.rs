use std::str::FromStr;

use crate::{
    Argument, Expression, Node, NodeId, NodeType, PathId, ScalarLiteral, ScopedMutability, Variant,
};

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

/// A TypeLiteral is literal type node.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeLiteral {
    /// Never type `!`.
    Never,
    /// Any type `$`.
    Any,
    /// Infer type `_`.
    Infer,
    /// Unknown / uninitialized type and value.
    Undefined,
    /// Void / empty / unit type.
    Void,
    /// Null type and value.
    Null,
    /// Boolean type.
    Boolean,
    /// Character type.
    Character,
    /// String type (unsized).
    String,
    /// "Number" type (alias).
    Number,
    /// Integer type.
    Int(IntType),
    /// Float type.
    Float(FloatType),
    /// Composite type.
    Composite(CompositeType),
    /// Self type.
    Self_,
}

/// A CompositeType represents composite types.
#[derive(Debug, Clone, PartialEq)]
pub enum CompositeType {
    /// Base type `type`.
    Type,
    /// Struct type `struct MyStruct { ... }`.
    Struct,
    /// Enum type `enum MyEnum { ... }`.
    Enum,
    /// Union type `A | B | C`.
    Union,
    /// Tuple type `(T1, T2, ...)`.
    Tuple,
    /// Trait type `trait MyTrait { ... }`.
    Trait,
    /// Function type `function (T1, T2, ...) => T`.
    Function,
}

/// An Type in the type system.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Infer placeholder `_`.
    Infer,
    /// Never `!`.
    Never,
    /// Not `!T`.
    Not(NodeId<Type>),
    /// Maybe '?T'.
    Maybe(NodeId<Type>),
    /// Reference `&T` to a `T`. Or `&var T` for a mutable reference.
    Reference {
        mutability: ScopedMutability,
        target: NodeId<Type>,
    },
    /// Virtual type `$T`.
    Virtual(NodeId<Type>),

    /// Variadic type `..T`. Behaves like a slice.
    Variadic(NodeId<Type>),
    /// Array type `T[N]`. Must have static length.
    Array {
        element: NodeId<Type>,
        count: NodeId<Expression>,
    },
    /// Slice type `T[]`. Unknown length (dynamically sized).
    Slice { element: NodeId<Type> },
    /// Tuple type.
    Tuple(Vec<NodeId<Type>>),
    /// Union type `A | B | C`.
    Union(Vec<NodeId<Type>>),
    /// Intersection type `A & B & C`.
    Intersection(Vec<NodeId<Type>>),

    /// An expression yet to be evaluated into a Type.
    Expression(NodeId<Expression>),
    /// Scalar primitive type.
    TypeLiteral(TypeLiteral),
    /// Literal value type.
    ScalarLiteral(ScalarLiteral),
    /// Self type (only inside associated scopes for types).
    Self_,
    /// Path to a type like `MyModule.MyType` or `MyModule.MyType<T1, T2, ...>`.
    Path {
        path: PathId,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Variant type.
    Variant(NodeId<Variant>),
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
