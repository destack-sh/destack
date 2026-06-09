use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Arena, GenericInstance, GenericParameterBinding, GenericTemplate, GlobalNodeIdAny,
    LocalGenericInstanceId, LocalGenericParameterId, LocalGenericTemplateId, SegmentView,
};

/// Cumulative generic parameters and instances for one DIR module.
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

    /// Iterate committed generic parameters with their local ids.
    pub fn iter_parameters(
        &self,
    ) -> impl Iterator<Item = (LocalGenericParameterId, &GenericParameterBinding)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_parameters())
    }

    /// Iterate committed generic instances with their local ids.
    pub fn iter_instances(
        &self,
    ) -> impl Iterator<Item = (LocalGenericInstanceId, &GenericInstance)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_instances())
    }

    /// Return the generic instance attached to a source node for one template.
    pub fn node_instance_id_for_template(
        &self,
        node_id: GlobalNodeIdAny,
        template_id: LocalGenericTemplateId,
    ) -> Option<LocalGenericInstanceId> {
        for segment in self.segments.iter().rev() {
            if let Some(instance_id) = segment.node_instance_id_for_template(node_id, template_id) {
                return Some(instance_id);
            }
        }

        None
    }

    /// Iterate generic instances attached to a source node.
    pub fn node_instance_ids(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> impl Iterator<Item = LocalGenericInstanceId> + '_ {
        self.segments
            .iter()
            .flat_map(move |segment| segment.node_instance_ids(node_id))
    }

    /// Return the generic template declared by one source node.
    pub fn template_by_source(&self, source: GlobalNodeIdAny) -> Option<LocalGenericTemplateId> {
        for (template_id, template) in self.iter_templates() {
            if template.source == source {
                return Some(template_id);
            }
        }

        None
    }

    /// Find one exact generic instance by shape.
    pub fn find_instance(&self, expected: &GenericInstance) -> Option<LocalGenericInstanceId> {
        for (instance_id, instance) in self.iter_instances() {
            if instance == expected {
                return Some(instance_id);
            }
        }

        None
    }

    /// Intern one generic instance into a mutable tail segment.
    pub fn intern_instance(
        &self,
        tail: &mut GenericSegment,
        instance: GenericInstance,
    ) -> LocalGenericInstanceId {
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

    /// Get a generic template by id.
    pub fn get_template(&self, template_id: LocalGenericTemplateId) -> &GenericTemplate {
        for segment in self.segments.iter() {
            if let Some(template) = segment.get_local_template(template_id) {
                return template;
            }
        }

        panic!("DIR generic template {template_id:?} is not visible")
    }

    /// Get a generic parameter by id.
    pub fn get_parameter(&self, parameter_id: LocalGenericParameterId) -> &GenericParameterBinding {
        for segment in self.segments.iter() {
            if let Some(parameter) = segment.get_local_parameter(parameter_id) {
                return parameter;
            }
        }

        panic!("DIR generic parameter {parameter_id:?} is not visible")
    }

    /// Get a generic instance by id.
    pub fn get_instance(&self, instance_id: LocalGenericInstanceId) -> &GenericInstance {
        self.get_instance_maybe(instance_id)
            .unwrap_or_else(|| panic!("DIR generic instance {instance_id:?} is not visible"))
    }

    /// Get a generic instance by id when it is visible.
    pub fn get_instance_maybe(
        &self,
        instance_id: LocalGenericInstanceId,
    ) -> Option<&GenericInstance> {
        for segment in self.segments.iter() {
            if let Some(instance) = segment.get_local_instance(instance_id) {
                return Some(instance);
            }
        }

        None
    }

    /// Get the number of templates in the table.
    pub fn template_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.template_count())
            .unwrap_or(0)
    }

    /// Get the number of parameters in the table.
    pub fn parameter_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.parameter_count())
            .unwrap_or(0)
    }

    /// Get the number of generic instances in the table.
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

/// Generic parameters and instances added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericSegment {
    /// The module id of the generic segment.
    pub module_id: ModuleId,
    /// The first generic template id owned by this table segment.
    pub(crate) first_template_id: u32,
    /// The first generic parameter id owned by this table segment.
    pub(crate) first_parameter_id: u32,
    /// The first generic instance id owned by this table segment.
    pub(crate) first_instance_id: u32,
    /// Generic templates.
    pub(crate) templates: Arena<GenericTemplate>,
    /// Generic parameters.
    pub(crate) parameters: Arena<GenericParameterBinding>,
    /// Interned generic instances.
    pub(crate) instances: Arena<GenericInstance>,
    /// Generic instances keyed by source node and template.
    pub(crate) nodes:
        IndexMap<GlobalNodeIdAny, IndexMap<LocalGenericTemplateId, LocalGenericInstanceId>>,
}

impl GenericSegment {
    /// Create a new generic segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_template_id: 0,
            first_parameter_id: 0,
            first_instance_id: 0,
            templates: Arena::new(),
            parameters: Arena::new(),
            instances: Arena::new(),
            nodes: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing generic table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_template_id: base.template_count(),
            first_parameter_id: base.parameter_count(),
            first_instance_id: base.instance_count(),
            templates: Arena::new(),
            parameters: Arena::new(),
            instances: Arena::new(),
            nodes: IndexMap::new(),
        }
    }

    /// Append a generic template to this segment.
    pub fn push_template(&mut self, template: GenericTemplate) -> LocalGenericTemplateId {
        let template_id = LocalGenericTemplateId::new(self.template_count());
        self.templates.allocate(template);

        template_id
    }

    /// Append a generic parameter to this segment.
    pub fn push_parameter(
        &mut self,
        parameter: GenericParameterBinding,
    ) -> LocalGenericParameterId {
        let parameter_id = LocalGenericParameterId::new(self.parameter_count());
        self.parameters.allocate(parameter);

        parameter_id
    }

    /// Append a generic instance to this segment.
    pub fn push_instance(&mut self, instance: GenericInstance) -> LocalGenericInstanceId {
        let instance_id = LocalGenericInstanceId::new(self.instance_count());
        self.instances.allocate(instance);

        instance_id
    }

    /// Intern one generic instance into this segment.
    pub fn intern_instance(&mut self, instance: GenericInstance) -> LocalGenericInstanceId {
        if let Some(instance_id) = self.find_instance(&instance) {
            return instance_id;
        }

        self.push_instance(instance)
    }

    /// Attach a generic instance to a source node.
    pub fn bind_node_instance(
        &mut self,
        node_id: GlobalNodeIdAny,
        instance_id: LocalGenericInstanceId,
    ) -> bool {
        assert!(
            self.contains_instance_id(instance_id),
            "DIR generic instance {instance_id:?} is not allocated in this segment"
        );

        let template_id = self.get_instance(instance_id).template.local_id;
        let instances = self.nodes.entry(node_id).or_default();
        if let Some(previous) = instances.get(&template_id).copied() {
            assert_eq!(
                previous, instance_id,
                "DIR generic source {node_id:?} already has a different instance for template {template_id:?}"
            );

            return false;
        }

        instances.insert(template_id, instance_id);

        true
    }

    /// Return the generic instance attached to a source node for one template.
    pub fn node_instance_id_for_template(
        &self,
        node_id: GlobalNodeIdAny,
        template_id: LocalGenericTemplateId,
    ) -> Option<LocalGenericInstanceId> {
        self.nodes
            .get(&node_id)
            .and_then(|instances| instances.get(&template_id).copied())
    }

    /// Iterate generic instances attached to a source node.
    pub fn node_instance_ids(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> impl Iterator<Item = LocalGenericInstanceId> + '_ {
        self.nodes
            .get(&node_id)
            .into_iter()
            .flat_map(|instances| instances.values().copied())
    }

    /// Iterate source nodes with their generic instances.
    pub fn node_instances(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalGenericInstanceId)> + '_ {
        self.nodes.iter().flat_map(|(node_id, instances)| {
            instances
                .values()
                .map(|instance_id| (*node_id, *instance_id))
        })
    }

    /// Return the number of source nodes with generic instances.
    pub fn node_instance_count(&self) -> usize {
        self.nodes.values().map(IndexMap::len).sum()
    }

    /// Find one exact generic instance by shape.
    pub fn find_instance(&self, expected: &GenericInstance) -> Option<LocalGenericInstanceId> {
        for (instance_id, instance) in self.iter_instances() {
            if instance == expected {
                return Some(instance_id);
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

    /// Get a generic parameter by id.
    pub fn get_parameter(&self, parameter_id: LocalGenericParameterId) -> &GenericParameterBinding {
        self.get_local_parameter(parameter_id).unwrap_or_else(|| {
            panic!("DIR generic parameter {parameter_id:?} is not allocated in this segment")
        })
    }

    /// Get a generic instance by id.
    pub fn get_instance(&self, instance_id: LocalGenericInstanceId) -> &GenericInstance {
        self.get_instance_maybe(instance_id).unwrap_or_else(|| {
            panic!("DIR generic instance {instance_id:?} is not allocated in this segment")
        })
    }

    /// Get a generic instance by id when it is allocated in this segment.
    pub fn get_instance_maybe(
        &self,
        instance_id: LocalGenericInstanceId,
    ) -> Option<&GenericInstance> {
        self.get_local_instance(instance_id)
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

    /// Iterate committed generic parameters with their local ids.
    pub fn iter_parameters(
        &self,
    ) -> impl Iterator<Item = (LocalGenericParameterId, &GenericParameterBinding)> + '_ {
        (self.first_parameter_id..self.parameter_count()).map(|index| {
            let parameter_id = LocalGenericParameterId::new(index);
            (parameter_id, self.get_parameter(parameter_id))
        })
    }

    /// Iterate committed generic instances with their local ids.
    pub fn iter_instances(
        &self,
    ) -> impl Iterator<Item = (LocalGenericInstanceId, &GenericInstance)> + '_ {
        (self.first_instance_id..self.instance_count()).map(|index| {
            let instance_id = LocalGenericInstanceId::new(index);
            (instance_id, self.get_instance(instance_id))
        })
    }

    /// Get the number of templates in the segment.
    pub fn template_count(&self) -> u32 {
        self.first_template_id + self.templates.len() as u32
    }

    /// Get the number of parameters in the segment.
    pub fn parameter_count(&self) -> u32 {
        self.first_parameter_id + self.parameters.len() as u32
    }

    /// Get the number of generic instances in the segment.
    pub fn instance_count(&self) -> u32 {
        self.first_instance_id + self.instances.len() as u32
    }

    /// Return whether this segment has no entries.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
            && self.parameters.is_empty()
            && self.instances.is_empty()
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

    /// Get a generic parameter owned by this table segment.
    pub(crate) fn get_local_parameter(
        &self,
        parameter_id: LocalGenericParameterId,
    ) -> Option<&GenericParameterBinding> {
        self.contains_parameter_id(parameter_id).then(|| {
            self.parameters
                .get(parameter_id.0 - self.first_parameter_id)
        })
    }

    /// Get a generic instance owned by this table segment.
    pub(crate) fn get_local_instance(
        &self,
        instance_id: LocalGenericInstanceId,
    ) -> Option<&GenericInstance> {
        self.contains_instance_id(instance_id)
            .then(|| self.instances.get(instance_id.0 - self.first_instance_id))
    }

    /// Return whether this segment contains the given template id.
    fn contains_template_id(&self, template_id: LocalGenericTemplateId) -> bool {
        template_id.0 >= self.first_template_id && template_id.0 < self.template_count()
    }

    /// Return whether this segment contains the given parameter id.
    fn contains_parameter_id(&self, parameter_id: LocalGenericParameterId) -> bool {
        parameter_id.0 >= self.first_parameter_id && parameter_id.0 < self.parameter_count()
    }

    /// Return whether this segment contains the given instance id.
    fn contains_instance_id(&self, instance_id: LocalGenericInstanceId) -> bool {
        instance_id.0 >= self.first_instance_id && instance_id.0 < self.instance_count()
    }
}
