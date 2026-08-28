use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Expression, Identifier, IdentifierName, LocalNodeId, Node, NodeType, PropertyName, Tree,
};

/// One writable JavaScript place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Place {
    /// Identifier place like `value`.
    Identifier { identifier: Identifier },
    /// Member place like `object.value`.
    Member {
        object: LocalNodeId<Expression>,
        property: IdentifierName,
    },
    /// Private member place like `object.#value`.
    PrivateMember {
        object: LocalNodeId<Expression>,
        property: Identifier,
    },
    /// Index place like `object[key]`.
    Index {
        object: LocalNodeId<Expression>,
        key: LocalNodeId<Expression>,
    },
}

impl Node for Place {
    const TYPE: NodeType = NodeType::Place;
}

impl Place {
    /// Return whether this place requires parentheses in statement position.
    pub(crate) fn needs_statement_parentheses(&self, tree: &Tree) -> bool {
        match self {
            Self::Member { object, .. }
            | Self::PrivateMember { object, .. }
            | Self::Index { object, .. } => tree.get(*object).needs_statement_parentheses(tree),
            Self::Identifier { .. } => false,
        }
    }
}

/// One JavaScript binding pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Pattern {
    /// Binding pattern like `x`.
    Binding { identifier: Identifier },
    /// Array pattern like `[a, , ...rest]`.
    Array {
        fields: Vec<LocalNodeId<ArrayPatternField>>,
        rest: Option<LocalNodeId<Pattern>>,
    },
    /// Object pattern like `{ a, b: value, ...rest }`.
    Object {
        fields: Vec<LocalNodeId<ObjectPatternField>>,
        rest: Option<Identifier>,
    },
}

impl Node for Pattern {
    const TYPE: NodeType = NodeType::Pattern;
}

/// One array binding field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ArrayPatternField {
    /// Positional field like `value` or `value = default`.
    Positional {
        pattern: LocalNodeId<Pattern>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Elided array slot.
    Elision,
}

impl Node for ArrayPatternField {
    const TYPE: NodeType = NodeType::ArrayPatternField;
}

/// One object binding field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ObjectPatternField {
    /// Named field like `name: value` or `name: value = default`.
    Named {
        name: PropertyName,
        pattern: LocalNodeId<Pattern>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Shorthand field like `value` or `value = default`.
    Shorthand {
        identifier: Identifier,
        default: Option<LocalNodeId<Expression>>,
    },
}

impl Node for ObjectPatternField {
    const TYPE: NodeType = NodeType::ObjectPatternField;
}

/// One destructuring assignment target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum AssignPattern {
    /// Writable place like `value`, `object.value`, or `object[key]`.
    Place { place: LocalNodeId<Place> },
    /// Array destructuring target like `[a, , ...rest]`.
    Array {
        fields: Vec<LocalNodeId<ArrayAssignPatternField>>,
        rest: Option<LocalNodeId<AssignPattern>>,
    },
    /// Object destructuring target like `{ x, y: z, ...rest }`.
    Object {
        fields: Vec<LocalNodeId<ObjectAssignPatternField>>,
        rest: Option<LocalNodeId<Place>>,
    },
}

impl Node for AssignPattern {
    const TYPE: NodeType = NodeType::AssignPattern;
}

impl AssignPattern {
    /// Return whether this target requires parentheses in statement position.
    pub(crate) fn needs_statement_parentheses(&self, tree: &Tree) -> bool {
        match self {
            Self::Object { .. } => true,
            Self::Place { place } => tree.get(*place).needs_statement_parentheses(tree),
            Self::Array { .. } => false,
        }
    }
}

/// One array assignment field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ArrayAssignPatternField {
    /// Positional field like `value` or `value = default`.
    Positional {
        pattern: LocalNodeId<AssignPattern>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Elided array slot.
    Elision,
}

impl Node for ArrayAssignPatternField {
    const TYPE: NodeType = NodeType::ArrayAssignPatternField;
}

/// One object assignment field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ObjectAssignPatternField {
    /// Named field like `name: value` or `name: value = default`.
    Named {
        name: PropertyName,
        pattern: LocalNodeId<AssignPattern>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Shorthand field like `value` or `value = default`.
    Shorthand {
        identifier: Identifier,
        default: Option<LocalNodeId<Expression>>,
    },
}

impl Node for ObjectAssignPatternField {
    const TYPE: NodeType = NodeType::ObjectAssignPatternField;
}
