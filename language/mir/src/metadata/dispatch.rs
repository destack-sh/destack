use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    Block, Function, Instruction, InterfaceDispatchShape, Itab, ItabId, LocalNodeId, Type, Vtable,
    VtableId,
};

/// Dynamic callsite key for dispatch metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallSite {
    /// Dynamic callsite represented by a call instruction.
    Instruction(LocalNodeId<Instruction>),
    /// Dynamic callsite represented by a terminator in a block.
    Terminator(LocalNodeId<Block>),
}

/// Optimization metadata attached to a dynamic dispatch callsite.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DevirtualizationMetadata {
    /// The source level declared method for this callsite, when known.
    pub declared_target: Option<LocalNodeId<Function>>,
}

impl DevirtualizationMetadata {
    /// Return whether this metadata carries any devirtualization facts.
    pub fn is_empty(&self) -> bool {
        self.declared_target.is_none()
    }
}

/// Table of dispatch metadata entries.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DispatchTable {
    /// Class vtables keyed by `VtableId` index.
    pub vtables: Vec<Vtable>,
    /// Interface itabs keyed by `ItabId` index.
    pub itabs: Vec<Itab>,
    /// Canonical interface dispatch shapes keyed by interface type id.
    pub interface_dispatch_shapes: HashMap<LocalNodeId<Type>, InterfaceDispatchShape>,
    /// Sparse devirtualization metadata keyed by dynamic callsite.
    pub callsite_metadata: HashMap<CallSite, DevirtualizationMetadata>,
}

impl DispatchTable {
    /// Create a new empty dispatch table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a vtable and return its id.
    pub fn insert_vtable(&mut self, table: Vtable) -> VtableId {
        let id = VtableId::new(self.vtables.len() as u32);
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

    /// Return callsite metadata for a dynamic dispatch callsite.
    pub fn callsite_metadata(&self, callsite: CallSite) -> Option<&DevirtualizationMetadata> {
        self.callsite_metadata.get(&callsite)
    }

    /// Return mutable callsite metadata for a dynamic dispatch callsite.
    pub fn callsite_metadata_mut(
        &mut self,
        callsite: CallSite,
    ) -> Option<&mut DevirtualizationMetadata> {
        self.callsite_metadata.get_mut(&callsite)
    }

    /// Insert callsite metadata for a dynamic dispatch callsite.
    pub fn insert_callsite_metadata(
        &mut self,
        callsite: CallSite,
        metadata: DevirtualizationMetadata,
    ) -> Option<DevirtualizationMetadata> {
        self.callsite_metadata.insert(callsite, metadata)
    }

    /// Remove callsite metadata for a dynamic dispatch callsite.
    pub fn remove_callsite_metadata(
        &mut self,
        callsite: CallSite,
    ) -> Option<DevirtualizationMetadata> {
        self.callsite_metadata.remove(&callsite)
    }
}
