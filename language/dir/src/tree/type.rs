use destack_source::{AdaptImage, NodeSpanType};
use serde::{Deserialize, Serialize};

use crate::{
    Argument, Declaration, Expression, FunctionSignature, GenericArgument, GlobalSymbolId, Key,
    LocalNodeId, Mutability, Node, NodeType, Path, ScalarLiteral, StringId, SymbolSpaceOrder,
    TypeLiteral, VarianceBound,
};

/// One object type property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum TypeProperty {
    /// Named field.
    Field {
        is_optional: bool,
        is_readonly: bool,
        key: Key,
        declared_type: LocalNodeId<TypeExpression>,
    },
    /// Named method.
    Method {
        is_optional: bool,
        key: Option<Key>,
        signature: FunctionSignature,
    },
    /// Index signature.
    IndexSignature {
        is_readonly: bool,
        name: StringId,
        key_type: LocalNodeId<TypeExpression>,
        value_type: LocalNodeId<TypeExpression>,
    },
    /// Malformed type property slot.
    Error,
}

impl Node for TypeProperty {
    const TYPE: NodeType = NodeType::TypeProperty;
}

/// A mapped type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct TypeMappedParameter {
    /// The parameter name.
    pub name: StringId,
    /// The parameter symbol.
    pub symbol: crate::LocalSymbolId,
    /// The source type iterated by `in`.
    pub source_type: LocalNodeId<TypeExpression>,
    /// The optional key remap.
    pub key_remap: Option<LocalNodeId<TypeExpression>>,
}

/// A mapped-type modifier sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub enum TypeModifier {
    /// The plain modifier without an explicit sign.
    Present,
    /// Add a modifier with an explicit `+` sign.
    Add,
    /// Remove a modifier with an explicit `-` sign.
    Remove,
    /// No modifier specified.
    None,
}

/// A type predicate subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub enum TypePredicateSubject {
    /// Identifier subject.
    Identifier(StringId),
    /// `this` subject.
    This,
}

/// A type-space syntax node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum TypeExpression {
    /// Parenthesized type expression.
    Parenthesized {
        expression: LocalNodeId<TypeExpression>,
    },

    /// Scalar literal type syntax.
    ScalarLiteral { value: ScalarLiteral },

    /// Literal type syntax.
    Literal { value: TypeLiteral },

    /// Tuple type syntax.
    Tuple {
        elements: Vec<LocalNodeId<GenericArgument>>,
    },

    /// Array type syntax.
    Array {
        element: LocalNodeId<TypeExpression>,
    },

    /// Object type syntax.
    Object {
        properties: Vec<LocalNodeId<TypeProperty>>,
    },

    /// Embedded declaration type syntax.
    Declaration {
        declaration: LocalNodeId<Declaration>,
    },

    /// Qualified type reference with optional generic arguments.
    Reference {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        space_order: SymbolSpaceOrder,
    },

    /// Resolved local type reference.
    LocalReference {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        target_symbol: GlobalSymbolId,
    },

    /// Resolved module type reference.
    ModuleReference {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        target_symbol: GlobalSymbolId,
    },

    /// Resolved global type reference.
    GlobalReference {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        target_symbol: GlobalSymbolId,
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

    /// Type import expression.
    Import {
        target: LocalNodeId<Expression>,
        arguments: Vec<LocalNodeId<Argument>>,
        qualifier: Option<Path>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },

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

    /// `!T`.
    Not {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `^T`.
    ValueOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `&T`.
    ReferenceOf {
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
        readonly: TypeModifier,
        optional: TypeModifier,
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
    /// Return the resolved target symbol when one exists.
    pub fn target_symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            TypeExpression::LocalReference { target_symbol, .. }
            | TypeExpression::ModuleReference { target_symbol, .. }
            | TypeExpression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        }
    }

    /// Return attached generic arguments when present.
    pub fn generic_arguments(&self) -> Option<&[LocalNodeId<GenericArgument>]> {
        match self {
            TypeExpression::Reference {
                generic_arguments, ..
            }
            | TypeExpression::LocalReference {
                generic_arguments, ..
            }
            | TypeExpression::ModuleReference {
                generic_arguments, ..
            }
            | TypeExpression::GlobalReference {
                generic_arguments, ..
            }
            | TypeExpression::Member {
                generic_arguments, ..
            }
            | TypeExpression::Import {
                generic_arguments, ..
            } => Some(generic_arguments),
            _ => None,
        }
    }

    /// Resolve the source span kind that identifies this member name token.
    pub fn member_source_part(
        tree: &crate::NodeTree,
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
            TypeExpression::LocalReference { path, .. }
            | TypeExpression::ModuleReference { path, .. }
            | TypeExpression::GlobalReference { path, .. }
                if tree.get_source(current_id.id) == source_id
                    && usize::from(segment_index) < path.segments.len() =>
            {
                NodeSpanType::Segment(segment_index)
            }
            _ => NodeSpanType::Main,
        }
    }
}
