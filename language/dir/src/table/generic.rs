use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Arena, GenericInstance, GenericSlot, GlobalNodeIdAny, LocalGenericSlotId, LocalInstanceId,
    SegmentView,
};

/// Cumulative generic slots and instances for one DIR module.
#[derive(Debug, Clone)]
pub struct GenericTable<'a> {
    /// The module id of the generic table.
    pub module_id: ModuleId,
    /// The ordered generic table segments.
    segments: SegmentView<'a, GenericSegment>,
}

impl GenericTable<'static> {
    /// Create a generic table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<GenericSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a generic table from one segment.
    pub fn from_segment(segment: Arc<GenericSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> GenericTable<'a> {
    /// Create a generic table from a segment view.
    pub fn from_view(segments: SegmentView<'a, GenericSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("generic table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "generic table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a generic table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b GenericSegment) -> GenericTable<'b> {
        GenericTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate committed generic slots with their local ids.
    pub fn iter_slots(&self) -> impl Iterator<Item = (LocalGenericSlotId, &GenericSlot)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_slots())
    }

    /// Iterate committed instances with their local ids.
    pub fn iter_instances(&self) -> impl Iterator<Item = (LocalInstanceId, &GenericInstance)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_instances())
    }

    /// Return the instance attached to a source node.
    pub fn node_instance_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalInstanceId> {
        for segment in self.segments.iter().rev() {
            if let Some(instance_id) = segment.node_instance_id(node_id) {
                return Some(instance_id);
            }
        }

        None
    }

    /// Find one exact instance by shape.
    pub fn find_instance(&self, expected: &GenericInstance) -> Option<LocalInstanceId> {
        for (instance_id, instance) in self.iter_instances() {
            if instance == expected {
                return Some(instance_id);
            }
        }

        None
    }

    /// Intern one instance into a mutable tail segment.
    pub fn intern_instance(
        &self,
        tail: &mut GenericSegment,
        instance: GenericInstance,
    ) -> LocalInstanceId {
        assert_eq!(
            self.module_id, tail.module_id,
            "generic table tail belongs to a different module"
        );

        if let Some(instance_id) = self.find_instance(&instance) {
            return instance_id;
        }

        if let Some(instance_id) = tail.find_instance(&instance) {
            return instance_id;
        }

        tail.push_instance(instance)
    }

    /// Get a generic slot by id.
    pub fn get_slot(&self, slot_id: LocalGenericSlotId) -> &GenericSlot {
        for segment in self.segments.iter() {
            if let Some(slot) = segment.get_local_slot(slot_id) {
                return slot;
            }
        }

        panic!("DIR generic slot {slot_id:?} is not visible")
    }

    /// Get an instance by id.
    pub fn get_instance(&self, instance_id: LocalInstanceId) -> &GenericInstance {
        for segment in self.segments.iter() {
            if let Some(instance) = segment.get_local_instance(instance_id) {
                return instance;
            }
        }

        panic!("DIR generic instance {instance_id:?} is not visible")
    }

    /// Get the number of slots in the table.
    pub fn slot_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.slot_count())
            .unwrap_or(0)
    }

    /// Get the number of instances in the table.
    pub fn instance_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.instance_count())
            .unwrap_or(0)
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Generic slots and instances added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericSegment {
    /// The module id of the generic segment.
    pub module_id: ModuleId,
    /// The first generic slot id owned by this table segment.
    pub(crate) first_slot_id: u32,
    /// The first instance id owned by this table segment.
    pub(crate) first_instance_id: u32,
    /// Generic slots.
    pub(crate) slots: Arena<GenericSlot>,
    /// Interned generic instances.
    pub(crate) instances: Arena<GenericInstance>,
    /// Generic instances keyed by DIR node.
    pub(crate) nodes: IndexMap<GlobalNodeIdAny, LocalInstanceId>,
}

impl GenericSegment {
    /// Create a new generic segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_slot_id: 0,
            first_instance_id: 0,
            slots: Arena::new(),
            instances: Arena::new(),
            nodes: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing generic table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_slot_id: base.slot_count(),
            first_instance_id: base.instance_count(),
            slots: Arena::new(),
            instances: Arena::new(),
            nodes: IndexMap::new(),
        }
    }

    /// Append a generic slot to this segment.
    pub fn push_slot(&mut self, slot: GenericSlot) -> LocalGenericSlotId {
        let slot_id = LocalGenericSlotId::new(self.slot_count());
        self.slots.allocate(slot);

        slot_id
    }

    /// Append a generic instance to this segment.
    pub fn push_instance(&mut self, instance: GenericInstance) -> LocalInstanceId {
        let instance_id = LocalInstanceId::new(self.instance_count());
        self.instances.allocate(instance);

        instance_id
    }

    /// Attach an instance to a source node.
    pub fn set_node_instance(&mut self, node_id: GlobalNodeIdAny, instance_id: LocalInstanceId) {
        self.nodes.insert(node_id, instance_id);
    }

    /// Return the instance attached to a source node.
    pub fn node_instance_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalInstanceId> {
        self.nodes.get(&node_id).copied()
    }

    /// Iterate source nodes with their instances.
    pub fn node_instances(&self) -> impl Iterator<Item = (GlobalNodeIdAny, LocalInstanceId)> + '_ {
        self.nodes
            .iter()
            .map(|(node_id, instance_id)| (*node_id, *instance_id))
    }

    /// Return the number of source nodes with instances.
    pub fn node_instance_count(&self) -> usize {
        self.nodes.len()
    }

    /// Find one exact instance by shape.
    pub fn find_instance(&self, expected: &GenericInstance) -> Option<LocalInstanceId> {
        for (instance_id, instance) in self.iter_instances() {
            if instance == expected {
                return Some(instance_id);
            }
        }

        None
    }

    /// Get a generic slot by id.
    pub fn get_slot(&self, slot_id: LocalGenericSlotId) -> &GenericSlot {
        self.get_local_slot(slot_id).unwrap_or_else(|| {
            panic!("DIR generic slot {slot_id:?} is not allocated in this segment")
        })
    }

    /// Get an instance by id.
    pub fn get_instance(&self, instance_id: LocalInstanceId) -> &GenericInstance {
        self.get_local_instance(instance_id).unwrap_or_else(|| {
            panic!("DIR generic instance {instance_id:?} is not allocated in this segment")
        })
    }

    /// Iterate committed generic slots with their local ids.
    pub fn iter_slots(&self) -> impl Iterator<Item = (LocalGenericSlotId, &GenericSlot)> + '_ {
        (self.first_slot_id..self.slot_count()).map(|index| {
            let slot_id = LocalGenericSlotId::new(index);
            (slot_id, self.get_slot(slot_id))
        })
    }

    /// Iterate committed instances with their local ids.
    pub fn iter_instances(&self) -> impl Iterator<Item = (LocalInstanceId, &GenericInstance)> + '_ {
        (self.first_instance_id..self.instance_count()).map(|index| {
            let instance_id = LocalInstanceId::new(index);
            (instance_id, self.get_instance(instance_id))
        })
    }

    /// Get the number of slots in the segment.
    pub fn slot_count(&self) -> u32 {
        self.first_slot_id + self.slots.len() as u32
    }

    /// Get the number of instances in the segment.
    pub fn instance_count(&self) -> u32 {
        self.first_instance_id + self.instances.len() as u32
    }

    /// Return whether this segment has no entries.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty() && self.instances.is_empty() && self.nodes.is_empty()
    }

    /// Get a generic slot owned by this table segment.
    pub(crate) fn get_local_slot(&self, slot_id: LocalGenericSlotId) -> Option<&GenericSlot> {
        self.contains_slot_id(slot_id)
            .then(|| self.slots.get(slot_id.0 - self.first_slot_id))
    }

    /// Get an instance owned by this table segment.
    pub(crate) fn get_local_instance(
        &self,
        instance_id: LocalInstanceId,
    ) -> Option<&GenericInstance> {
        self.contains_instance_id(instance_id)
            .then(|| self.instances.get(instance_id.0 - self.first_instance_id))
    }

    /// Return whether this segment contains the given slot id.
    fn contains_slot_id(&self, slot_id: LocalGenericSlotId) -> bool {
        slot_id.0 >= self.first_slot_id && slot_id.0 < self.slot_count()
    }

    /// Return whether this segment contains the given instance id.
    fn contains_instance_id(&self, instance_id: LocalInstanceId) -> bool {
        instance_id.0 >= self.first_instance_id && instance_id.0 < self.instance_count()
    }
}
