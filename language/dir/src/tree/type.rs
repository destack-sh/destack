use destack_source::{NodeSpanList, NodeSpanType};
use serde::{Deserialize, Serialize};

use crate::{
    Declaration, Expression, FunctionSignature, GenericArgument, GenericParameter, Key,
    LocalNodeId, LocalSymbolId, MappedTypeModifier, Mutability, Node, NodeType, Parameter, Path,
    ScalarLiteral, StringId, Tree, TupleElement, TypeLiteral, VarianceBound, WhereClause,
};

/// One type-surface member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeMember {
    /// Named field.
    Field {
        key: Key,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        symbol: LocalSymbolId,
        is_static: bool,
        is_optional: bool,
        is_readonly: bool,
    },
    /// Named method.
    Method {
        key: Key,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
        is_static: bool,
        is_optional: bool,
    },
    /// Call signature declaration.
    CallSignature {
        signature: FunctionTypeDeclaration,
        symbol: LocalSymbolId,
    },
    /// Construct signature declaration.
    ConstructSignature {
        signature: ConstructorTypeDeclaration,
        symbol: LocalSymbolId,
    },
    /// Index signature.
    IndexSignature {
        name: StringId,
        key_type: LocalNodeId<TypeExpression>,
        value_type: LocalNodeId<TypeExpression>,
        symbol: LocalSymbolId,
        is_optional: bool,
        is_readonly: bool,
    },
    /// Type embedding.
    Embed {
        value: LocalNodeId<TypeExpression>,
        symbol: LocalSymbolId,
    },
    /// Associated type requirement or definition.
    AssociatedType {
        name: StringId,
        generic_parameters: Vec<LocalNodeId<GenericParameter>>,
        where_clauses: Vec<LocalNodeId<WhereClause>>,
        constraint: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<TypeExpression>>,
        symbol: LocalSymbolId,
    },
    /// Associated compile-time constant requirement or definition.
    AssociatedConst {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Malformed type member slot.
    Error { symbol: LocalSymbolId },
}

impl Node for TypeMember {
    const TYPE: NodeType = NodeType::TypeMember;
}

impl TypeMember {
    /// Get the declared name of the type member when one exists.
    pub fn name(&self) -> Option<StringId> {
        match self {
            TypeMember::AssociatedType { name, .. } | TypeMember::AssociatedConst { name, .. } => {
                Some(*name)
            }
            _ => None,
        }
    }

    /// Get the key of the type member when one exists.
    pub fn key(&self) -> Option<&Key> {
        match self {
            TypeMember::Field { key, .. } => Some(key),
            TypeMember::Method { key, .. } => Some(key),
            _ => None,
        }
    }

    /// Get the function signature of the type member when one exists.
    pub fn signature(&self) -> Option<&FunctionSignature> {
        match self {
            TypeMember::Method { signature, .. } => Some(signature),
            _ => None,
        }
    }

    /// Get the symbol of the type member.
    pub fn symbol(&self) -> LocalSymbolId {
        match self {
            TypeMember::Field { symbol, .. }
            | TypeMember::Method { symbol, .. }
            | TypeMember::CallSignature { symbol, .. }
            | TypeMember::ConstructSignature { symbol, .. }
            | TypeMember::IndexSignature { symbol, .. }
            | TypeMember::Embed { symbol, .. }
            | TypeMember::AssociatedType { symbol, .. }
            | TypeMember::AssociatedConst { symbol, .. }
            | TypeMember::Error { symbol } => *symbol,
        }
    }
}

/// A mapped type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeMappedParameter {
    /// The parameter name.
    pub name: StringId,
    /// The source type iterated by `in`.
    pub source_type: LocalNodeId<TypeExpression>,
    /// The optional key remap.
    pub key_remap: Option<LocalNodeId<TypeExpression>>,
    /// The parameter symbol.
    pub symbol: LocalSymbolId,
}

/// A type predicate subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypePredicateSubject {
    /// Identifier subject.
    Identifier(StringId),
    /// `this` subject.
    This,
}

/// One function type declaration in type space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionTypeDeclaration {
    /// The generic parameters of the function type.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the function type.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The optional `this` parameter.
    pub this_parameter: Option<LocalNodeId<Parameter>>,
    /// The parameters of the function type.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// The return type of the function type.
    pub return_type: Option<LocalNodeId<TypeExpression>>,
}

/// One constructor type declaration in type space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructorTypeDeclaration {
    /// The generic parameters of the constructor type.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the constructor type.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The parameters of the constructor type.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// The return type of the constructor type.
    pub return_type: Option<LocalNodeId<TypeExpression>>,
    /// Whether the constructor type is abstract.
    pub is_abstract: bool,
}

/// A type-space expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeExpression {
    /// Parenthesized type expression.
    Parenthesized {
        expression: LocalNodeId<TypeExpression>,
    },

    /// Scalar literal type.
    ScalarLiteral { value: ScalarLiteral },

    /// Literal type.
    Literal { value: TypeLiteral },

    /// Bare `intrinsic` marker in type space.
    Intrinsic,

    /// Parenthesized tuple type.
    Tuple {
        elements: Vec<LocalNodeId<TupleElement>>,
    },

    /// Bracket tuple type.
    ArrayTuple {
        elements: Vec<LocalNodeId<TupleElement>>,
    },

    /// Homogeneous array type.
    Array {
        element: LocalNodeId<TypeExpression>,
    },

    /// Runtime-length homogeneous view type.
    Slice {
        /// The element type.
        element: LocalNodeId<TypeExpression>,
    },

    /// Fixed-length array type.
    FixedArray {
        /// The element type.
        element: LocalNodeId<TypeExpression>,
        /// The length expression.
        length: LocalNodeId<Expression>,
    },

    /// Object type.
    Object {
        members: Vec<LocalNodeId<TypeMember>>,
    },

    /// Embedded declaration type.
    Declaration {
        declaration: LocalNodeId<Declaration>,
    },

    /// Function type declaration.
    FunctionTypeDeclaration(FunctionTypeDeclaration),

    /// Constructor type declaration.
    ConstructorTypeDeclaration(ConstructorTypeDeclaration),

    /// Qualified type reference with optional generic arguments.
    Reference {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },

    /// Type member projection with optional generic arguments.
    Member {
        left: LocalNodeId<TypeExpression>,
        name: StringId,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },

    /// `const` in type space.
    Const,

    /// `this` in type space.
    This,

    /// `readonly T`.
    Readonly {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `keyof T`.
    KeyOf {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `typeof value`.
    TypeOfValue { value: LocalNodeId<Expression> },

    /// `T!`.
    Must {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `T as comptime`.
    AsComptime {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `!T`.
    Not {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `^T`.
    OwnedOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `&T`.
    BorrowedOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `*T`.
    PointerOf {
        mutability: Option<Mutability>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// Union type.
    Union {
        elements: Vec<LocalNodeId<TypeExpression>>,
    },

    /// Intersection type.
    Intersection {
        elements: Vec<LocalNodeId<TypeExpression>>,
    },

    /// Conditional type.
    Conditional {
        left: LocalNodeId<TypeExpression>,
        extends_type: LocalNodeId<TypeExpression>,
        then_type: LocalNodeId<TypeExpression>,
        else_type: LocalNodeId<TypeExpression>,
    },

    /// Mapped type.
    Mapped {
        parameter: TypeMappedParameter,
        readonly: MappedTypeModifier,
        optional: MappedTypeModifier,
        value: LocalNodeId<TypeExpression>,
    },

    /// Indexed access type.
    Index {
        left: LocalNodeId<TypeExpression>,
        index: LocalNodeId<TypeExpression>,
    },

    /// Template literal type.
    TemplateLiteral {
        strings: Vec<StringId>,
        spans: Vec<LocalNodeId<TypeExpression>>,
    },

    /// Infer binding.
    Infer {
        name: StringId,
        constraint: Option<LocalNodeId<TypeExpression>>,
    },

    /// Type predicate.
    Predicate {
        asserts: bool,
        subject: TypePredicateSubject,
        target: Option<LocalNodeId<TypeExpression>>,
    },

    /// Missing type child.
    Missing,

    /// Error placeholder.
    Error,
}

impl Node for TypeExpression {
    const TYPE: NodeType = NodeType::TypeExpression;
}

impl TypeExpression {
    /// Return attached generic arguments when present.
    pub fn generic_arguments(&self) -> Option<&[LocalNodeId<GenericArgument>]> {
        match self {
            TypeExpression::Reference {
                generic_arguments, ..
            }
            | TypeExpression::Member {
                generic_arguments, ..
            } => Some(generic_arguments),
            _ => None,
        }
    }

    /// Resolve the source span kind that identifies this member name token.
    pub fn member_source_part(
        tree: &Tree,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> NodeSpanType {
        let source_id = tree.get_source(expression_id.id);
        let mut segment_index = 0u16;
        let mut current_id = expression_id;

        // walk left through one lowered member chain
        loop {
            let current_expression = tree.get::<TypeExpression>(current_id);

            // count each synthetic member hop that still belongs to the same source node
            if let TypeExpression::Member { left, .. } = current_expression
                && tree.get_source(left.id) == source_id
            {
                current_id = *left;
                segment_index = segment_index
                    .checked_add(1)
                    .expect("member source part segment index overflow");
                continue;
            }

            break;
        }

        // use indexed path segments for lowered qualified paths
        match tree.get::<TypeExpression>(current_id) {
            TypeExpression::Reference { path, .. }
                if tree.get_source(current_id.id) == source_id
                    && usize::from(segment_index) < path.segments.len() =>
            {
                NodeSpanType::ListItem(NodeSpanList::Segment, segment_index)
            }
            _ => NodeSpanType::Main,
        }
    }
}
