use serde::{Deserialize, Serialize};

use crate::{
    Expression, FunctionSignature, Key, Keyword, LocalNodeId, Mutability, Node, NodeType,
    Visibility,
};

/// The type of a binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum BindingKind {
    /// Definite binding (like `x: int32`).
    Must,
    /// Maybe binding (like `x?: int32`).
    Maybe,
}

/// The abstraction modifier of a binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum AbstractionModifier {
    /// Abstract binding.
    Abstract,
    /// Override binding.
    Override,
    /// Abstract override binding.
    AbstractOverride,
}

/// Variance annotation for type parameters.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum VarianceModifier {
    /// Contravariant type parameter.
    In,
    /// Covariant type parameter.
    Out,
}

impl VarianceModifier {
    /// Return the keyword string for the variance modifier.
    pub fn as_str(&self) -> &'static str {
        match self {
            VarianceModifier::In => "in",
            VarianceModifier::Out => "out",
        }
    }
}

/// The anchor of a binding (static or instance).
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum BindingAnchor {
    /// Static container (static in relation to the container).
    Static,
    /// Instance container (whatever contains the declaration).
    #[default]
    Instance,
}

/// The operator to apply to the binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum BindingOperator {
    /// Apply `as const` to the value of the binding.
    AsConst,
}

/// The accessor kind of a binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessorKind {
    /// Auto-accessor (generates getter/setter).
    Accessor,
}

/// The evaluation timing of a binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Timing {
    /// Must be evaluated at compile time.
    Comptime,
}

/// The modifiers of a field-like item.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BindingModifier {
    /// The kind of the binding.
    pub kind: Option<BindingKind>,
    /// The abstraction modifier of the binding.
    pub abstraction: Option<AbstractionModifier>,
    /// The variance of a type parameter.
    pub variance: Option<VarianceModifier>,
    /// The anchor of the binding.
    pub anchor: Option<BindingAnchor>,
    /// The mutability of the binding.
    pub mutability: Option<Mutability>,
    /// The visibility of the binding.
    pub visibility: Option<Visibility>,
    /// The operator to apply to the binding.
    pub operator: Option<BindingOperator>,
    /// The accessor kind of the binding.
    pub accessor: Option<AccessorKind>,
    /// The evaluation timing of the binding.
    pub timing: Option<Timing>,
}

impl BindingModifier {
    /// Create a new binding modifiers with the given kind.
    pub fn with_kind(self, kind: BindingKind) -> Self {
        Self {
            kind: Some(kind),
            ..self
        }
    }

    /// Create a new binding modifiers with the given anchor.
    pub fn with_anchor(self, anchor: BindingAnchor) -> Self {
        Self {
            anchor: Some(anchor),
            ..self
        }
    }

    /// Create a new binding modifiers with the given variance.
    pub fn with_variance(self, variance: VarianceModifier) -> Self {
        Self {
            variance: Some(variance),
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

    /// Create a new binding modifiers with the given accessor kind.
    pub fn with_accessor(self, accessor: AccessorKind) -> Self {
        Self {
            accessor: Some(accessor),
            ..self
        }
    }

    /// Create a new binding modifiers with the given timing.
    pub fn with_timing(self, timing: Timing) -> Self {
        Self {
            timing: Some(timing),
            ..self
        }
    }
}

/// The mode of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
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

impl FunctionMode {
    /// Get the keyword for the function accessor.
    #[inline]
    pub fn to_keyword(&self) -> Option<Keyword> {
        match self {
            FunctionMode::Getter => Some(Keyword::Get),
            FunctionMode::Setter => Some(Keyword::Set),
            FunctionMode::Constructor => Some(Keyword::Constructor),
            FunctionMode::New => Some(Keyword::New),
            FunctionMode::Call => None,
        }
    }
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Property {
    /// Named field (like `x: int32`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        value: Option<LocalNodeId<Expression>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Named member function (like `foo()` or `<T>(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
    },
    /// Spread property (like `...a`).
    Spread {
        modifiers: Option<BindingModifier>,
        value: LocalNodeId<Expression>,
    },
}

impl Node for Property {
    const TYPE: NodeType = NodeType::Property;
}

/// A Member is a member of a class-like declaration.
///
/// Members differ from Properties in that they support class-specific constructs:
/// - Static blocks for initialization
/// - Type embedding via `...Type` syntax
/// - Visibility modifiers (public, private, protected)
/// - Static anchor
///
/// Examples:
/// ```
/// // field
/// x: int32
/// private y: boolean = true
/// public static z: int32 = 42
///
/// // method
/// foo() { }
/// public abstract bar(): void
/// get name(): string { }
///
/// // embedding (compile-time type inclusion)
/// ...Base
///
/// // static block (ES2022)
/// static { console.log("initializing") }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Member {
    /// Associated type alias (like `type Item = T`).
    Type {
        modifiers: Option<BindingModifier>,
        name: LocalNodeId<Expression>,
        ty: Option<LocalNodeId<Expression>>,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Named field (like `x: int32`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        value: Option<LocalNodeId<Expression>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Named member function (like `foo()` or `<T>(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
    },
    /// Type embedding (like `...Base`), includes all members from the embedded type.
    Embed {
        modifiers: Option<BindingModifier>,
        value: LocalNodeId<Expression>,
    },
    /// Static initialization block (like `static { ... }`).
    StaticBlock {
        modifiers: Option<BindingModifier>,
        body: LocalNodeId<Expression>,
    },
    /// Comptime block (like `comptime { ... }`).
    ComptimeBlock {
        modifiers: Option<BindingModifier>,
        body: LocalNodeId<Expression>,
    },
}

impl Node for Member {
    const TYPE: NodeType = NodeType::Member;
}
