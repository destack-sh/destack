use dyst_source::StringId;

use crate::{Expression, Mutability, Name, Node, NodeId, NodeType, Visibility};

/// The kind of a variant (tuple or struct).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VariantKind {
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

/// A VariantField is a field declaration in some type.
///
/// Examples:
/// ```
/// bar: int32
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
pub enum VariantField {
    /// Named field.
    Named {
        /// The modifiers of the field.
        modifiers: Option<BindingModifier>,
        /// The name of the field.
        name: Name,
        /// The type of the field.
        ty: NodeId<Expression>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
    /// Positional field.
    Positional {
        /// The modifiers of the field.
        modifiers: Option<BindingModifier>,
        /// The type of the field.
        ty: NodeId<Expression>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
    /// Dynamic field.
    Dynamic {
        /// The modifiers of the field.
        modifiers: Option<BindingModifier>,
        /// The name of the field (if any).
        name: Option<StringId>,
        /// The type of the field.
        ty: NodeId<Expression>,
        /// The key type of the field.
        key: NodeId<Expression>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
}

impl Node for VariantField {
    const KIND: NodeType = NodeType::VariantField;
}

impl VariantField {
    /// Get the modifiers of the field.
    #[inline]
    pub fn modifiers(&self) -> Option<BindingModifier> {
        match self {
            VariantField::Named { modifiers, .. } => *modifiers,
            VariantField::Positional { modifiers, .. } => *modifiers,
            VariantField::Dynamic { modifiers, .. } => *modifiers,
        }
    }

    /// Get the name of the field.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self {
            VariantField::Named { name, .. } => Some(name.string()),
            VariantField::Positional { .. } => None,
            VariantField::Dynamic { name, .. } => *name,
        }
    }

    /// Get the type of the field.
    #[inline]
    pub fn ty(&self) -> NodeId<Expression> {
        match self {
            VariantField::Named { ty, .. } => *ty,
            VariantField::Positional { ty, .. } => *ty,
            VariantField::Dynamic { ty, .. } => *ty,
        }
    }

    /// Get the default value of the field.
    #[inline]
    pub fn default(&self) -> Option<NodeId<Expression>> {
        match self {
            VariantField::Named { default, .. } => *default,
            VariantField::Positional { default, .. } => *default,
            VariantField::Dynamic { default, .. } => *default,
        }
    }
}
