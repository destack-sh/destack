use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Field, Function, Global, LocalNodeId, Type};

/// Canonical dispatch facts for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DispatchMetadata {
    /// Class vtables keyed by `VtableId` index.
    pub vtables: Vec<Vtable>,
    /// Canonical class vtable ids keyed by type id.
    pub(crate) vtable_id_by_type: HashMap<LocalNodeId<Type>, VtableId>,
    /// Interface itabs keyed by `ItabId` index.
    pub itabs: Vec<Itab>,
    /// Canonical interface itab ids keyed by concrete type id, then interface type id.
    pub(crate) itab_id_by_type: HashMap<LocalNodeId<Type>, HashMap<LocalNodeId<Type>, ItabId>>,
    /// Canonical interface dispatch shapes keyed by interface type id.
    pub interface_dispatch_shapes: HashMap<LocalNodeId<Type>, InterfaceDispatchShape>,
}

impl DispatchMetadata {
    /// Create a new empty dispatch table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy dispatch metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(vtable_id) = self.vtable_id(from) {
            self.vtable_id_by_type.insert(to, vtable_id);
        }

        if let Some(itabs) = self.itab_id_by_type.get(&from).cloned() {
            self.itab_id_by_type.insert(to, itabs);
        }
    }
    /// Insert a vtable and return its id.
    pub fn insert_vtable(&mut self, table: Vtable) -> VtableId {
        let id = VtableId::new(self.vtables.len() as u32);
        self.vtable_id_by_type.insert(table.ty, id);
        self.vtables.push(table);
        id
    }

    /// Insert a vtable at a specific id.
    pub fn insert_vtable_at(&mut self, id: VtableId, table: Vtable) {
        let index = id.index();
        if index > self.vtables.len() {
            panic!("vtable index {index} out of order");
        }
        if index < self.vtables.len() {
            panic!("vtable index {index} already populated");
        }
        self.vtable_id_by_type.insert(table.ty, id);
        self.vtables.push(table);
    }

    /// Return the vtable for an id.
    pub fn vtable(&self, id: VtableId) -> &Vtable {
        self.vtables
            .get(id.index())
            .unwrap_or_else(|| panic!("missing vtable entry {}", id.index()))
    }

    /// Iterate all populated vtables.
    pub fn iter_vtables(&self) -> impl Iterator<Item = (VtableId, &Vtable)> {
        self.vtables
            .iter()
            .enumerate()
            .map(|(index, table)| (VtableId::new(index as u32), table))
    }

    /// Insert an itab and return its id.
    pub fn insert_itab(&mut self, table: Itab) -> ItabId {
        let id = ItabId::new(self.itabs.len() as u32);
        self.record_itab_id(table.concrete, table.interface, id);
        self.itabs.push(table);
        id
    }

    /// Insert an itab at a specific id.
    pub fn insert_itab_at(&mut self, id: ItabId, table: Itab) {
        let index = id.index();
        if index > self.itabs.len() {
            panic!("itab index {index} out of order");
        }
        if index < self.itabs.len() {
            panic!("itab index {index} already populated");
        }
        self.record_itab_id(table.concrete, table.interface, id);
        self.itabs.push(table);
    }

    /// Return the itab for an id.
    pub fn itab(&self, id: ItabId) -> &Itab {
        self.itabs
            .get(id.index())
            .unwrap_or_else(|| panic!("missing itab entry {}", id.index()))
    }

    /// Iterate all populated itabs.
    pub fn iter_itabs(&self) -> impl Iterator<Item = (ItabId, &Itab)> {
        self.itabs
            .iter()
            .enumerate()
            .map(|(index, table)| (ItabId::new(index as u32), table))
    }

    /// Return dispatch shape metadata for an interface type id.
    pub fn interface_dispatch_shape(
        &self,
        interface: LocalNodeId<Type>,
    ) -> Option<&InterfaceDispatchShape> {
        self.interface_dispatch_shapes.get(&interface)
    }

    /// Return mutable dispatch shape metadata for an interface type id.
    pub fn interface_dispatch_shape_mut(
        &mut self,
        interface: LocalNodeId<Type>,
    ) -> Option<&mut InterfaceDispatchShape> {
        self.interface_dispatch_shapes.get_mut(&interface)
    }

    /// Insert dispatch shape metadata for an interface type id.
    pub fn insert_interface_dispatch_shape(
        &mut self,
        interface: LocalNodeId<Type>,
        shape: InterfaceDispatchShape,
    ) -> Option<InterfaceDispatchShape> {
        self.interface_dispatch_shapes.insert(interface, shape)
    }

    /// Remove dispatch shape metadata for an interface type id.
    pub fn remove_interface_dispatch_shape(
        &mut self,
        interface: LocalNodeId<Type>,
    ) -> Option<InterfaceDispatchShape> {
        self.interface_dispatch_shapes.remove(&interface)
    }

    /// Return the class vtable id for a type when present.
    pub fn vtable_id(&self, ty: LocalNodeId<Type>) -> Option<VtableId> {
        if let Some(vtable_id) = self.vtable_id_by_type.get(&ty) {
            return Some(*vtable_id);
        }

        self.vtables
            .iter()
            .enumerate()
            .find_map(|(index, vtable)| (vtable.ty == ty).then_some(VtableId::new(index as u32)))
    }

    /// Return the itab id for a concrete type and interface when present.
    pub fn itab_id(
        &self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
    ) -> Option<ItabId> {
        if let Some(itab_id) = self
            .itab_id_by_type
            .get(&concrete)
            .and_then(|itabs| itabs.get(&interface))
        {
            return Some(*itab_id);
        }

        self.itabs.iter().enumerate().find_map(|(index, itab)| {
            (itab.concrete == concrete && itab.interface == interface)
                .then_some(ItabId::new(index as u32))
        })
    }

    /// Rebuild dispatch lookup indexes from canonical tables.
    pub fn rebuild_lookup_index(&mut self) {
        self.vtable_id_by_type.clear();
        self.itab_id_by_type.clear();

        for (index, vtable) in self.vtables.iter().enumerate() {
            let vtable_id = VtableId::new(index as u32);
            self.vtable_id_by_type.insert(vtable.ty, vtable_id);
        }

        let itab_entries = self
            .itabs
            .iter()
            .enumerate()
            .map(|(index, itab)| (itab.concrete, itab.interface, ItabId::new(index as u32)))
            .collect::<Vec<_>>();

        for (concrete, interface, itab_id) in itab_entries {
            self.record_itab_id(concrete, interface, itab_id);
        }
    }

    /// Record one cached itab lookup entry.
    fn record_itab_id(
        &mut self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
        itab_id: ItabId,
    ) {
        self.itab_id_by_type
            .entry(concrete)
            .or_default()
            .insert(interface, itab_id);
    }
}

/// Identifier for a vtable entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VtableId(
    /// Raw index into the vtable table.
    u32,
);

impl VtableId {
    /// Create a vtable id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for a vtable method slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VtableSlotId(
    /// Raw index into the vtable entry list.
    pub u32,
);

impl VtableSlotId {
    /// Create a vtable slot id from a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw index for this id.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for an itab entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItabId(
    /// Raw index into the itab table.
    u32,
);

impl ItabId {
    /// Create an itab id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for an interface dispatch slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InterfaceSlotId(
    /// Raw index into the interface dispatch entry list.
    pub u32,
);

impl InterfaceSlotId {
    /// Create an interface slot id from a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw index for this id.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
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

/// Canonical interface dispatch shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceDispatchShape {
    /// The interface type owning this shape.
    pub interface: LocalNodeId<Type>,
    /// Entries in declaration order.
    pub entries: Vec<InterfaceDispatchEntry>,
}

/// Metadata for a class vtable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vtable {
    /// The class type owning this table.
    pub ty: LocalNodeId<Type>,
    /// Storage backing for this vtable.
    pub storage: VtableStorage,
    /// Entries in declaration order.
    pub entries: Vec<VtableEntry>,
}

/// Storage backing for a class vtable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VtableStorage {
    /// Global data symbol containing the vtable entries.
    Global(LocalNodeId<Global>),
}

/// Metadata for an interface itab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Itab {
    /// The concrete type providing the implementation.
    pub concrete: LocalNodeId<Type>,
    /// The interface type being dispatched.
    pub interface: LocalNodeId<Type>,
    /// Storage backing for this itab.
    pub storage: ItabStorage,
    /// Entries in declaration order.
    pub entries: Vec<ItabEntry>,
}

/// Storage backing for an interface itab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItabStorage {
    /// Immediate handle encoded as an itab id.
    Handle,
}
