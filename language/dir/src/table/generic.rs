use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Arena, GenericApplication, GenericSlot, GenericTemplate, GlobalNodeIdAny,
    LocalGenericApplicationId, LocalGenericSlotId, LocalGenericTemplateId, SegmentView,
};

/// Cumulative generic slots and applications for one DIR module.
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

    /// Iterate committed generic templates with their local ids.
    pub fn iter_templates(
        &self,
    ) -> impl Iterator<Item = (LocalGenericTemplateId, &GenericTemplate)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_templates())
    }

    /// Iterate committed generic slots with their local ids.
    pub fn iter_slots(&self) -> impl Iterator<Item = (LocalGenericSlotId, &GenericSlot)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_slots())
    }

    /// Iterate committed generic applications with their local ids.
    pub fn iter_applications(
        &self,
    ) -> impl Iterator<Item = (LocalGenericApplicationId, &GenericApplication)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_applications())
    }

    /// Return the generic application attached to a source node.
    pub fn node_application_id(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<LocalGenericApplicationId> {
        for segment in self.segments.iter().rev() {
            if let Some(application_id) = segment.node_application_id(node_id) {
                return Some(application_id);
            }
        }

        None
    }

    /// Find one exact generic application by shape.
    pub fn find_application(
        &self,
        expected: &GenericApplication,
    ) -> Option<LocalGenericApplicationId> {
        for (application_id, application) in self.iter_applications() {
            if application == expected {
                return Some(application_id);
            }
        }

        None
    }

    /// Intern one generic application into a mutable tail segment.
    pub fn intern_application(
        &self,
        tail: &mut GenericSegment,
        application: GenericApplication,
    ) -> LocalGenericApplicationId {
        assert_eq!(
            self.module_id, tail.module_id,
            "generic table tail belongs to a different module"
        );

        if let Some(application_id) = self.find_application(&application) {
            return application_id;
        }

        if let Some(application_id) = tail.find_application(&application) {
            return application_id;
        }

        tail.push_application(application)
    }

    /// Get a generic template by id.
    pub fn get_template(&self, template_id: LocalGenericTemplateId) -> &GenericTemplate {
        for segment in self.segments.iter() {
            if let Some(template) = segment.get_local_template(template_id) {
                return template;
            }
        }

        panic!("DIR generic template {template_id:?} is not visible")
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

    /// Get a generic application by id.
    pub fn get_application(
        &self,
        application_id: LocalGenericApplicationId,
    ) -> &GenericApplication {
        for segment in self.segments.iter() {
            if let Some(application) = segment.get_local_application(application_id) {
                return application;
            }
        }

        panic!("DIR generic application {application_id:?} is not visible")
    }

    /// Get the number of templates in the table.
    pub fn template_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.template_count())
            .unwrap_or(0)
    }

    /// Get the number of slots in the table.
    pub fn slot_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.slot_count())
            .unwrap_or(0)
    }

    /// Get the number of generic applications in the table.
    pub fn application_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.application_count())
            .unwrap_or(0)
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Generic slots and applications added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericSegment {
    /// The module id of the generic segment.
    pub module_id: ModuleId,
    /// The first generic template id owned by this table segment.
    pub(crate) first_template_id: u32,
    /// The first generic slot id owned by this table segment.
    pub(crate) first_slot_id: u32,
    /// The first generic application id owned by this table segment.
    pub(crate) first_application_id: u32,
    /// Generic templates.
    pub(crate) templates: Arena<GenericTemplate>,
    /// Generic slots.
    pub(crate) slots: Arena<GenericSlot>,
    /// Interned generic applications.
    pub(crate) applications: Arena<GenericApplication>,
    /// Generic applications keyed by DIR node.
    pub(crate) nodes: IndexMap<GlobalNodeIdAny, LocalGenericApplicationId>,
}

impl GenericSegment {
    /// Create a new generic segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_template_id: 0,
            first_slot_id: 0,
            first_application_id: 0,
            templates: Arena::new(),
            slots: Arena::new(),
            applications: Arena::new(),
            nodes: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing generic table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_template_id: base.template_count(),
            first_slot_id: base.slot_count(),
            first_application_id: base.application_count(),
            templates: Arena::new(),
            slots: Arena::new(),
            applications: Arena::new(),
            nodes: IndexMap::new(),
        }
    }

    /// Append a generic template to this segment.
    pub fn push_template(&mut self, template: GenericTemplate) -> LocalGenericTemplateId {
        let template_id = LocalGenericTemplateId::new(self.template_count());
        self.templates.allocate(template);

        template_id
    }

    /// Append a generic slot to this segment.
    pub fn push_slot(&mut self, slot: GenericSlot) -> LocalGenericSlotId {
        let slot_id = LocalGenericSlotId::new(self.slot_count());
        self.slots.allocate(slot);

        slot_id
    }

    /// Append a generic application to this segment.
    pub fn push_application(
        &mut self,
        application: GenericApplication,
    ) -> LocalGenericApplicationId {
        let application_id = LocalGenericApplicationId::new(self.application_count());
        self.applications.allocate(application);

        application_id
    }

    /// Attach a generic application to a source node.
    pub fn set_node_application(
        &mut self,
        node_id: GlobalNodeIdAny,
        application_id: LocalGenericApplicationId,
    ) {
        self.nodes.insert(node_id, application_id);
    }

    /// Return the generic application attached to a source node.
    pub fn node_application_id(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<LocalGenericApplicationId> {
        self.nodes.get(&node_id).copied()
    }

    /// Iterate source nodes with their generic applications.
    pub fn node_applications(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalGenericApplicationId)> + '_ {
        self.nodes
            .iter()
            .map(|(node_id, application_id)| (*node_id, *application_id))
    }

    /// Return the number of source nodes with generic applications.
    pub fn node_application_count(&self) -> usize {
        self.nodes.len()
    }

    /// Find one exact generic application by shape.
    pub fn find_application(
        &self,
        expected: &GenericApplication,
    ) -> Option<LocalGenericApplicationId> {
        for (application_id, application) in self.iter_applications() {
            if application == expected {
                return Some(application_id);
            }
        }

        None
    }

    /// Get a generic template by id.
    pub fn get_template(&self, template_id: LocalGenericTemplateId) -> &GenericTemplate {
        self.get_local_template(template_id).unwrap_or_else(|| {
            panic!("DIR generic template {template_id:?} is not allocated in this segment")
        })
    }

    /// Get a generic slot by id.
    pub fn get_slot(&self, slot_id: LocalGenericSlotId) -> &GenericSlot {
        self.get_local_slot(slot_id).unwrap_or_else(|| {
            panic!("DIR generic slot {slot_id:?} is not allocated in this segment")
        })
    }

    /// Get a generic application by id.
    pub fn get_application(
        &self,
        application_id: LocalGenericApplicationId,
    ) -> &GenericApplication {
        self.get_local_application(application_id)
            .unwrap_or_else(|| {
                panic!(
                    "DIR generic application {application_id:?} is not allocated in this segment"
                )
            })
    }

    /// Iterate committed generic templates with their local ids.
    pub fn iter_templates(
        &self,
    ) -> impl Iterator<Item = (LocalGenericTemplateId, &GenericTemplate)> + '_ {
        (self.first_template_id..self.template_count()).map(|index| {
            let template_id = LocalGenericTemplateId::new(index);
            (template_id, self.get_template(template_id))
        })
    }

    /// Iterate committed generic slots with their local ids.
    pub fn iter_slots(&self) -> impl Iterator<Item = (LocalGenericSlotId, &GenericSlot)> + '_ {
        (self.first_slot_id..self.slot_count()).map(|index| {
            let slot_id = LocalGenericSlotId::new(index);
            (slot_id, self.get_slot(slot_id))
        })
    }

    /// Iterate committed generic applications with their local ids.
    pub fn iter_applications(
        &self,
    ) -> impl Iterator<Item = (LocalGenericApplicationId, &GenericApplication)> + '_ {
        (self.first_application_id..self.application_count()).map(|index| {
            let application_id = LocalGenericApplicationId::new(index);
            (application_id, self.get_application(application_id))
        })
    }

    /// Get the number of templates in the segment.
    pub fn template_count(&self) -> u32 {
        self.first_template_id + self.templates.len() as u32
    }

    /// Get the number of slots in the segment.
    pub fn slot_count(&self) -> u32 {
        self.first_slot_id + self.slots.len() as u32
    }

    /// Get the number of generic applications in the segment.
    pub fn application_count(&self) -> u32 {
        self.first_application_id + self.applications.len() as u32
    }

    /// Return whether this segment has no entries.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
            && self.slots.is_empty()
            && self.applications.is_empty()
            && self.nodes.is_empty()
    }

    /// Get a generic template owned by this table segment.
    pub(crate) fn get_local_template(
        &self,
        template_id: LocalGenericTemplateId,
    ) -> Option<&GenericTemplate> {
        self.contains_template_id(template_id)
            .then(|| self.templates.get(template_id.0 - self.first_template_id))
    }

    /// Get a generic slot owned by this table segment.
    pub(crate) fn get_local_slot(&self, slot_id: LocalGenericSlotId) -> Option<&GenericSlot> {
        self.contains_slot_id(slot_id)
            .then(|| self.slots.get(slot_id.0 - self.first_slot_id))
    }

    /// Get a generic application owned by this table segment.
    pub(crate) fn get_local_application(
        &self,
        application_id: LocalGenericApplicationId,
    ) -> Option<&GenericApplication> {
        self.contains_application_id(application_id).then(|| {
            self.applications
                .get(application_id.0 - self.first_application_id)
        })
    }

    /// Return whether this segment contains the given template id.
    fn contains_template_id(&self, template_id: LocalGenericTemplateId) -> bool {
        template_id.0 >= self.first_template_id && template_id.0 < self.template_count()
    }

    /// Return whether this segment contains the given slot id.
    fn contains_slot_id(&self, slot_id: LocalGenericSlotId) -> bool {
        slot_id.0 >= self.first_slot_id && slot_id.0 < self.slot_count()
    }

    /// Return whether this segment contains the given application id.
    fn contains_application_id(&self, application_id: LocalGenericApplicationId) -> bool {
        application_id.0 >= self.first_application_id && application_id.0 < self.application_count()
    }
}
