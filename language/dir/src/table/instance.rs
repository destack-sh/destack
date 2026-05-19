use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Arena, GlobalNodeIdAny, Instantiation, LocalInstantiationId, SegmentView};

/// Cumulative generic instantiations for one DIR module.
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

    /// Iterate committed instantiations with their local ids.
    pub fn iter_instantiations(
        &self,
    ) -> impl Iterator<Item = (LocalInstantiationId, &Instantiation)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_instantiations())
    }

    /// Return the instantiation attached to a source node.
    pub fn node_instantiation_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalInstantiationId> {
        for segment in self.segments.iter().rev() {
            if let Some(instantiation_id) = segment.node_instantiation_id(node_id) {
                return Some(instantiation_id);
            }
        }

        None
    }

    /// Find one exact instantiation by shape.
    pub fn find_instantiation(&self, expected: &Instantiation) -> Option<LocalInstantiationId> {
        for (instantiation_id, instantiation) in self.iter_instantiations() {
            if instantiation == expected {
                return Some(instantiation_id);
            }
        }

        None
    }

    /// Intern one instantiation into a mutable tail segment.
    pub fn intern_instantiation(
        &self,
        tail: &mut InstanceSegment,
        instantiation: Instantiation,
    ) -> LocalInstantiationId {
        assert_eq!(
            self.module_id, tail.module_id,
            "instance table tail belongs to a different module"
        );

        if let Some(instantiation_id) = self.find_instantiation(&instantiation) {
            return instantiation_id;
        }

        if let Some(instantiation_id) = tail.find_instantiation(&instantiation) {
            return instantiation_id;
        }

        tail.push_instantiation(instantiation)
    }

    /// Get an instantiation by id.
    pub fn get_instantiation(&self, instantiation_id: LocalInstantiationId) -> &Instantiation {
        for segment in self.segments.iter() {
            if let Some(instantiation) = segment.get_local_instantiation(instantiation_id) {
                return instantiation;
            }
        }

        panic!("DIR instantiation {instantiation_id:?} is not visible")
    }

    /// Get the number of instantiations in the table.
    pub fn instantiation_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.instantiation_count())
            .unwrap_or(0)
    }

    /// Return true when this table has no instantiations.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Generic instantiations added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSegment {
    /// The module id of the instance segment.
    pub module_id: ModuleId,
    /// The first instantiation id owned by this table segment.
    pub(crate) first_instantiation_id: u32,
    /// Interned generic instantiations.
    pub(crate) instantiations: Arena<Instantiation>,
    /// Generic instantiations keyed by DIR node.
    pub(crate) nodes: IndexMap<GlobalNodeIdAny, LocalInstantiationId>,
}

impl InstanceSegment {
    /// Create a new instance segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_instantiation_id: 0,
            instantiations: Arena::new(),
            nodes: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing instance table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_instantiation_id: base.instantiation_count(),
            instantiations: Arena::new(),
            nodes: IndexMap::new(),
        }
    }

    /// Append a generic instantiation to this segment.
    pub fn push_instantiation(&mut self, instantiation: Instantiation) -> LocalInstantiationId {
        let instantiation_id = LocalInstantiationId::new(self.instantiation_count());
        self.instantiations.allocate(instantiation);

        instantiation_id
    }

    /// Attach an instantiation to a source node.
    pub fn set_node_instantiation(
        &mut self,
        node_id: GlobalNodeIdAny,
        instantiation_id: LocalInstantiationId,
    ) {
        self.nodes.insert(node_id, instantiation_id);
    }

    /// Return the instantiation attached to a source node.
    pub fn node_instantiation_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalInstantiationId> {
        self.nodes.get(&node_id).copied()
    }

    /// Iterate source nodes with their instantiations.
    pub fn node_instantiations(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalInstantiationId)> + '_ {
        self.nodes
            .iter()
            .map(|(node_id, instantiation_id)| (*node_id, *instantiation_id))
    }

    /// Return the number of source nodes with instantiations.
    pub fn node_instantiation_count(&self) -> usize {
        self.nodes.len()
    }

    /// Find one exact instantiation by shape.
    pub fn find_instantiation(&self, expected: &Instantiation) -> Option<LocalInstantiationId> {
        for (instantiation_id, instantiation) in self.iter_instantiations() {
            if instantiation == expected {
                return Some(instantiation_id);
            }
        }

        None
    }

    /// Get an instantiation by id.
    pub fn get_instantiation(&self, instantiation_id: LocalInstantiationId) -> &Instantiation {
        self.get_local_instantiation(instantiation_id)
            .unwrap_or_else(|| {
                panic!("DIR instantiation {instantiation_id:?} is not allocated in this segment")
            })
    }

    /// Iterate committed instantiations with their local ids.
    pub fn iter_instantiations(
        &self,
    ) -> impl Iterator<Item = (LocalInstantiationId, &Instantiation)> + '_ {
        (self.first_instantiation_id..self.instantiation_count()).map(|index| {
            let instantiation_id = LocalInstantiationId::new(index);
            (instantiation_id, self.get_instantiation(instantiation_id))
        })
    }

    /// Get the number of instantiations in the segment.
    pub fn instantiation_count(&self) -> u32 {
        self.first_instantiation_id + self.instantiations.len() as u32
    }

    /// Return whether this segment has no instantiations.
    pub fn is_empty(&self) -> bool {
        self.instantiations.is_empty() && self.nodes.is_empty()
    }

    /// Get an instantiation owned by this table segment.
    pub(crate) fn get_local_instantiation(
        &self,
        instantiation_id: LocalInstantiationId,
    ) -> Option<&Instantiation> {
        self.contains_instantiation_id(instantiation_id).then(|| {
            self.instantiations
                .get(instantiation_id.0 - self.first_instantiation_id)
        })
    }

    /// Return whether this segment contains the given instantiation id.
    fn contains_instantiation_id(&self, instantiation_id: LocalInstantiationId) -> bool {
        instantiation_id.0 >= self.first_instantiation_id
            && instantiation_id.0 < self.instantiation_count()
    }
}
