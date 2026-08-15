use std::mem;

use serde::{Deserialize, Serialize};

use destack_core::StringId;
use destack_serde::Reflect;

use crate::{Field, Function, LocalNodeId, Type};

/// Canonical dispatch table for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct DispatchTable {
    /// Virtual dispatch tables.
    virtual_tables: Vec<VirtualTable>,
    /// Dynamic dispatch tables.
    dynamic_tables: Vec<DynamicTable>,
    /// Dynamic dispatch shapes keyed by constraint type id.
    dynamic_shapes: Vec<DynamicShape>,
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
        let index = self
            .virtual_tables
            .binary_search_by_key(&table.concrete.get(), |candidate| candidate.concrete.get());

        match index {
            Ok(index) => Some(mem::replace(&mut self.virtual_tables[index], table)),
            Err(index) => {
                self.virtual_tables.insert(index, table);

                None
            }
        }
    }

    /// Return the virtual table for a type when present.
    pub fn virtual_table(&self, ty: LocalNodeId<Type>) -> Option<&VirtualTable> {
        self.virtual_tables
            .binary_search_by_key(&ty.get(), |table| table.concrete.get())
            .ok()
            .map(|index| &self.virtual_tables[index])
    }

    /// Iterate all virtual tables.
    pub fn iter_virtual_tables(&self) -> impl Iterator<Item = &VirtualTable> {
        self.virtual_tables.iter()
    }

    /// Insert a dynamic table.
    pub fn insert_dynamic_table(&mut self, table: DynamicTable) -> Option<DynamicTable> {
        let key = (table.concrete.get(), table.constraint.get());
        let index = self.dynamic_tables.binary_search_by_key(&key, |candidate| {
            (candidate.concrete.get(), candidate.constraint.get())
        });

        match index {
            Ok(index) => Some(mem::replace(&mut self.dynamic_tables[index], table)),
            Err(index) => {
                self.dynamic_tables.insert(index, table);

                None
            }
        }
    }

    /// Return the dynamic table for a concrete type and constraint when present.
    pub fn dynamic_table(
        &self,
        concrete: LocalNodeId<Type>,
        constraint: LocalNodeId<Type>,
    ) -> Option<&DynamicTable> {
        let key = (concrete.get(), constraint.get());

        self.dynamic_tables
            .binary_search_by_key(&key, |table| (table.concrete.get(), table.constraint.get()))
            .ok()
            .map(|index| &self.dynamic_tables[index])
    }

    /// Iterate all dynamic tables.
    pub fn iter_dynamic_tables(&self) -> impl Iterator<Item = &DynamicTable> {
        self.dynamic_tables.iter()
    }

    /// Return the dynamic shape for a constraint type id.
    pub fn dynamic_shape(&self, constraint: LocalNodeId<Type>) -> Option<&DynamicShape> {
        self.dynamic_shapes
            .binary_search_by_key(&constraint.get(), |shape| shape.constraint.get())
            .ok()
            .map(|index| &self.dynamic_shapes[index])
    }

    /// Iterate all dynamic shapes.
    pub fn iter_dynamic_shapes(&self) -> impl Iterator<Item = &DynamicShape> {
        self.dynamic_shapes.iter()
    }

    /// Insert a dynamic shape.
    pub fn insert_dynamic_shape(&mut self, shape: DynamicShape) -> Option<DynamicShape> {
        let index = self
            .dynamic_shapes
            .binary_search_by_key(&shape.constraint.get(), |candidate| {
                candidate.constraint.get()
            });

        match index {
            Ok(index) => Some(mem::replace(&mut self.dynamic_shapes[index], shape)),
            Err(index) => {
                self.dynamic_shapes.insert(index, shape);

                None
            }
        }
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
    /// Whether the constraint answers keyed finds by field name.
    pub is_keyed: bool,
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
