use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Field, Function, Global, LocalNodeId, Type};

/// Canonical dispatch metadata for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DispatchMetadata {
    /// Class dispatch tables.
    pub vtables: Vec<Vtable>,
    /// Class dispatch table vector index keyed by type id.
    #[serde(skip, default)]
    pub(crate) vtable_indices: HashMap<LocalNodeId<Type>, usize>,
    /// The interface dispatch tables.
    pub interface_tables: Vec<InterfaceTable>,
    /// The interface table vector index keyed by concrete type id, then interface type id.
    #[serde(skip, default)]
    pub(crate) interface_table_indices:
        HashMap<LocalNodeId<Type>, HashMap<LocalNodeId<Type>, usize>>,
    /// Interface slot layouts keyed by interface type id.
    pub interface_shapes: HashMap<LocalNodeId<Type>, InterfaceShape>,
}

impl DispatchMetadata {
    /// Create empty dispatch metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy dispatch metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(index) = self.vtable_index(from) {
            self.vtable_indices.insert(to, index);
        }

        let interface_tables = self
            .interface_tables
            .iter()
            .enumerate()
            .filter_map(|(index, table)| {
                (table.concrete == from).then_some((table.interface, index))
            })
            .collect::<HashMap<_, _>>();

        if !interface_tables.is_empty() {
            self.interface_table_indices.insert(to, interface_tables);
        }
    }

    /// Insert a vtable.
    pub fn insert_vtable(&mut self, table: Vtable) {
        let index = self.vtables.len();
        self.vtable_indices.insert(table.ty, index);
        self.vtables.push(table);
    }

    /// Return the vtable for a type when present.
    pub fn vtable(&self, ty: LocalNodeId<Type>) -> Option<&Vtable> {
        let index = self.vtable_index(ty)?;

        self.vtables.get(index)
    }

    /// Iterate all vtables.
    pub fn iter_vtables(&self) -> impl Iterator<Item = &Vtable> {
        self.vtables.iter()
    }

    /// Insert an interface table.
    pub fn insert_interface_table(&mut self, table: InterfaceTable) {
        let index = self.interface_tables.len();
        self.record_interface_table(table.concrete, table.interface, index);
        self.interface_tables.push(table);
    }

    /// Return the interface table for a concrete type and interface when present.
    pub fn interface_table(
        &self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
    ) -> Option<&InterfaceTable> {
        let index = self.interface_table_index(concrete, interface)?;

        self.interface_tables.get(index)
    }

    /// Iterate all interface tables.
    pub fn iter_interface_tables(&self) -> impl Iterator<Item = &InterfaceTable> {
        self.interface_tables.iter()
    }

    /// Return interface shape metadata for an interface type id.
    pub fn interface_shape(&self, interface: LocalNodeId<Type>) -> Option<&InterfaceShape> {
        self.interface_shapes.get(&interface)
    }

    /// Insert interface shape metadata for an interface type id.
    pub fn insert_interface_shape(
        &mut self,
        interface: LocalNodeId<Type>,
        shape: InterfaceShape,
    ) -> Option<InterfaceShape> {
        self.interface_shapes.insert(interface, shape)
    }

    /// Rebuild dispatch lookup indexes from canonical tables.
    pub fn rebuild_indices(&mut self) {
        self.vtable_indices.clear();
        self.interface_table_indices.clear();

        for (index, vtable) in self.vtables.iter().enumerate() {
            self.vtable_indices.insert(vtable.ty, index);
        }

        let table_entries = self
            .interface_tables
            .iter()
            .enumerate()
            .map(|(index, table)| (table.concrete, table.interface, index))
            .collect::<Vec<_>>();

        for (concrete, interface, index) in table_entries {
            self.record_interface_table(concrete, interface, index);
        }
    }

    /// Record one interface table lookup entry.
    fn record_interface_table(
        &mut self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
        index: usize,
    ) {
        self.interface_table_indices
            .entry(concrete)
            .or_default()
            .insert(interface, index);
    }

    /// Return the vtable vector index for one type.
    fn vtable_index(&self, ty: LocalNodeId<Type>) -> Option<usize> {
        self.vtable_indices
            .get(&ty)
            .copied()
            .or_else(|| self.vtables.iter().position(|vtable| vtable.ty == ty))
    }

    /// Return the interface table vector index for one concrete and interface pair.
    fn interface_table_index(
        &self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
    ) -> Option<usize> {
        self.interface_table_indices
            .get(&concrete)
            .and_then(|interface_tables| interface_tables.get(&interface))
            .copied()
            .or_else(|| {
                self.interface_tables
                    .iter()
                    .position(|table| table.concrete == concrete && table.interface == interface)
            })
    }
}

/// Metadata for a class vtable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vtable {
    /// The class type owning this table.
    pub ty: LocalNodeId<Type>,
    /// The static global containing this table.
    pub global: LocalNodeId<Global>,
    /// Entries in declaration order.
    pub entries: Vec<VtableEntry>,
}

/// Metadata for one concrete implementation of one interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceTable {
    /// The concrete type providing the implementation.
    pub concrete: LocalNodeId<Type>,
    /// The interface type being dispatched.
    pub interface: LocalNodeId<Type>,
    /// The static global containing this table.
    pub global: LocalNodeId<Global>,
    /// Slots in interface shape order.
    pub entries: Vec<InterfaceTableEntry>,
}

impl InterfaceTable {
    /// The first user-visible interface slot.
    pub const FIRST_SLOT: DispatchSlot = DispatchSlot(1);

    /// Return the table slot for one interface slot index.
    pub const fn slot_for_index(index: usize) -> DispatchSlot {
        DispatchSlot(Self::FIRST_SLOT.0 + index as u32)
    }

    /// Return the storage slot count for an interface table.
    pub const fn storage_len(interface_slots: usize) -> usize {
        Self::FIRST_SLOT.0 as usize + interface_slots
    }
}

/// Slot layout for one interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceShape {
    /// The interface type owning this shape.
    pub interface: LocalNodeId<Type>,
    /// User-visible slots in declaration order.
    pub slots: Vec<InterfaceSlot>,
}

/// Entry in a class vtable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VtableEntry {
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

/// Entry in an interface dispatch table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceTableEntry {
    /// Slot containing a field offset.
    FieldOffset {
        /// The field offset in bytes.
        offset: u32,
    },
    /// Slot containing a concrete method implementation.
    Method {
        /// The concrete method implementation.
        function: LocalNodeId<Function>,
    },
}

/// Slot descriptor for an interface layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceSlot {
    /// Field slot.
    Field {
        /// The canonical dispatch field id.
        field: LocalNodeId<Field>,
        /// The field name.
        field_name: StringId,
    },
    /// Method slot.
    Method {
        /// The method name.
        name: StringId,
        /// The method signature.
        signature: LocalNodeId<Type>,
    },
}

/// Slot index inside a dispatch table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DispatchSlot(
    /// Raw index into the dispatch table.
    pub u32,
);

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
