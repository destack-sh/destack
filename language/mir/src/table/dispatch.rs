use std::mem;

use serde::{Deserialize, Serialize};

use destack_core::StringId;
use destack_serde::Reflect;

use crate::{Field, Function, LocalNodeId, Type};

/// Canonical dispatch table for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct DispatchTable {
    /// Virtual dispatch tables.
    pub virtual_tables: Vec<VirtualTable>,
    /// Dynamic dispatch tables.
    pub dynamic_tables: Vec<DynamicTable>,
    /// Dynamic dispatch shapes keyed by constraint type id.
    pub dynamic_shapes: Vec<DynamicShape>,
}

impl DispatchTable {
    /// Create an empty dispatch table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy dispatch table entries from one type id to another.
    pub fn copy_type_entries(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(table) = self.virtual_table(from).cloned() {
            let mut table = table;
            table.concrete = to;
            self.insert_virtual_table(table);
        }

        let dynamic_tables = self
            .dynamic_tables
            .iter()
            .filter(|table| table.concrete == from)
            .cloned()
            .collect::<Vec<_>>();

        for mut table in dynamic_tables {
            table.concrete = to;
            self.insert_dynamic_table(table);
        }
    }

    /// Insert a virtual table.
    pub fn insert_virtual_table(&mut self, table: VirtualTable) -> Option<VirtualTable> {
        if let Some(index) = self
            .virtual_tables
            .iter()
            .position(|candidate| candidate.concrete == table.concrete)
        {
            return Some(mem::replace(&mut self.virtual_tables[index], table));
        }

        self.virtual_tables.push(table);

        None
    }

    /// Return the virtual table for a type when present.
    pub fn virtual_table(&self, ty: LocalNodeId<Type>) -> Option<&VirtualTable> {
        self.virtual_tables
            .iter()
            .find(|table| table.concrete == ty)
    }

    /// Iterate all virtual tables.
    pub fn iter_virtual_tables(&self) -> impl Iterator<Item = &VirtualTable> {
        self.virtual_tables.iter()
    }

    /// Insert a dynamic table.
    pub fn insert_dynamic_table(&mut self, table: DynamicTable) -> Option<DynamicTable> {
        if let Some(index) = self.dynamic_tables.iter().position(|candidate| {
            candidate.concrete == table.concrete && candidate.constraint == table.constraint
        }) {
            return Some(mem::replace(&mut self.dynamic_tables[index], table));
        }

        self.dynamic_tables.push(table);

        None
    }

    /// Return the dynamic table for a concrete type and constraint when present.
    pub fn dynamic_table(
        &self,
        concrete: LocalNodeId<Type>,
        constraint: LocalNodeId<Type>,
    ) -> Option<&DynamicTable> {
        self.dynamic_tables
            .iter()
            .find(|table| table.concrete == concrete && table.constraint == constraint)
    }

    /// Iterate all dynamic tables.
    pub fn iter_dynamic_tables(&self) -> impl Iterator<Item = &DynamicTable> {
        self.dynamic_tables.iter()
    }

    /// Return the dynamic shape for a constraint type id.
    pub fn dynamic_shape(&self, constraint: LocalNodeId<Type>) -> Option<&DynamicShape> {
        self.dynamic_shapes
            .iter()
            .find(|shape| shape.constraint == constraint)
    }

    /// Iterate all dynamic shapes.
    pub fn iter_dynamic_shapes(&self) -> impl Iterator<Item = &DynamicShape> {
        self.dynamic_shapes.iter()
    }

    /// Insert a dynamic shape.
    pub fn insert_dynamic_shape(&mut self, shape: DynamicShape) -> Option<DynamicShape> {
        if let Some(index) = self
            .dynamic_shapes
            .iter()
            .position(|candidate| candidate.constraint == shape.constraint)
        {
            return Some(mem::replace(&mut self.dynamic_shapes[index], shape));
        }

        self.dynamic_shapes.push(shape);

        None
    }
}

/// Virtual method table for one concrete type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VirtualTable {
    /// The concrete type owning this table.
    pub concrete: LocalNodeId<Type>,
    /// Method implementations in virtual slot order.
    pub methods: Vec<LocalNodeId<Function>>,
}

/// Table for one concrete implementation of one dynamic constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicTable {
    /// The concrete type providing the implementation.
    pub concrete: LocalNodeId<Type>,
    /// The dynamic constraint type being dispatched.
    pub constraint: LocalNodeId<Type>,
    /// Entries in dynamic shape order.
    pub entries: Vec<DynamicEntry>,
    /// The concrete field entries sorted by name for keyed finds.
    pub names: Vec<DynamicNamedEntry>,
}

/// One name-keyed entry in a dynamic dispatch table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicNamedEntry {
    /// The concrete field name.
    pub name: StringId,
    /// The entry backing the name.
    pub entry: DynamicEntry,
}

impl DynamicTable {
    /// Return the table slot for one dynamic slot index.
    pub const fn slot_for_index(index: usize) -> DispatchSlot {
        DispatchSlot(index as u32)
    }

    /// Return the storage slot count for a dynamic table.
    pub const fn storage_len(dynamic_slots: usize) -> usize {
        dynamic_slots
    }
}

/// Table shape for one dynamic constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicShape {
    /// The dynamic constraint type owning this shape.
    pub constraint: LocalNodeId<Type>,
    /// Slots in declaration order.
    pub slots: Vec<DynamicSlot>,
}

/// Entry in a dynamic dispatch table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DynamicEntry {
    /// Slot containing a field offset.
    Field {
        /// The field offset in bytes.
        offset: u32,
    },
    /// Slot containing a concrete function implementation.
    Function {
        /// The concrete function implementation.
        function: LocalNodeId<Function>,
    },
    /// Slot without a concrete member, read as undefined.
    Absent,
}

/// Slot descriptor for a dynamic shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DynamicSlot {
    /// Field slot.
    Field {
        /// The canonical dispatch field id.
        field: LocalNodeId<Field>,
        /// The field name.
        name: StringId,
    },
    /// Function slot.
    Function {
        /// The function name, absent for call signatures.
        name: Option<StringId>,
        /// The function signature.
        signature: LocalNodeId<Type>,
    },
}

/// Slot index inside a dispatch table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct DispatchSlot(pub u32);

impl DispatchSlot {
    /// Create a dispatch slot from a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw index for this slot.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}
