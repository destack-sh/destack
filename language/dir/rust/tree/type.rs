use crate::{
    Definition, Expression, Node, NodeId, NodeType, ScalarLiteral, ScopedMutability, StringId,
};

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
    pub fn as_str(self) -> String {
        format!("float{}", self.width)
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

    /// Variadic type `..T`. Behaves like a slice/array.
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

    /// Scalar primitive type.
    TypeLiteral(TypeLiteral),
    /// Literal value type.
    ScalarLiteral(ScalarLiteral),
    /// Self type (only inside associated scopes for types).
    Self_,
    /// Definition type.
    /// nocheckin #Incomplete: shouldn't the Definition type be an instance (statically parameterized)?
    ///  (same with all statically parameterized instantiations like Functions etc.?)
    Definition(NodeId<Definition>),
    /// An expression yet to be evaluated into a Type (like a Path).
    Expression(NodeId<Expression>),

    /// Error type that could not be evaluated.
    Error,
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
pub struct FloatType {
    /// Bit width.
    pub width: u16,
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
