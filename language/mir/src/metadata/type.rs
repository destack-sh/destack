use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{LayoutMetadata, LocalNodeId, Type};

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

/// Table of canonical well known MIR types.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WellKnownTypes {
    /// Canonical well known string reference type.
    pub string: Option<LocalNodeId<Type>>,
}

impl WellKnownTypes {
    /// Copy one canonical identity when the type id is remapped.
    pub fn remap_type(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if self.string == Some(from) {
            self.string = Some(to);
        }
    }
}

/// Primitive type index for fast lookups.
#[derive(Clone, Debug, Default)]
pub(crate) struct PrimitiveTypeIndex {
    /// Indexed void type id.
    pub void: Option<LocalNodeId<Type>>,
    /// Indexed boolean type id.
    pub boolean: Option<LocalNodeId<Type>>,
    /// Indexed type descriptor type id.
    pub type_descriptor: Option<LocalNodeId<Type>>,
    /// Indexed type id type id.
    pub type_id: Option<LocalNodeId<Type>>,
    /// Indexed isize type id.
    pub isize: Option<LocalNodeId<Type>>,
    /// Indexed usize type id.
    pub usize: Option<LocalNodeId<Type>>,
    /// Indexed integer type ids keyed by width and signedness.
    pub ints: HashMap<(u16, bool), LocalNodeId<Type>>,
    /// Indexed float type ids keyed by width.
    pub floats: HashMap<u16, LocalNodeId<Type>>,
}

/// Key describing one primitive type.
#[derive(Clone, Debug)]
pub(crate) enum PrimitiveTypeKey {
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

impl LayoutMetadata {
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

    /// Return the primitive type key for a MIR type when applicable.
    pub(crate) fn primitive_type_key(ty: &Type) -> Option<PrimitiveTypeKey> {
        match ty {
            Type::Void => Some(PrimitiveTypeKey::Void),
            Type::Boolean => Some(PrimitiveTypeKey::Boolean),
            Type::TypeDescriptor => Some(PrimitiveTypeKey::TypeDescriptor),
            Type::TypeId => Some(PrimitiveTypeKey::TypeId),
            Type::Isize => Some(PrimitiveTypeKey::Isize),
            Type::Usize => Some(PrimitiveTypeKey::Usize),
            Type::Int {
                width,
                is_signed: signed,
            } => Some(PrimitiveTypeKey::Int {
                width: *width,
                signed: *signed,
            }),
            Type::Float { width } => Some(PrimitiveTypeKey::Float { width: *width }),
            _ => None,
        }
    }

    /// Record a type id in the primitive type index.
    pub(crate) fn record_primitive_type(
        &mut self,
        type_id: LocalNodeId<Type>,
        key: PrimitiveTypeKey,
    ) {
        match key {
            PrimitiveTypeKey::Void => {
                self.primitive_type_index.void.get_or_insert(type_id);
            }
            PrimitiveTypeKey::Boolean => {
                self.primitive_type_index.boolean.get_or_insert(type_id);
            }
            PrimitiveTypeKey::TypeDescriptor => {
                self.primitive_type_index
                    .type_descriptor
                    .get_or_insert(type_id);
            }
            PrimitiveTypeKey::TypeId => {
                self.primitive_type_index.type_id.get_or_insert(type_id);
            }
            PrimitiveTypeKey::Isize => {
                self.primitive_type_index.isize.get_or_insert(type_id);
            }
            PrimitiveTypeKey::Usize => {
                self.primitive_type_index.usize.get_or_insert(type_id);
            }
            PrimitiveTypeKey::Int { width, signed } => {
                self.primitive_type_index
                    .ints
                    .entry((width, signed))
                    .or_insert(type_id);
            }
            PrimitiveTypeKey::Float { width } => {
                self.primitive_type_index
                    .floats
                    .entry(width)
                    .or_insert(type_id);
            }
        }
    }

    /// Return the indexed boolean type id.
    pub fn boolean_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_index.boolean
    }

    /// Return the indexed void type id.
    pub fn void_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_index.void
    }

    /// Return the indexed type descriptor type id.
    pub fn type_descriptor_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_index.type_descriptor
    }

    /// Return the indexed type id type id.
    pub fn type_id_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_index.type_id
    }

    /// Return the indexed isize type id.
    pub fn isize_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_index.isize
    }

    /// Return the indexed usize type id.
    pub fn usize_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_index.usize
    }

    /// Return the indexed integer type id for a width and signedness.
    pub fn int_type(&self, width: u16, signed: bool) -> Option<LocalNodeId<Type>> {
        self.primitive_type_index
            .ints
            .get(&(width, signed))
            .copied()
    }

    /// Return the indexed float type id for a width.
    pub fn float_type(&self, width: u16) -> Option<LocalNodeId<Type>> {
        self.primitive_type_index.floats.get(&width).copied()
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

    /// Return the canonical well known string type.
    pub fn string_type(&self) -> Option<LocalNodeId<Type>> {
        self.well_known_types.string
    }

    /// Record the canonical well known string type.
    pub fn set_string_type(&mut self, type_id: LocalNodeId<Type>) -> Option<LocalNodeId<Type>> {
        self.well_known_types.string.replace(type_id)
    }
}
