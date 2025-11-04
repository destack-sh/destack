use dyst_source::StringId;

use crate::{Expression, Mutability, Name, Node, NodeId, NodeType, Visibility};

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

/// A Field is a field in some variant type.
///
/// Examples:
/// ```
/// bar: int32
/// T
/// public T
/// readonly name: T
/// baz: T
/// public T
/// readonly bar: int32
/// baz?: T // shorthand for baz: T?
/// "Content-Type": string
/// [x: string]: any
/// [string]: woof
/// [T] = "hello"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    /// Named field.
    Named {
        modifiers: Option<BindingModifier>,
        name: Name,
        ty: NodeId<Expression>,
        default: Option<NodeId<Expression>>,
    },
    /// Positional field.
    Positional {
        modifiers: Option<BindingModifier>,
        ty: NodeId<Expression>,
        default: Option<NodeId<Expression>>,
    },
    /// Dynamic field.
    Dynamic {
        modifiers: Option<BindingModifier>,
        name: Option<StringId>,
        ty: NodeId<Expression>,
        key: NodeId<Expression>,
        default: Option<NodeId<Expression>>,
    },
}

impl Node for Field {
    const TYPE: NodeType = NodeType::Field;
}
