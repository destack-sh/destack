use crate::{
    Expression, FunctionSignature, GlobalSymbolId, Key, LocalNodeId, LocalSymbolId, Mutability,
    Node, NodeType, StaticExpression, Visibility,
};

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
///
/// Examples:
/// ```
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
    /// Unresolved named field (like `x: int32`).
    UnresolvedNamed {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        value: Option<LocalNodeId<Expression>>,
        default: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Unresolved named member function (like `foo()` or `<T>(): T`).
    UnresolvedMethod {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Unresolved spread property (like `...a`).
    UnresolvedSpread {
        modifiers: Option<BindingModifier>,
        value: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
    },

    /// Named field (like `x: int32`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        value: LocalNodeId<Expression>,
        default: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
        target_symbol: GlobalSymbolId,
    },
    /// Named member function (like `foo()` or `<T>(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
        body: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
        target_symbol: GlobalSymbolId,
    },
    /// Spread property (like `...a`).
    Spread {
        modifiers: Option<BindingModifier>,
        value: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
        target_symbol: GlobalSymbolId,
    },
}

impl Node for Property {
    const TYPE: NodeType = NodeType::Property;

    fn is_resolved(&self) -> bool {
        matches!(
            self,
            Property::Field { .. } | Property::Method { .. } | Property::Spread { .. }
        )
    }
}

/// Static property in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
#[derive(Debug, Clone, PartialEq)]
pub enum StaticProperty {
    /// Unresolved dynamic property.
    Unresolved { node: LocalNodeId<Property> },

    /// Named static field (like `x: int32`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        value: LocalNodeId<StaticExpression>,
        default: LocalNodeId<StaticExpression>,
        symbol: LocalSymbolId,
        target_symbol: GlobalSymbolId,
    },
    /// Named static member function (like `foo()` or `<T>(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
        body: LocalNodeId<StaticExpression>,
        symbol: LocalSymbolId,
        target_symbol: GlobalSymbolId,
    },
}

impl Node for StaticProperty {
    const TYPE: NodeType = NodeType::StaticProperty;

    fn is_resolved(&self) -> bool {
        !matches!(self, StaticProperty::Unresolved { .. })
    }
}
