use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Global, LocalNodeId, Type};

/// Canonical type facts for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TypeMetadata {
    /// Cached primitive type ids keyed by primitive shape.
    #[serde(skip, default)]
    pub(crate) primitive_types: HashMap<PrimitiveType, LocalNodeId<Type>>,
    /// Nominal lineage keyed by type id.
    pub lineage_by_type: HashMap<LocalNodeId<Type>, TypeLineage>,
    /// Runtime type descriptor globals keyed by type id.
    pub descriptor_by_type: HashMap<LocalNodeId<Type>, LocalNodeId<Global>>,
    /// Canonical display names keyed by type id.
    pub display_name_by_type: HashMap<LocalNodeId<Type>, StringId>,
}

impl TypeMetadata {
    /// Create empty type metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy type metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(lineage) = self.lineage(from).cloned() {
            self.set_lineage(to, lineage);
        }

        if let Some(descriptor) = self.descriptor_global(from) {
            self.set_descriptor_global(to, descriptor);
        }

        if let Some(display_name) = self.display_name(from) {
            self.set_display_name(to, display_name);
        }
    }

    /// Return lineage metadata for a type when present.
    pub fn lineage(&self, ty: LocalNodeId<Type>) -> Option<&TypeLineage> {
        self.lineage_by_type.get(&ty)
    }

    /// Record lineage metadata for a type.
    pub fn set_lineage(
        &mut self,
        ty: LocalNodeId<Type>,
        lineage: TypeLineage,
    ) -> Option<TypeLineage> {
        self.lineage_by_type.insert(ty, lineage)
    }

    /// Return the primitive shape for a MIR type when applicable.
    pub(crate) fn primitive_type(ty: &Type) -> Option<PrimitiveType> {
        match ty {
            Type::Void => Some(PrimitiveType::Void),
            Type::Boolean => Some(PrimitiveType::Boolean),
            Type::TypeDescriptor => Some(PrimitiveType::TypeDescriptor),
            Type::TypeId => Some(PrimitiveType::TypeId),
            Type::Isize => Some(PrimitiveType::Isize),
            Type::Usize => Some(PrimitiveType::Usize),
            Type::Int {
                width,
                is_signed: signed,
            } => Some(PrimitiveType::Int {
                width: *width,
                signed: *signed,
            }),
            Type::Float { width } => Some(PrimitiveType::Float { width: *width }),
            _ => None,
        }
    }

    /// Record a type id in the primitive type cache.
    pub(crate) fn record_primitive_type(
        &mut self,
        type_id: LocalNodeId<Type>,
        primitive: PrimitiveType,
    ) {
        self.primitive_types.entry(primitive).or_insert(type_id);
    }

    /// Return the indexed boolean type id.
    pub fn boolean_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_types.get(&PrimitiveType::Boolean).copied()
    }

    /// Return the indexed void type id.
    pub fn void_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_types.get(&PrimitiveType::Void).copied()
    }

    /// Return the indexed type descriptor type id.
    pub fn type_descriptor_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_types
            .get(&PrimitiveType::TypeDescriptor)
            .copied()
    }

    /// Return the indexed type id type id.
    pub fn type_id_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_types.get(&PrimitiveType::TypeId).copied()
    }

    /// Return the indexed isize type id.
    pub fn isize_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_types.get(&PrimitiveType::Isize).copied()
    }

    /// Return the indexed usize type id.
    pub fn usize_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_types.get(&PrimitiveType::Usize).copied()
    }

    /// Return the indexed integer type id for a width and signedness.
    pub fn int_type(&self, width: u16, signed: bool) -> Option<LocalNodeId<Type>> {
        self.primitive_types
            .get(&PrimitiveType::Int { width, signed })
            .copied()
    }

    /// Return the indexed float type id for a width.
    pub fn float_type(&self, width: u16) -> Option<LocalNodeId<Type>> {
        self.primitive_types
            .get(&PrimitiveType::Float { width })
            .copied()
    }

    /// Return the display name for a type when present.
    pub fn display_name(&self, ty: LocalNodeId<Type>) -> Option<StringId> {
        self.display_name_by_type.get(&ty).copied()
    }

    /// Record the display name for a type.
    pub fn set_display_name(&mut self, ty: LocalNodeId<Type>, name: StringId) -> Option<StringId> {
        self.display_name_by_type.insert(ty, name)
    }

    /// Return the existing display name for a type or insert the provided one.
    pub fn ensure_display_name(&mut self, ty: LocalNodeId<Type>, name: StringId) -> StringId {
        *self.display_name_by_type.entry(ty).or_insert(name)
    }

    /// Return the runtime type descriptor global for a type when present.
    pub fn descriptor_global(&self, ty: LocalNodeId<Type>) -> Option<LocalNodeId<Global>> {
        self.descriptor_by_type.get(&ty).copied()
    }

    /// Record the runtime type descriptor global for a type.
    pub fn set_descriptor_global(
        &mut self,
        ty: LocalNodeId<Type>,
        descriptor: LocalNodeId<Global>,
    ) -> Option<LocalNodeId<Global>> {
        self.descriptor_by_type.insert(ty, descriptor)
    }
}

/// Lineage metadata for nominal types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeLineage {
    /// Optional parent type for class inheritance.
    pub parent: Option<LocalNodeId<Type>>,
    /// Interfaces implemented by this type.
    pub interfaces: Vec<LocalNodeId<Type>>,
    /// True when the type is sealed to external extension.
    pub is_sealed: bool,
    /// True when the type is final and cannot be subclassed.
    pub is_final: bool,
    /// True when the type is abstract and cannot be instantiated.
    pub is_abstract: bool,
    /// True when the type represents an interface.
    pub is_interface: bool,
}

/// Primitive type shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PrimitiveType {
    /// Void primitive type.
    Void,
    /// Boolean primitive type.
    Boolean,
    /// Runtime type descriptor type.
    TypeDescriptor,
    /// Runtime type id type.
    TypeId,
    /// Pointer sized signed integer type.
    Isize,
    /// Pointer sized unsigned integer type.
    Usize,
    /// Integer type with width and signedness.
    Int { width: u16, signed: bool },
    /// Float type with width.
    Float { width: u16 },
}
