use crate::{
    Definition, Expression, Mutability, Node, NodeId, NodeType, Parameter, ScalarLiteral,
    ScopedMutability, StringId, WithClause,
};

/// A PrimitiveType is a primitive type node.
#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveType {
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
    /// Interface type `interface MyInterface { ... }`.
    Interface,
    /// Function type `function (T1, T2, ...) => T`.
    Function,
}

/// A TypeLiteral is a scalar type.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeLiteral {
    /// Never type `!`.
    Never,
    /// Any type `$` or `any`.
    Any,
    /// Infer placeholder `_`.
    Infer,
    /// Undefined type and value.
    Undefined,
    /// Unknown type.
    Unknown,
    /// Void type.
    Void,
    /// Null type and value.
    Null,
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Composite type.
    Composite(CompositeType),
    /// Scalar literal.
    ScalarLiteral(ScalarLiteral),
}

/// A TypeUnaryOperator is a type unary operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypeUnaryOperator {
    /// Not `!T`.
    Not,
    /// Maybe 'T?'.
    Maybe,
    /// Must 'T!'.
    Must,
    /// `type`
    Type,
    /// `readonly`
    Readonly,
    /// `typeof`
    Typeof,
    /// `keyof`
    Keyof,
    /// `infer`
    Infer,
}

/// A TypeBinaryOperator is a type binary operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypeBinaryOperator {
    /// `as`
    Cast,
    /// `in`
    In,
    /// `is`
    Is,
    /// `instanceof`
    Instanceof,
    /// `satisfies`
    Satisfies,
}

/// An Type in the type system.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Scalar type literal.
    Scalar(TypeLiteral),
    /// Resolved definition type.
    /// NOTE #Incomplete: shouldn't Type::Definition be an instance (statically parameterized)?
    ///  (same with all statically parameterized instantiations like Functions etc.?)
    Definition(NodeId<Definition>),

    /// Type unary operator.
    Unary {
        operator: TypeUnaryOperator,
        right: NodeId<Type>,
    },
    /// Mutable or immutable type `T`.
    Mutable {
        mutability: Mutability,
        right: NodeId<Type>,
    },
    /// Reference `&T` to a `T`. Or `&var T` for a mutable reference.
    Reference {
        mutability: Option<ScopedMutability>,
        variance: Option<VarianceBound>,
        right: NodeId<Type>,
    },
    /// Type binary operator.
    Binary {
        left: NodeId<Type>,
        operator: TypeBinaryOperator,
        right: NodeId<Type>,
    },

    /// Range type `T..T` (or `T..=T` for inclusive range).
    Range {
        start: NodeId<Type>,
        end: NodeId<Type>,
        is_inclusive: bool,
    },
    /// Sized array type `T[N]`.
    ArraySized {
        element: NodeId<Type>,
        count: NodeId<Expression>,
    },
    /// Array slice type `T[]`. Dynamically sized.
    ArraySlice { element: NodeId<Type> },
    /// Array type `[T1, T2, ...]`. May be fixed or dynamically sized.
    Array { elements: Vec<NodeId<Type>> },
    /// Tuple type `(T1, T2, ...)`. Fixed size.
    Tuple(Vec<NodeId<Type>>),
    /// Intersection type `A & B & C`.
    Intersection(Vec<NodeId<Type>>),

    /// Expression yet to be evaluated into a Type (like a Path).
    UnresolvedExpression(NodeId<Expression>),
    /// Unresolved Self type.
    UnresolvedSelf,

    /// Error type that could not be evaluated.
    Error,
}

impl Node for Type {
    const KIND: NodeType = NodeType::Type;
}

impl Type {
    /// Whether the type is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        !matches!(
            self,
            Type::UnresolvedExpression(_) | Type::UnresolvedSelf | Type::Error
        )
    }
}

/// The polymorphism of some type or definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Generics {
    /// The static parameters of the definition.
    pub static_parameters: Option<Vec<NodeId<Parameter>>> = None,
    /// The with clauses of the definition.
    pub with_clauses: Option<Vec<NodeId<WithClause>>> = None,
    /// The where clauses of the definition.
    pub where_clauses: Option<Vec<NodeId<WhereClause>>> = None,
}

/// A TypeBound is a type bound for a reference operation.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VarianceBound {
    /// Extends a type (such that X is a subtype of Y, i.e. X <: Y).
    Extends,
    /// Super a type (such that X is a supertype of Y, i.e. X >: Y).
    Super,
}

/// An IntType represents arbitrary width integer with signedness.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IntType {
    /// 8-bit signed integer (range: -2^7 to 2^7-1)
    Int8,
    /// 16-bit signed integer (range: -2^15 to 2^15-1)
    Int16,
    /// 32-bit signed integer (range: -2^31 to 2^31-1)
    Int32,
    /// 64-bit signed integer (range: -2^63 to 2^63-1)
    Int64,
    /// 128-bit signed integer (range: -2^127 to 2^127-1)
    Int128,
    /// 256-bit signed integer (range: -2^255 to 2^255-1)
    Int256,
    /// 8-bit unsigned integer (range: 0 to 2^8-1)
    Uint8,
    /// 16-bit unsigned integer (range: 0 to 2^16-1)
    Uint16,
    /// 32-bit unsigned integer (range: 0 to 2^32-1)
    Uint32,
    /// 64-bit unsigned integer (range: 0 to 2^64-1)
    Uint64,
    /// 128-bit unsigned integer (range: 0 to 2^128-1)
    Uint128,
    /// 256-bit unsigned integer (range: 0 to 2^256-1)
    Uint256,
    /// Arbitrary width integer with signedness.
    Variable { width: u16, is_signed: bool },
}

impl IntType {
    pub fn width(&self) -> u16 {
        match self {
            IntType::Int8 => 8,
            IntType::Int16 => 16,
            IntType::Int32 => 32,
            IntType::Int64 => 64,
            IntType::Int128 => 128,
            IntType::Int256 => 256,
            IntType::Uint8 => 8,
            IntType::Uint16 => 16,
            IntType::Uint32 => 32,
            IntType::Uint64 => 64,
            IntType::Uint128 => 128,
            IntType::Uint256 => 256,
            IntType::Variable {
                width,
                is_signed: _,
            } => *width,
        }
    }

    pub fn is_signed(&self) -> bool {
        match self {
            IntType::Int8 => true,
            IntType::Int16 => true,
            IntType::Int32 => true,
            IntType::Int64 => true,
            IntType::Int128 => true,
            IntType::Int256 => true,
            IntType::Uint8 => false,
            IntType::Uint16 => false,
            IntType::Uint32 => false,
            IntType::Uint64 => false,
            IntType::Uint128 => false,
            IntType::Uint256 => false,
            IntType::Variable {
                width: _,
                is_signed,
            } => *is_signed,
        }
    }

    #[inline]
    pub fn as_str(self) -> String {
        match self {
            IntType::Int8 => "int8".to_string(),
            IntType::Int16 => "int16".to_string(),
            IntType::Int32 => "int32".to_string(),
            IntType::Int64 => "int64".to_string(),
            IntType::Int128 => "int128".to_string(),
            IntType::Int256 => "int256".to_string(),
            IntType::Uint8 => "uint8".to_string(),
            IntType::Uint16 => "uint16".to_string(),
            IntType::Uint32 => "uint32".to_string(),
            IntType::Uint64 => "uint64".to_string(),
            IntType::Uint128 => "uint128".to_string(),
            IntType::Uint256 => "uint256".to_string(),
            IntType::Variable { width, is_signed } => {
                let mut as_str = if is_signed {
                    "int".to_string()
                } else {
                    "uint".to_string()
                };
                as_str.push_str(&width.to_string());
                as_str
            }
        }
    }
}

/// A FloatType represents IEEE-754 float.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FloatType {
    /// 16-bit IEEE-754 float.
    Float16,
    /// 32-bit IEEE-754 float.
    Float32,
    /// 64-bit IEEE-754 float.
    Float64,
    /// 80-bit IEEE-754 float.
    Float80,
    /// 128-bit IEEE-754 float.
    Float128,
    /// Arbitrary width IEEE-754 float.
    Variable { width: u16 },
}

impl FloatType {
    pub fn width(&self) -> u16 {
        match self {
            FloatType::Float16 => 16,
            FloatType::Float32 => 32,
            FloatType::Float64 => 64,
            FloatType::Float80 => 80,
            FloatType::Float128 => 128,
            FloatType::Variable { width } => *width,
        }
    }

    #[inline]
    pub fn as_str(self) -> String {
        match self {
            FloatType::Float16 => "float16".to_string(),
            FloatType::Float32 => "float32".to_string(),
            FloatType::Float64 => "float64".to_string(),
            FloatType::Float80 => "float80".to_string(),
            FloatType::Float128 => "float128".to_string(),
            FloatType::Variable { width } => format!("float{width}"),
        }
    }
}

/// A WhereClause is a single clause in a where type declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum WhereClause {
    /// Where assertion (like `T: int32`).
    Assertion {
        /// The target to assert (like `T` in `T: int32`)
        left: StringId,
        /// The assertion type (like `int32` in `T: int32`)
        right: NodeId<Expression>,
    },
    /// Where guard (like `T > Y`).
    Guard {
        /// The guard (like `T > Y` in `with T > Y`)
        guard: NodeId<Expression>,
    },
}

impl Node for WhereClause {
    const KIND: NodeType = NodeType::WhereClause;
}
