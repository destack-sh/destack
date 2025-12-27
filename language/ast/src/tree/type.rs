use crate::{Expression, LocalNodeId, StringId};

/// A mapped type parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeMappedParameter {
    /// The parameter name (like `K`).
    pub name: StringId,
    /// The constraint type (like `keyof T`).
    pub constraint: LocalNodeId<Expression>,
    /// The optional key remap (like `as Foo<K>`).
    pub key_remap: Option<LocalNodeId<Expression>>,
}

/// A type modifier for mapped types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeModifier {
    /// Add a modifier (like `readonly` or `?`).
    Add,
    /// Remove a modifier (like `-readonly` or `-?`).
    Remove,
    /// No modifier specified.
    None,
}

/// Mapped type modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeMappedModifiers {
    /// The readonly modifier.
    pub readonly: TypeModifier,
    /// The optional modifier.
    pub optional: TypeModifier,
}

/// A type predicate subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypePredicateSubject {
    /// Identifier subject (like `x` in `x is T`).
    Identifier(StringId),
    /// `this` subject.
    This,
}
