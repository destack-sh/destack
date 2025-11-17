use crate::{
    BindingModifier, Expression, FunctionSignature, Key, Mutability, Node, NodeId, NodeType,
    Parameter, ScalarLiteral, StringId, SymbolId, WithClause,
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
    /// Bigint type (unsized).
    Bigint,
    /// "Number" type (alias).
    Number,
    /// Integer type.
    Int(IntType),
    /// Float type.
    Float(FloatType),
    /// Symbol type.
    Symbol,
    /// Unique symbol type.
    UniqueSymbol,
}

/// A DefinitionType represents composite types.
#[derive(Debug, Clone, PartialEq)]
pub enum DefinitionType {
    /// Root type `type`.
    Type,
    /// Module type.
    Namespace,
    /// Struct type.
    Struct,
    /// Class type.
    Class,
    /// Enum type.
    Enum,
    /// Union type.
    Union,
    /// Interface type.
    Interface,
    /// Extension type.
    Extension,
    /// Function type.
    Function,
}

/// A TypeLiteral is a scalar type.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeLiteral {
    /// Never type `never`.
    Never,
    /// Any type `any`.
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
    Composite(DefinitionType),
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
    /// `infer`
    Infer,
    /// `as const`
    AsConst,
    /// `asserts`
    Asserts,
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
    InstanceOf,
    /// `satisfies`
    Satisfies,
    /// `extends`
    Extends,
    /// `implements`
    Implements,
}

/// An Type in the type system.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Scalar type literal.
    Scalar(TypeLiteral),
    /// Resolved symbol type.
    Symbol(SymbolId),

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
    /// Value `^T` of a `T`. Or `^var T` for a mutable value.
    ValueOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: NodeId<Type>,
    },
    /// Reference of `&T` to a `T`. Or `&var T` for a mutable reference.
    ReferenceOf {
        mutability: Option<Mutability>,
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
    /// Array type with fixed size (like `T[N]`).
    ArraySized {
        element: NodeId<Type>,
        count: NodeId<Expression>,
    },
    /// Array type with dynamically sized elements (like `T[]`).
    Array { element: Option<NodeId<Type>> },
    /// Tuple type `[T1, T2, ...]`.
    Tuple { elements: Vec<NodeId<Type>> },
    /// Struct type `{ a: T1, b: T2, ... }`.
    Struct { attributes: Vec<NodeId<TypeField>> },
    /// Union type `A | B | C`.
    Union { elements: Vec<NodeId<Type>> },
    /// Intersection type `A & B & C`.
    Intersection { elements: Vec<NodeId<Type>> },
    /// Function type `(T1, T2, ...) -> T`.
    Function { signature: FunctionSignature },

    /// Expression yet to be resolved into a Type (like a Path).
    UnresolvedExpression(NodeId<Expression>),
    /// Unresolved Self type.
    UnresolvedSelf,

    /// Error type that could not be resolved.
    Error,
}

impl Node for Type {
    const TYPE: NodeType = NodeType::Type;
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

/// The type of an attribute (like a property or field).
#[derive(Debug, Clone, PartialEq)]
pub enum TypeField {
    /// Named field (like `a: T`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        ty: NodeId<Type>,
    },
    /// Named method (like `foo(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
    },
}

impl Node for TypeField {
    const TYPE: NodeType = NodeType::TypeField;
}

/// The polymorphism of some type or declaration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Generics {
    /// The static parameters of the declaration.
    pub static_parameters: Option<Vec<NodeId<Parameter>>> = None,
    /// The with clauses of the declaration.
    pub with_clauses: Option<Vec<NodeId<WithClause>>> = None,
    /// The where clauses of the declaration.
    pub where_clauses: Option<Vec<NodeId<WhereClause>>> = None,
}

impl Generics {
    /// Check whether the generics contain any clauses.
    pub fn is_empty(&self) -> bool {
        self.static_parameters.is_none()
            && self.with_clauses.is_none()
            && self.where_clauses.is_none()
    }
}

/// The polymoprhic relations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Heritage {
    /// The extends types of the declaration.
    pub extends_types: Option<Vec<NodeId<Type>>> = None,
    /// The implements types of the declaration.
    pub implements_types: Option<Vec<NodeId<Type>>> = None,
    /// The embedded types of the declaration.
    pub embedded_types: Option<Vec<NodeId<Type>>> = None,
}

impl Heritage {
    /// Check whether the heritage carries any relations.
    pub fn is_empty(&self) -> bool {
        self.extends_types.is_none()
            && self.implements_types.is_none()
            && self.embedded_types.is_none()
    }
}

/// A TypeKind determines nominal vs. structural typing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypeKind {
    /// Structural typing (like `type T = { a: int32, b: boolean }`).
    Structural,
    /// Nominal typing (like `newtype T = int32`).
    Nominal,
}

/// A TypeBound is a type bound for a reference operation.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VarianceBound {
    /// Implements a type (such that X implements Y, i.e. X implements Y).
    Implements,
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
    /// Pointer-sized integer (range: 0 to 2^pointer_width-1).
    IntP,
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
    /// Pointer-sized unsigned integer (range: 0 to 2^pointer_width-1).
    UintP,
    /// Arbitrary width integer with signedness.
    Arbitrary { width: u16, is_signed: bool },
}

impl IntType {
    /// Try to simplify an arbitrary width integer type to a fixed width integer type.
    pub fn try_simplify(&self) -> Option<IntType> {
        match *self {
            IntType::Arbitrary {
                width,
                is_signed: true,
            } => match width {
                8 => Some(IntType::Int8),
                16 => Some(IntType::Int16),
                32 => Some(IntType::Int32),
                64 => Some(IntType::Int64),
                128 => Some(IntType::Int128),
                256 => Some(IntType::Int256),
                _ => None,
            },
            IntType::Arbitrary {
                width,
                is_signed: false,
            } => match width {
                8 => Some(IntType::Uint8),
                16 => Some(IntType::Uint16),
                32 => Some(IntType::Uint32),
                64 => Some(IntType::Uint64),
                128 => Some(IntType::Uint128),
                256 => Some(IntType::Uint256),
                _ => None,
            },
            _ => None,
        }
    }

    /// Simplify this integer type if possible.
    pub fn simplify(self) -> Self {
        match self.try_simplify() {
            Some(int_type) => int_type,
            None => self,
        }
    }

    /// Get the width of the integer type.
    pub fn width(&self) -> Option<u16> {
        let width = match self {
            IntType::Int8 => 8,
            IntType::Int16 => 16,
            IntType::Int32 => 32,
            IntType::Int64 => 64,
            IntType::Int128 => 128,
            IntType::Int256 => 256,
            IntType::IntP => return None,
            IntType::Uint8 => 8,
            IntType::Uint16 => 16,
            IntType::Uint32 => 32,
            IntType::Uint64 => 64,
            IntType::Uint128 => 128,
            IntType::Uint256 => 256,
            IntType::UintP => return None,
            IntType::Arbitrary {
                width,
                is_signed: _,
            } => *width,
        };
        Some(width)
    }

    /// Whether the integer type is signed.
    pub fn is_signed(&self) -> bool {
        match self {
            IntType::Int8 => true,
            IntType::Int16 => true,
            IntType::Int32 => true,
            IntType::Int64 => true,
            IntType::Int128 => true,
            IntType::Int256 => true,
            IntType::IntP => true,
            IntType::Uint8 => false,
            IntType::Uint16 => false,
            IntType::Uint32 => false,
            IntType::Uint64 => false,
            IntType::Uint128 => false,
            IntType::Uint256 => false,
            IntType::UintP => false,
            IntType::Arbitrary {
                width: _,
                is_signed,
            } => *is_signed,
        }
    }

    /// Get the string representation of the integer type.
    #[inline]
    pub fn as_str(self) -> String {
        match self {
            IntType::Int8 => "int8".to_string(),
            IntType::Int16 => "int16".to_string(),
            IntType::Int32 => "int32".to_string(),
            IntType::Int64 => "int64".to_string(),
            IntType::Int128 => "int128".to_string(),
            IntType::Int256 => "int256".to_string(),
            IntType::IntP => "intp".to_string(),
            IntType::Uint8 => "uint8".to_string(),
            IntType::Uint16 => "uint16".to_string(),
            IntType::Uint32 => "uint32".to_string(),
            IntType::Uint64 => "uint64".to_string(),
            IntType::Uint128 => "uint128".to_string(),
            IntType::Uint256 => "uint256".to_string(),
            IntType::UintP => "uintp".to_string(),
            IntType::Arbitrary { width, is_signed } => {
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
    Arbitrary { width: u16 },
}

impl FloatType {
    /// Try to simplify an arbitrary width float type to a fixed width float type.
    pub fn try_simplify(&self) -> Option<FloatType> {
        match self {
            &FloatType::Arbitrary { width } => match width {
                16 => Some(FloatType::Float16),
                32 => Some(FloatType::Float32),
                64 => Some(FloatType::Float64),
                80 => Some(FloatType::Float80),
                128 => Some(FloatType::Float128),
                _ => None,
            },
            _ => None,
        }
    }

    /// Simplify this float type if possible.
    pub fn simplify(self) -> Self {
        match self.try_simplify() {
            Some(float_type) => float_type,
            None => self,
        }
    }

    /// Get the width of the float type.
    pub fn width(&self) -> u16 {
        match self {
            FloatType::Float16 => 16,
            FloatType::Float32 => 32,
            FloatType::Float64 => 64,
            FloatType::Float80 => 80,
            FloatType::Float128 => 128,
            FloatType::Arbitrary { width } => *width,
        }
    }

    /// Get the string representation of the float type.
    #[inline]
    pub fn as_str(self) -> String {
        match self {
            FloatType::Float16 => "float16".to_string(),
            FloatType::Float32 => "float32".to_string(),
            FloatType::Float64 => "float64".to_string(),
            FloatType::Float80 => "float80".to_string(),
            FloatType::Float128 => "float128".to_string(),
            FloatType::Arbitrary { width } => format!("float{width}"),
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
    const TYPE: NodeType = NodeType::WhereClause;
}
