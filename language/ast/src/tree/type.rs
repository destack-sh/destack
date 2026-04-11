use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, StringId};

/// A mapped type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeMappedParameter {
    /// The parameter name (like `K`).
    pub name: StringId,
    /// The constraint type (like `keyof T`).
    pub constraint: LocalNodeId<Expression>,
    /// The optional key remap (like `as Foo<K>`).
    pub key_remap: Option<LocalNodeId<Expression>>,
}

/// A type modifier for mapped types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeModifier {
    /// The plain modifier without an explicit sign.
    Present,
    /// Add a modifier with an explicit `+` sign.
    Add,
    /// Remove a modifier (like `-readonly` or `-?`).
    Remove,
    /// No modifier specified.
    None,
}

/// Mapped type modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeMappedModifiers {
    /// The readonly modifier.
    pub readonly: TypeModifier,
    /// The optional modifier.
    pub optional: TypeModifier,
}

/// A type predicate subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypePredicateSubject {
    /// Identifier subject (like `x` in `x is T`).
    Identifier(StringId),
    /// `this` subject.
    This,
}
