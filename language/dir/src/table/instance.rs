use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Arena, GlobalNodeIdAny, Instance, LocalInstanceId, SegmentView};

/// Cumulative generic instances for one DIR module.
#[derive(Debug, Clone)]
pub struct InstanceTable<'a> {
    /// The module id of the instance table.
    pub module_id: ModuleId,
    /// The ordered instance table segments.
    segments: SegmentView<'a, InstanceSegment>,
}

impl InstanceTable<'static> {
    /// Create an instance table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<InstanceSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create an instance table from one segment.
    pub fn from_segment(segment: Arc<InstanceSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> InstanceTable<'a> {
    /// Create an instance table from a segment view.
    pub fn from_view(segments: SegmentView<'a, InstanceSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("instance table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "instance table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create an instance table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b InstanceSegment) -> InstanceTable<'b> {
        InstanceTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate committed instances with their local ids.
    pub fn iter_instances(&self) -> impl Iterator<Item = (LocalInstanceId, &Instance)> + '_ {
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
    pub fn find_instance(&self, expected: &Instance) -> Option<LocalInstanceId> {
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
        tail: &mut InstanceSegment,
        instance: Instance,
    ) -> LocalInstanceId {
        assert_eq!(
            self.module_id, tail.module_id,
            "instance table tail belongs to a different module"
        );

        if let Some(instance_id) = self.find_instance(&instance) {
            return instance_id;
        }

        if let Some(instance_id) = tail.find_instance(&instance) {
            return instance_id;
        }

        tail.push_instance(instance)
    }

    /// Get an instance by id.
    pub fn get_instance(&self, instance_id: LocalInstanceId) -> &Instance {
        for segment in self.segments.iter() {
            if let Some(instance) = segment.get_local_instance(instance_id) {
                return instance;
            }
        }

        panic!("DIR instance {instance_id:?} is not visible")
    }

    /// Get the number of instances in the table.
    pub fn instance_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.instance_count())
            .unwrap_or(0)
    }

    /// Return true when this table has no instances.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Generic instances added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSegment {
    /// The module id of the instance segment.
    pub module_id: ModuleId,
    /// The first instance id owned by this table segment.
    pub(crate) first_instance_id: u32,
    /// Interned generic instances.
    pub(crate) instances: Arena<Instance>,
    /// Generic instances keyed by DIR node.
    pub(crate) nodes: IndexMap<GlobalNodeIdAny, LocalInstanceId>,
}

impl InstanceSegment {
    /// Create a new instance segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_instance_id: 0,
            instances: Arena::new(),
            nodes: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing instance table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_instance_id: base.instance_count(),
            instances: Arena::new(),
            nodes: IndexMap::new(),
        }
    }

    /// Append a generic instance to this segment.
    pub fn push_instance(&mut self, instance: Instance) -> LocalInstanceId {
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
    pub fn find_instance(&self, expected: &Instance) -> Option<LocalInstanceId> {
        for (instance_id, instance) in self.iter_instances() {
            if instance == expected {
                return Some(instance_id);
            }
        }

        None
    }

    /// Get an instance by id.
    pub fn get_instance(&self, instance_id: LocalInstanceId) -> &Instance {
        self.get_local_instance(instance_id).unwrap_or_else(|| {
            panic!("DIR instance {instance_id:?} is not allocated in this segment")
        })
    }

    /// Iterate committed instances with their local ids.
    pub fn iter_instances(&self) -> impl Iterator<Item = (LocalInstanceId, &Instance)> + '_ {
        (self.first_instance_id..self.instance_count()).map(|index| {
            let instance_id = LocalInstanceId::new(index);
            (instance_id, self.get_instance(instance_id))
        })
    }

    /// Get the number of instances in the segment.
    pub fn instance_count(&self) -> u32 {
        self.first_instance_id + self.instances.len() as u32
    }

    /// Return whether this segment has no instances.
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty() && self.nodes.is_empty()
    }

    /// Get an instance owned by this table segment.
    pub(crate) fn get_local_instance(&self, instance_id: LocalInstanceId) -> Option<&Instance> {
        self.contains_instance_id(instance_id)
            .then(|| self.instances.get(instance_id.0 - self.first_instance_id))
    }

    /// Return whether this segment contains the given instance id.
    fn contains_instance_id(&self, instance_id: LocalInstanceId) -> bool {
        instance_id.0 >= self.first_instance_id && instance_id.0 < self.instance_count()
    }
}
