use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;
use destack_serde::Reflect;

use crate::{FloatType, LocalNodeId, Symbol, Tree, Type};

/// Canonical type table for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct TypeTable {
    /// Cached primitive type ids keyed by primitive shape.
    #[serde(skip, default)]
    pub(crate) primitive_types: HashMap<PrimitiveType, LocalNodeId<Type>>,
    /// Nominal lineage keyed by type id.
    pub lineage_by_type: HashMap<LocalNodeId<Type>, TypeLineage>,
    /// Canonical display names keyed by type id.
    pub display_name_by_type: HashMap<LocalNodeId<Type>, StringId>,
    /// Persistent symbols keyed by nominal type id.
    pub symbol_by_type: HashMap<LocalNodeId<Type>, Symbol>,
}

impl TypeTable {
    /// Create an empty type table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy type table entries from one type id to another.
    pub fn copy_type_entries(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(lineage) = self.lineage(from).cloned() {
            self.set_lineage(to, lineage);
        }

        if let Some(display_name) = self.display_name(from) {
            self.set_display_name(to, display_name);
        }
    }

    /// Rebuild primitive type cache from one MIR tree.
    pub fn rebuild_primitive_types(&mut self, tree: &Tree) {
        self.primitive_types.clear();

        // collect primitive rows in node order
        for (type_id, ty) in tree.iter_nodes::<Type>() {
            let Some(primitive) = Self::primitive_type(ty) else {
                continue;
            };
            self.record_primitive_type(type_id, primitive);
        }
    }

    /// Return lineage for a type when present.
    pub fn lineage(&self, ty: LocalNodeId<Type>) -> Option<&TypeLineage> {
        self.lineage_by_type.get(&ty)
    }

    /// Record lineage for a type.
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
            Type::Character => Some(PrimitiveType::Character),
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
            Type::Float(float_type) => Some(PrimitiveType::Float {
                format: *float_type,
            }),
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

    /// Return the indexed character type id.
    pub fn character_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_types.get(&PrimitiveType::Character).copied()
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

    /// Return the indexed float type id for a format.
    pub fn float_type(&self, format: FloatType) -> Option<LocalNodeId<Type>> {
        self.primitive_types
            .get(&PrimitiveType::Float { format })
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

    /// Return the persistent symbol for one nominal type when present.
    pub fn symbol(&self, ty: LocalNodeId<Type>) -> Option<Symbol> {
        self.symbol_by_type.get(&ty).copied()
    }

    /// Record the persistent symbol for one nominal type.
    pub fn set_symbol(&mut self, ty: LocalNodeId<Type>, symbol: Symbol) -> Option<Symbol> {
        self.symbol_by_type.insert(ty, symbol)
    }
}

/// Lineage tables for nominal types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
    /// Unicode scalar value primitive type.
    Character,
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
    /// Float type with format.
    Float { format: FloatType },
}
