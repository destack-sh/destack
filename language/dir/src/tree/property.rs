use crate::{
    Expression, FunctionSignature, Key, LocalNodeId, LocalSymbolId, Mutability, Node, NodeType,
    StaticExpression, Visibility,
};

/// Static property in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
/// This is a plain value type, not a tree node, so that we can pass it around directly.
#[derive(Debug, Clone, PartialEq)]
pub enum StaticProperty {
    /// Unevaluated property (needs compile-time evaluation).
    Unevaluated { node: LocalNodeId<Property> },

    /// Evaluated static field (like `x: int32`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        value: StaticExpression,
        default: Option<StaticExpression>,
        symbol: LocalSymbolId,
    },
    /// Evaluated static member function (like `foo()` or `<T>(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
        body: StaticExpression,
        symbol: LocalSymbolId,
    },
}

impl StaticProperty {
    /// Check if the static property and its values have been evaluated.
    pub fn is_evaluated(&self) -> bool {
        match self {
            StaticProperty::Unevaluated { .. } => false,
            StaticProperty::Field { value, default, .. } => {
                value.is_evaluated() && default.as_ref().is_none_or(|d| d.is_evaluated())
            }
            StaticProperty::Method { body, .. } => body.is_evaluated(),
        }
    }
}

/// The type of a binding.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingKind {
    /// Definite binding (like `x: int32`).
    Must,
    /// Maybe binding (like `x?: int32` or just `T?`).
    Maybe,
}

/// The anchor of a binding (static or instance).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingAnchor {
    /// Static container (static in relation to the container).
    Static,
    /// Instance container (whatever contains the declaration).
    Instance,
}

/// The operator to apply to the binding.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingOperator {
    /// Apply `as const` to the value of the binding.
    AsConst,
}

/// The modifiers of a field-like item.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BindingModifier {
    /// The kind of the binding.
    pub kind: Option<BindingKind>,
    /// The anchor of the binding.
    pub anchor: Option<BindingAnchor>,
    /// The mutability of the field.
    pub mutability: Option<Mutability>,
    /// The visibility of the field.
    pub visibility: Option<Visibility>,
    /// The operator to apply to the binding.
    pub operator: Option<BindingOperator>,
}

/// The mode of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionMode {
    /// Getter function.
    Getter,
    /// Setter function.
    Setter,
    /// Constructor function.
    Constructor,
    /// New type function.
    New,
    /// Implicit call function.
    Call,
}

/// A Property is a property of a variant type (may be a field or method).
/// `symbol` is the declaration symbol for this property.
/// For object literals, field resolution (to expected type's field) is in ResolutionTable.
///
/// Examples:
/// ```text
/// // field
/// x: int32
/// x
/// ...Bar
/// a: T
/// a?: T
/// private b: int32 = 4
/// public static c: int32 = 4
///
/// // method
/// foo()
/// <T>(): T
/// get x(): int32
/// set x(value: int32): void
/// public abstract foo(): void
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Named field (like `x: int32`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        value: Option<LocalNodeId<Expression>>,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Named member function (like `foo()` or `<T>(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Spread property (like `...a`).
    Spread {
        modifiers: Option<BindingModifier>,
        value: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
    },
}

impl Node for Property {
    const TYPE: NodeType = NodeType::Property;
}
