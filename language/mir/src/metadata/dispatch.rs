use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Field, Function, Global, LocalNodeId, Type};

/// Canonical dispatch facts for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DispatchMetadata {
    /// Class dispatch tables.
    pub vtables: Vec<Vtable>,
    /// Class dispatch table vector index keyed by type id.
    #[serde(skip, default)]
    pub(crate) vtable_index_by_type: HashMap<LocalNodeId<Type>, usize>,
    /// Interface dispatch tables.
    pub itabs: Vec<Itab>,
    /// Interface dispatch table vector index keyed by concrete type id, then interface type id.
    #[serde(skip, default)]
    pub(crate) itab_index_by_type: HashMap<LocalNodeId<Type>, HashMap<LocalNodeId<Type>, usize>>,
    /// Canonical interface dispatch shapes keyed by interface type id.
    pub interface_dispatch_shapes: HashMap<LocalNodeId<Type>, InterfaceDispatchShape>,
}

impl DispatchMetadata {
    /// Create empty dispatch metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy dispatch metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(index) = self.vtable_index(from) {
            self.vtable_index_by_type.insert(to, index);
        }

        let itabs = self
            .itabs
            .iter()
            .enumerate()
            .filter_map(|(index, itab)| (itab.concrete == from).then_some((itab.interface, index)))
            .collect::<HashMap<_, _>>();

        if !itabs.is_empty() {
            self.itab_index_by_type.insert(to, itabs);
        }
    }

    /// Insert a vtable.
    pub fn insert_vtable(&mut self, table: Vtable) {
        let index = self.vtables.len();
        self.vtable_index_by_type.insert(table.ty, index);
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

    /// Insert an itab.
    pub fn insert_itab(&mut self, table: Itab) {
        let index = self.itabs.len();
        self.record_itab(table.concrete, table.interface, index);
        self.itabs.push(table);
    }

    /// Return the itab for a concrete type and interface when present.
    pub fn itab(&self, concrete: LocalNodeId<Type>, interface: LocalNodeId<Type>) -> Option<&Itab> {
        let index = self.itab_index(concrete, interface)?;

        self.itabs.get(index)
    }

    /// Iterate all itabs.
    pub fn iter_itabs(&self) -> impl Iterator<Item = &Itab> {
        self.itabs.iter()
    }

    /// Return dispatch shape metadata for an interface type id.
    pub fn interface_dispatch_shape(
        &self,
        interface: LocalNodeId<Type>,
    ) -> Option<&InterfaceDispatchShape> {
        self.interface_dispatch_shapes.get(&interface)
    }

    /// Insert dispatch shape metadata for an interface type id.
    pub fn insert_interface_dispatch_shape(
        &mut self,
        interface: LocalNodeId<Type>,
        shape: InterfaceDispatchShape,
    ) -> Option<InterfaceDispatchShape> {
        self.interface_dispatch_shapes.insert(interface, shape)
    }

    /// Rebuild dispatch lookup indexes from canonical tables.
    pub fn rebuild_lookup_index(&mut self) {
        self.vtable_index_by_type.clear();
        self.itab_index_by_type.clear();

        for (index, vtable) in self.vtables.iter().enumerate() {
            self.vtable_index_by_type.insert(vtable.ty, index);
        }

        let itab_entries = self
            .itabs
            .iter()
            .enumerate()
            .map(|(index, itab)| (itab.concrete, itab.interface, index))
            .collect::<Vec<_>>();

        for (concrete, interface, index) in itab_entries {
            self.record_itab(concrete, interface, index);
        }
    }

    /// Record one itab lookup entry.
    fn record_itab(
        &mut self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
        index: usize,
    ) {
        self.itab_index_by_type
            .entry(concrete)
            .or_default()
            .insert(interface, index);
    }

    /// Return the vtable vector index for one type.
    fn vtable_index(&self, ty: LocalNodeId<Type>) -> Option<usize> {
        self.vtable_index_by_type
            .get(&ty)
            .copied()
            .or_else(|| self.vtables.iter().position(|vtable| vtable.ty == ty))
    }

    /// Return the itab vector index for one concrete and interface pair.
    fn itab_index(
        &self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
    ) -> Option<usize> {
        self.itab_index_by_type
            .get(&concrete)
            .and_then(|itabs| itabs.get(&interface))
            .copied()
            .or_else(|| {
                self.itabs
                    .iter()
                    .position(|itab| itab.concrete == concrete && itab.interface == interface)
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

/// Metadata for an interface itab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Itab {
    /// The concrete type providing the implementation.
    pub concrete: LocalNodeId<Type>,
    /// The interface type being dispatched.
    pub interface: LocalNodeId<Type>,
    /// The static global containing this table.
    pub global: LocalNodeId<Global>,
    /// Entries in declaration order.
    pub entries: Vec<ItabEntry>,
}

/// Canonical interface dispatch shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceDispatchShape {
    /// The interface type owning this shape.
    pub interface: LocalNodeId<Type>,
    /// Entries in declaration order.
    pub entries: Vec<InterfaceDispatchEntry>,
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

/// Entry in an interface itab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItabEntry {
    /// Slot containing the runtime type descriptor.
    TypeDescriptor,
    /// Slot containing an interface field offset.
    FieldOffset {
        /// The canonical interface dispatch field id.
        field: LocalNodeId<Field>,
        /// The interface field name.
        field_name: StringId,
        /// The field offset in bytes.
        offset: u32,
    },
    /// Slot mapping interface method declaration to target method.
    Method {
        /// The declared interface method.
        declared_method: LocalNodeId<Function>,
        /// The concrete method implementation.
        target_method: LocalNodeId<Function>,
    },
}

/// Slot descriptor for interface dispatch layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceDispatchEntry {
    /// Slot containing the runtime type descriptor.
    TypeDescriptor,
    /// Slot containing an interface field offset.
    FieldOffset {
        /// The canonical interface dispatch field id.
        field: LocalNodeId<Field>,
        /// The interface field name.
        field_name: StringId,
    },
    /// Slot containing an interface method declaration.
    Method {
        /// The declared interface method.
        declared_method: LocalNodeId<Function>,
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
