use dyst_source::StringId;

use crate::{Argument, Definition, Expression, Mutability, Name, Key, Node, NodeId, NodeType, Visibility};

/// The format of a variant (tuple or struct).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VariantFormat {
    /// A tuple struct with explicit representation.
    Tuple,
    /// A struct with explicit representation.
    Struct,
}

/// The type of a binding.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingKind {
    /// Definite binding (like `x: int32`).
    Must,
    /// Maybe binding (like `x?: int32` or just `T?`).
    Maybe,
}

/// The scope of a binding (dynamic or static).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingScope {
    /// Container scope (whatever contains the declaration).
    Container,
    /// Static scope (static in relation to the container).
    Static,
}

/// The operator to apply to the binding.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingOperator {
    /// Apply `as const` to the value of the binding.
    AsConst,
}

/// The modifiers of a field-like item.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct BindingModifier {
    /// The kind of the binding.
    pub kind: Option<BindingKind> = None,
    /// The scope of the binding.
    pub scope: Option<BindingScope> = None,
    /// The mutability of the field.
    pub mutability: Option<Mutability> = None,
    /// The visibility of the field.
    pub visibility: Option<Visibility> = None,
    /// The operator to apply to the binding.
    pub operator: Option<BindingOperator> = None,
}

impl BindingModifier {
    /// Create a new binding modifiers with the given kind.
    pub fn with_kind(self, kind: BindingKind) -> Self {
        Self {
            kind: Some(kind),
            ..self
        }
    }

    /// Create a new binding modifiers with the given scope.
    pub fn with_scope(self, scope: BindingScope) -> Self {
        Self {
            scope: Some(scope),
            ..self
        }
    }

    /// Create a new binding modifiers with the given mutability.
    pub fn with_mutability(self, mutability: Mutability) -> Self {
        Self {
            mutability: Some(mutability),
            ..self
        }
    }

    /// Create a new binding modifiers with the given visibility.
    pub fn with_visibility(self, visibility: Visibility) -> Self {
        Self {
            visibility: Some(visibility),
            ..self
        }
    }

    /// Create a new binding modifiers with the given operator.
    pub fn with_operator(self, operator: BindingOperator) -> Self {
        Self {
            operator: Some(operator),
            ..self
        }
    }
}

/// A Property is a property of a variant type (may be a field or method).
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Named field (like `x: int32`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        ty: Option<NodeId<Expression>>,
        value: Option<NodeId<Expression>>,
    },
    /// Named member function (like `foo()` or `<T>(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        definition: NodeId<Definition>,
    },
    /// Spread property (like `...a`).
    Spread {
        modifiers: Option<BindingModifier>,
        value: NodeId<Expression>,
    },
}

impl Node for Property {
    const TYPE: NodeType = NodeType::Property;
}

/// A EnumField is a enum field declaration.
///
/// Examples:
/// ```
/// A
/// B = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    /// The name of the enum field.
    pub name: Name,
    /// The default value of the enum field.
    pub value: Option<NodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}

/// A UnionField is a union field declaration.
///
/// Examples:
/// ```
/// A
/// A(int32)
/// A { x: int32, y: int32 } = 4
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum UnionField {
    /// Unit union field (like `A` or `A = 2`).
    Unit {
        name: StringId,
        value: Option<NodeId<Expression>>,
    },
    /// Tuple union field (like `A(int32)`).
    Tuple {
        name: StringId,
        fields: Vec<NodeId<Argument>>,
        value: Option<NodeId<Expression>>,
    },
    /// Struct union field (like `A { x: int32, y: int32 }`).
    Struct {
        name: StringId,
        fields: Vec<NodeId<Argument>>,
        value: Option<NodeId<Expression>>,
    },
}

impl Node for UnionField {
    const TYPE: NodeType = NodeType::UnionField;
}
