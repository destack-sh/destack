use crate::{
    DeclarationScope, Expression, Mutability, Node, NodeId, NodeType, StringId, Type, Visibility,
};

/// The type of a binding (definite or maybe).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingKind {
    /// Definite binding (like `x: int32`).
    Must,
    /// Maybe binding (like `x?: int32` or just `T?`).
    Maybe,
}

/// The modifiers of a field-like item.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BindingModifiers {
    /// The kind of the binding.
    pub kind: Option<BindingKind>,
    /// The mutability of the field.
    pub mutability: Option<Mutability>,
    /// The visibility of the field.
    pub visibility: Option<Visibility>,
}

/// A VariantField is a field of a variant.
#[derive(Debug, Clone, PartialEq)]
pub enum VariantField {
    /// Named field ().
    Named {
        /// The modifiers of the field.
        modifiers: Option<BindingModifiers>,
        /// The name of the field.
        name: StringId,
        /// The type of the field.
        ty: NodeId<Type>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
    /// Positional field.
    Positional {
        /// The modifiers of the field.
        modifiers: Option<BindingModifiers>,
        /// The type of the field.
        ty: NodeId<Type>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
    /// Dynamic field.
    Dynamic {
        /// The modifiers of the field.
        modifiers: Option<BindingModifiers>,
        /// The name of the field (if any).
        name: Option<StringId>,
        /// The type of the field.
        ty: NodeId<Type>,
        /// The key type of the field.
        key: NodeId<Type>,
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
    pub fn modifiers(&self) -> Option<BindingModifiers> {
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
            VariantField::Named { name, .. } => Some(*name),
            VariantField::Positional { .. } => None,
            VariantField::Dynamic { name, .. } => *name,
        }
    }

    /// Get the type of the field.
    #[inline]
    pub fn ty(&self) -> NodeId<Type> {
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

/// A Variant is a concrete structured data type.
#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    /// Struct type.
    Struct {
        /// Name of the variant.
        name: Option<StringId>,
        /// Representation type of the variant.
        ty: Option<NodeId<Type>>,
        /// Fields of the variant.
        fields: Vec<NodeId<VariantField>>,
        /// Discriminator value.
        value: Option<NodeId<Expression>>,
    },
    /// Tuple type.
    Tuple {
        /// Name of the variant.
        name: Option<StringId>,
        /// Representation type of the variant.
        ty: Option<NodeId<Type>>,
        /// Fields of the variant.
        fields: Vec<NodeId<VariantField>>,
        /// Discriminator value.
        value: Option<NodeId<Expression>>,
    },
    /// Unit / "void" type.
    Unit {
        /// Name of the variant.
        name: Option<StringId>,
        /// Representation type of the variant.
        ty: Option<NodeId<Type>>,
        /// Discriminator value.
        value: Option<NodeId<Expression>>,
    },
}

impl Node for Variant {
    const KIND: NodeType = NodeType::Variant;
}

impl Variant {
    /// Get the name of the variant.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self {
            Variant::Struct { name, .. } => *name,
            Variant::Tuple { name, .. } => *name,
            Variant::Unit { name, .. } => *name,
        }
    }

    /// Get the representation type of the variant.
    #[inline]
    pub fn ty(&self) -> Option<NodeId<Type>> {
        match self {
            Variant::Struct { ty, .. } => *ty,
            Variant::Tuple { ty, .. } => *ty,
            Variant::Unit { ty, .. } => *ty,
        }
    }
}
