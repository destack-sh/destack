use destack_serde::Reflect;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Field, Function, Global, LocalNodeId, Type};

/// Canonical dispatch metadata for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct DispatchMetadata {
    /// Virtual dispatch tables.
    pub virtual_tables: Vec<VirtualTable>,
    /// Virtual dispatch table vector index keyed by type id.
    #[serde(skip, default)]
    pub(crate) virtual_table_indices: HashMap<LocalNodeId<Type>, usize>,
    /// Dynamic dispatch tables.
    pub dynamic_tables: Vec<DynamicTable>,
    /// Dynamic table vector index keyed by concrete type id, then constraint type id.
    #[serde(skip, default)]
    pub(crate) dynamic_table_indices: HashMap<LocalNodeId<Type>, HashMap<LocalNodeId<Type>, usize>>,
    /// Dynamic dispatch layouts keyed by constraint type id.
    pub dynamic_layouts: HashMap<LocalNodeId<Type>, DynamicLayout>,
}

impl DispatchMetadata {
    /// Create empty dispatch metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy dispatch metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(index) = self.virtual_table_index(from) {
            self.virtual_table_indices.insert(to, index);
        }

        let dynamic_tables = self
            .dynamic_tables
            .iter()
            .enumerate()
            .filter_map(|(index, table)| {
                (table.concrete == from).then_some((table.constraint, index))
            })
            .collect::<HashMap<_, _>>();

        if !dynamic_tables.is_empty() {
            self.dynamic_table_indices.insert(to, dynamic_tables);
        }
    }

    /// Insert a virtual table.
    pub fn insert_virtual_table(&mut self, table: VirtualTable) {
        let index = self.virtual_tables.len();
        self.virtual_table_indices.insert(table.ty, index);
        self.virtual_tables.push(table);
    }

    /// Return the virtual table for a type when present.
    pub fn virtual_table(&self, ty: LocalNodeId<Type>) -> Option<&VirtualTable> {
        let index = self.virtual_table_index(ty)?;

        self.virtual_tables.get(index)
    }

    /// Iterate all virtual tables.
    pub fn iter_virtual_tables(&self) -> impl Iterator<Item = &VirtualTable> {
        self.virtual_tables.iter()
    }

    /// Insert a dynamic table.
    pub fn insert_dynamic_table(&mut self, table: DynamicTable) {
        let index = self.dynamic_tables.len();
        self.record_dynamic_table(table.concrete, table.constraint, index);
        self.dynamic_tables.push(table);
    }

    /// Return the dynamic table for a concrete type and constraint when present.
    pub fn dynamic_table(
        &self,
        concrete: LocalNodeId<Type>,
        constraint: LocalNodeId<Type>,
    ) -> Option<&DynamicTable> {
        let index = self.dynamic_table_index(concrete, constraint)?;

        self.dynamic_tables.get(index)
    }

    /// Iterate all dynamic tables.
    pub fn iter_dynamic_tables(&self) -> impl Iterator<Item = &DynamicTable> {
        self.dynamic_tables.iter()
    }

    /// Return dynamic layout metadata for a constraint type id.
    pub fn dynamic_layout(&self, constraint: LocalNodeId<Type>) -> Option<&DynamicLayout> {
        self.dynamic_layouts.get(&constraint)
    }

    /// Insert dynamic layout metadata for a constraint type id.
    pub fn insert_dynamic_layout(
        &mut self,
        constraint: LocalNodeId<Type>,
        layout: DynamicLayout,
    ) -> Option<DynamicLayout> {
        self.dynamic_layouts.insert(constraint, layout)
    }

    /// Rebuild dispatch lookup indexes from canonical tables.
    pub fn rebuild_indices(&mut self) {
        self.virtual_table_indices.clear();
        self.dynamic_table_indices.clear();

        for (index, table) in self.virtual_tables.iter().enumerate() {
            self.virtual_table_indices.insert(table.ty, index);
        }

        let table_entries = self
            .dynamic_tables
            .iter()
            .enumerate()
            .map(|(index, table)| (table.concrete, table.constraint, index))
            .collect::<Vec<_>>();

        for (concrete, constraint, index) in table_entries {
            self.record_dynamic_table(concrete, constraint, index);
        }
    }

    /// Record one dynamic table lookup entry.
    fn record_dynamic_table(
        &mut self,
        concrete: LocalNodeId<Type>,
        constraint: LocalNodeId<Type>,
        index: usize,
    ) {
        self.dynamic_table_indices
            .entry(concrete)
            .or_default()
            .insert(constraint, index);
    }

    /// Return the virtual table vector index for one type.
    fn virtual_table_index(&self, ty: LocalNodeId<Type>) -> Option<usize> {
        self.virtual_table_indices
            .get(&ty)
            .copied()
            .or_else(|| self.virtual_tables.iter().position(|table| table.ty == ty))
    }

    /// Return the dynamic table vector index for one concrete and constraint pair.
    fn dynamic_table_index(
        &self,
        concrete: LocalNodeId<Type>,
        constraint: LocalNodeId<Type>,
    ) -> Option<usize> {
        self.dynamic_table_indices
            .get(&concrete)
            .and_then(|dynamic_tables| dynamic_tables.get(&constraint))
            .copied()
            .or_else(|| {
                self.dynamic_tables
                    .iter()
                    .position(|table| table.concrete == concrete && table.constraint == constraint)
            })
    }
}

/// Metadata for one class virtual table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VirtualTable {
    /// The class type owning this table.
    pub ty: LocalNodeId<Type>,
    /// The static global containing this table.
    pub global: LocalNodeId<Global>,
    /// Entries in declaration order.
    pub entries: Vec<VirtualEntry>,
}

/// Metadata for one concrete implementation of one dynamic constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicTable {
    /// The concrete type providing the implementation.
    pub concrete: LocalNodeId<Type>,
    /// The dynamic constraint type being dispatched.
    pub constraint: LocalNodeId<Type>,
    /// The static global containing this table.
    pub global: LocalNodeId<Global>,
    /// Entries in dynamic layout order.
    pub entries: Vec<DynamicEntry>,
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

/// Slot layout for one dynamic constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicLayout {
    /// The dynamic constraint type owning this layout.
    pub constraint: LocalNodeId<Type>,
    /// Slots in declaration order.
    pub slots: Vec<DynamicSlot>,
}

/// Entry in a class virtual table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum VirtualEntry {
    /// Slot containing the runtime type descriptor.
    TypeDescriptor,
    /// Slot containing a drop glue function.
    Destructor {
        /// The drop glue function when present.
        function: Option<LocalNodeId<Function>>,
    },
    /// Slot containing a method implementation.
    Method {
        /// The concrete method implementation.
        function: LocalNodeId<Function>,
    },
}

/// Entry in a dynamic dispatch table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DynamicEntry {
    /// Slot containing a field offset.
    Field {
        /// The field offset in bytes.
        offset: u32,
    },
    /// Slot containing a concrete getter implementation.
    Getter {
        /// The concrete getter implementation.
        function: LocalNodeId<Function>,
    },
    /// Slot containing a concrete setter implementation.
    Setter {
        /// The concrete setter implementation.
        function: LocalNodeId<Function>,
    },
    /// Slot containing a concrete method implementation.
    Method {
        /// The concrete method implementation.
        function: LocalNodeId<Function>,
    },
    /// Slot containing a concrete call implementation.
    Call {
        /// The concrete call implementation.
        function: LocalNodeId<Function>,
    },
}

/// Slot descriptor for a dynamic layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DynamicSlot {
    /// Field slot.
    Field {
        /// The canonical dispatch field id.
        field: LocalNodeId<Field>,
        /// The field name.
        name: StringId,
    },
    /// Getter slot.
    Getter {
        /// The getter name.
        name: StringId,
        /// The getter signature.
        signature: LocalNodeId<Type>,
    },
    /// Setter slot.
    Setter {
        /// The setter name.
        name: StringId,
        /// The setter signature.
        signature: LocalNodeId<Type>,
    },
    /// Method slot.
    Method {
        /// The method name.
        name: StringId,
        /// The method signature.
        signature: LocalNodeId<Type>,
    },
    /// Call signature slot.
    Call {
        /// The call signature.
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
