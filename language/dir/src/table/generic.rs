use destack_serde::Reflect;
use std::sync::Arc;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use destack_core::FxIndexMap as IndexMap;

use crate::{
    Arena, AutoInterfaceSet, Cardinality, GenericParameterBinding, GenericParameterKey,
    GenericTemplate, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, Instance, InstanceKey,
    InstanceOrigin, Instantiation, LocalGenericParameterId, LocalGenericTemplateId,
    LocalInstanceId, LocalScopeId, SegmentView, TypeFold, VarianceModifier,
};

/// Cumulative generic templates and parameters for one DIR module.
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

    /// Return the derived variance recorded for one parameter.
    pub fn variance(&self, parameter_id: LocalGenericParameterId) -> Option<VarianceModifier> {
        for segment in self.segments.iter().rev() {
            if let Some(variance) = segment.variance(parameter_id) {
                return Some(variance);
            }
        }

        None
    }

    /// Return the cardinality recorded for one parameter.
    pub fn cardinality(&self, parameter_id: LocalGenericParameterId) -> Option<Cardinality> {
        for segment in self.segments.iter().rev() {
            if let Some(cardinality) = segment.cardinality(parameter_id) {
                return Some(cardinality);
            }
        }

        None
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

    /// Iterate committed instantiations across every segment.
    pub fn iter_instantiations(&self) -> impl Iterator<Item = &Instantiation> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_instantiations())
    }

    /// Return the generic parameter declared by one symbol.
    pub fn parameter_by_symbol(&self, symbol: GlobalSymbolId) -> Option<LocalGenericParameterId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.parameter_by_symbol(symbol))
    }

    /// Return the generic template declared by one source node.
    pub fn template_by_source(&self, source: GlobalNodeIdAny) -> Option<LocalGenericTemplateId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.template_by_source(source))
    }

    /// Return the generic template that governs one lexical scope.
    pub fn template_by_scope(&self, scope: LocalScopeId) -> Option<LocalGenericTemplateId> {
        for segment in self.segments.iter().rev() {
            if let Some(template_id) = segment.template_by_scope(scope) {
                return Some(template_id);
            }
        }

        None
    }

    /// Return the generic template declared by one symbol.
    pub fn template_by_symbol(&self, symbol: GlobalSymbolId) -> Option<LocalGenericTemplateId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.template_by_symbol(symbol))
    }

    /// Return one parameter's declared or checker-derived variance.
    pub fn parameter_variance(
        &self,
        parameter: LocalGenericParameterId,
    ) -> Option<VarianceModifier> {
        let binding = self.get_parameter(parameter);

        binding.variance.or_else(|| self.variance(parameter))
    }

    /// Return the generic template with a stable id, or None outside every segment.
    pub fn get_template_maybe(
        &self,
        template_id: LocalGenericTemplateId,
    ) -> Option<&GenericTemplate> {
        self.segments
            .iter()
            .find_map(|segment| segment.get_local_template(template_id))
    }

    /// Return the generic parameter with a stable id, or None outside every segment.
    pub fn get_parameter_maybe(
        &self,
        parameter_id: LocalGenericParameterId,
    ) -> Option<&GenericParameterBinding> {
        self.segments
            .iter()
            .find_map(|segment| segment.get_local_parameter(parameter_id))
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

    /// Get the number of templates in the table.
    pub fn template_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.template_count())
            .expect("generic table needs at least one segment")
    }

    /// Get the number of parameters in the table.
    pub fn parameter_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.parameter_count())
            .expect("generic table needs at least one segment")
    }

    /// Iterate committed generic instances with their local ids.
    pub fn iter_instances(&self) -> impl Iterator<Item = (LocalInstanceId, &Instance)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_instances())
    }

    /// Get a generic instance by id.
    pub fn get_instance(&self, instance_id: LocalInstanceId) -> &Instance {
        for segment in self.segments.iter() {
            if let Some(instance) = segment.get_local_instance(instance_id) {
                return instance;
            }
        }

        panic!("DIR generic instance {instance_id:?} is not visible")
    }

    /// Iterate the instantiations one template's body performs.
    pub fn instantiations_of(
        &self,
        owner: Option<GlobalSymbolId>,
    ) -> impl Iterator<Item = &Instantiation> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_instantiations())
            .filter(move |instantiation| instantiation.owner == owner)
    }

    /// Return the materialized type of one template type under one instance.
    pub fn instance_type(
        &self,
        instance: LocalInstanceId,
        source: GlobalTypeId,
    ) -> Option<GlobalTypeId> {
        for segment in self.segments.iter() {
            if let Some(resolved) = segment.instance_type(instance, source) {
                return Some(resolved);
            }
        }

        None
    }

    /// Return the materialized symbol type one instance resolves.
    pub fn instance_symbol(
        &self,
        instance: LocalInstanceId,
        symbol: GlobalSymbolId,
    ) -> Option<GlobalTypeId> {
        self.segments
            .iter()
            .find_map(|segment| segment.instance_symbol(instance, symbol))
    }

    /// Iterate the application types every segment records.
    pub fn iter_application_instances(
        &self,
    ) -> impl Iterator<Item = (GlobalTypeId, LocalInstanceId)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_application_instances())
    }

    /// Iterate the application region rows across every segment.
    pub fn iter_application_regions(
        &self,
    ) -> impl Iterator<Item = (GlobalTypeId, &[GlobalTypeId])> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_application_regions())
    }

    /// Iterate the requirement selections recorded across every segment.
    pub fn iter_dispatch_selections(
        &self,
    ) -> impl Iterator<Item = (&InstanceKey, &InstanceKey)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_dispatch_selections())
    }

    /// Get the number of instances in the table.
    pub fn instance_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.instance_count())
            .expect("generic table needs at least one segment")
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Generic templates and parameters added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct GenericSegment {
    /// The module id of the generic segment.
    pub module_id: ModuleId,
    /// The first generic template id owned by this table segment.
    pub(crate) first_template_id: u32,
    /// The first generic parameter id owned by this table segment.
    pub(crate) first_parameter_id: u32,
    /// Generic templates.
    pub(crate) templates: Arena<GenericTemplate>,
    /// Generic template ids keyed by lexical scope id.
    pub(crate) templates_by_scope: Vec<Option<LocalGenericTemplateId>>,
    /// Generic parameters.
    pub(crate) parameters: Arena<GenericParameterBinding>,
    /// Variances derived from declared member types.
    pub(crate) variances: Vec<(LocalGenericParameterId, VarianceModifier)>,
    /// Cardinalities derived from declared value positions.
    pub(crate) cardinalities: Vec<(LocalGenericParameterId, Cardinality)>,
    /// The first generic instance id owned by this table segment.
    pub(crate) first_instance_id: u32,
    /// Generic instances closed by this segment.
    pub(crate) instances: Arena<Instance>,
    /// Instantiations the checked bodies perform, open while they mention parameters.
    pub(crate) instantiations: Vec<Instantiation>,
    /// Materialized types keyed by instance and template type.
    pub(crate) instance_types: IndexMap<(LocalInstanceId, GlobalTypeId), (GlobalTypeId, bool)>,
    /// The interned instance behind each closed application type.
    pub(crate) application_instances: IndexMap<GlobalTypeId, LocalInstanceId>,
    /// The region terms each closed application substitutes for its instance's bound regions.
    pub(crate) application_regions: IndexMap<GlobalTypeId, Vec<GlobalTypeId>>,
    /// The implementing selection behind each closed interface requirement.
    pub(crate) dispatch_selections: IndexMap<InstanceKey, InstanceKey>,
    /// Materialized symbol types keyed by instance and symbol.
    pub(crate) instance_symbols: IndexMap<(LocalInstanceId, GlobalSymbolId), GlobalTypeId>,
}

impl GenericSegment {
    /// Create a new generic segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_template_id: 0,
            first_parameter_id: 0,
            templates: Arena::new(),
            templates_by_scope: Vec::new(),
            parameters: Arena::new(),
            variances: Vec::new(),
            cardinalities: Vec::new(),
            first_instance_id: 0,
            instances: Arena::new(),
            instance_types: IndexMap::default(),
            application_instances: IndexMap::default(),
            application_regions: IndexMap::default(),
            dispatch_selections: IndexMap::default(),
            instance_symbols: IndexMap::default(),
            instantiations: Vec::new(),
        }
    }

    /// Create a new empty segment after an existing generic table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_template_id: base.template_count(),
            first_parameter_id: base.parameter_count(),
            templates: Arena::new(),
            templates_by_scope: Vec::new(),
            parameters: Arena::new(),
            variances: Vec::new(),
            cardinalities: Vec::new(),
            first_instance_id: base.instance_count(),
            instances: Arena::new(),
            instance_types: IndexMap::default(),
            application_instances: IndexMap::default(),
            application_regions: IndexMap::default(),
            dispatch_selections: IndexMap::default(),
            instance_symbols: IndexMap::default(),
            instantiations: Vec::new(),
        }
    }

    /// Record one derived variance for an unannotated parameter.
    pub fn set_variance(
        &mut self,
        parameter_id: LocalGenericParameterId,
        variance: VarianceModifier,
    ) {
        self.variances.push((parameter_id, variance));
    }

    /// Return the derived variance recorded for one parameter.
    pub fn variance(&self, parameter_id: LocalGenericParameterId) -> Option<VarianceModifier> {
        self.variances
            .iter()
            .find(|(recorded, _)| *recorded == parameter_id)
            .map(|(_, variance)| *variance)
    }

    /// Record the cardinality one parameter's value positions derive.
    pub fn set_cardinality(
        &mut self,
        parameter_id: LocalGenericParameterId,
        cardinality: Cardinality,
    ) {
        match self
            .cardinalities
            .iter_mut()
            .find(|(recorded, _)| *recorded == parameter_id)
        {
            // upgrade a recorded Of to One
            Some((_, recorded)) => {
                if matches!(recorded, Cardinality::Of { .. })
                    && matches!(cardinality, Cardinality::One { .. })
                {
                    *recorded = cardinality;
                }
            }
            None => self.cardinalities.push((parameter_id, cardinality)),
        }
    }

    /// Return the cardinality recorded for one parameter.
    pub fn cardinality(&self, parameter_id: LocalGenericParameterId) -> Option<Cardinality> {
        self.cardinalities
            .iter()
            .find(|(recorded, _)| *recorded == parameter_id)
            .map(|(_, cardinality)| *cardinality)
    }

    /// Append a generic template to this segment.
    pub fn push_template(&mut self, template: GenericTemplate) -> LocalGenericTemplateId {
        // grow the scope index far enough to hold this template's scope
        let template_id = LocalGenericTemplateId::new(self.template_count());
        let scope_index = template.scope.0 as usize;
        if self.templates_by_scope.len() <= scope_index {
            self.templates_by_scope.resize(scope_index + 1, None);
        }

        // require one template per scope
        assert!(
            self.templates_by_scope[scope_index].is_none(),
            "DIR generic scope {:?} already has a template",
            template.scope
        );

        self.templates_by_scope[scope_index] = Some(template_id);
        self.templates.allocate(template);

        template_id
    }

    /// Replace the assumed conformances of one parameter in this segment.
    pub fn set_parameter_conformances(
        &mut self,
        parameter_id: LocalGenericParameterId,
        conformances: AutoInterfaceSet,
    ) {
        let slot = parameter_id.0 - self.first_parameter_id;
        self.parameters.get_mut(slot).conformances = conformances;
    }

    /// Replace the sealed conformances of one instance in this segment.
    pub fn set_instance_conformances(
        &mut self,
        instance_id: LocalInstanceId,
        conformances: AutoInterfaceSet,
    ) {
        let slot = instance_id.0 - self.first_instance_id;
        self.instances.get_mut(slot).conformances = conformances;
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

    /// Append a generic parameter and register it on its declaring template.
    pub fn push_template_parameter(
        &mut self,
        parameter: GenericParameterBinding,
    ) -> LocalGenericParameterId {
        let template_id = parameter.template;
        let parameter_id = self.push_parameter(parameter);

        // register the parameter in declaration order
        let slot = template_id.0 - self.first_template_id;
        self.templates.get_mut(slot).parameters.push(parameter_id);

        parameter_id
    }

    /// Get a generic template by id.
    pub fn get_template(&self, template_id: LocalGenericTemplateId) -> &GenericTemplate {
        self.get_local_template(template_id).unwrap_or_else(|| {
            panic!("DIR generic template {template_id:?} is not allocated in this segment")
        })
    }

    /// Return the generic template that governs one lexical scope.
    pub fn template_by_scope(&self, scope: LocalScopeId) -> Option<LocalGenericTemplateId> {
        self.templates_by_scope
            .get(scope.0 as usize)
            .copied()
            .flatten()
    }

    /// Return the generic template this segment declares at one source node.
    pub fn template_by_source(&self, source: GlobalNodeIdAny) -> Option<LocalGenericTemplateId> {
        self.iter_templates()
            .find_map(|(template_id, template)| (template.source == source).then_some(template_id))
    }

    /// Return the generic template this segment declares for one symbol.
    pub fn template_by_symbol(&self, symbol: GlobalSymbolId) -> Option<LocalGenericTemplateId> {
        self.iter_templates().find_map(|(template_id, template)| {
            (template.symbol == Some(symbol)).then_some(template_id)
        })
    }

    /// Return the generic parameter this segment declares for one symbol.
    pub fn parameter_by_symbol(&self, symbol: GlobalSymbolId) -> Option<LocalGenericParameterId> {
        self.iter_parameters()
            .find_map(|(parameter_id, parameter)| {
                (parameter.key == GenericParameterKey::Symbol(symbol)).then_some(parameter_id)
            })
    }

    /// Get a generic parameter by id.
    pub fn get_parameter(&self, parameter_id: LocalGenericParameterId) -> &GenericParameterBinding {
        self.get_local_parameter(parameter_id).unwrap_or_else(|| {
            panic!("DIR generic parameter {parameter_id:?} is not allocated in this segment")
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

    /// Iterate committed generic parameters with their local ids.
    pub fn iter_parameters(
        &self,
    ) -> impl Iterator<Item = (LocalGenericParameterId, &GenericParameterBinding)> + '_ {
        (self.first_parameter_id..self.parameter_count()).map(|index| {
            let parameter_id = LocalGenericParameterId::new(index);
            (parameter_id, self.get_parameter(parameter_id))
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

    /// Return whether this segment has no entries.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
            && self.parameters.is_empty()
            && self.variances.is_empty()
            && self.cardinalities.is_empty()
            && self.instances.is_empty()
            && self.instance_types.is_empty()
            && self.instantiations.is_empty()
    }

    /// Allocate one generic instance and return its local id.
    pub fn push_instance(&mut self, instance: Instance) -> LocalInstanceId {
        let instance_id = LocalInstanceId::new(self.instance_count());
        self.instances.allocate(instance);

        instance_id
    }

    /// Set the source kind that introduced one instance.
    pub fn set_instance_origin(&mut self, instance_id: LocalInstanceId, origin: InstanceOrigin) {
        if self.contains_instance_id(instance_id) {
            let instance = self
                .instances
                .get_mut(instance_id.0 - self.first_instance_id);
            instance.origin = origin;
        }
    }

    /// Get the number of instances in the segment.
    pub fn instance_count(&self) -> u32 {
        self.first_instance_id + self.instances.len() as u32
    }

    /// Get a generic instance owned by this table segment.
    pub fn get_local_instance(&self, instance_id: LocalInstanceId) -> Option<&Instance> {
        self.contains_instance_id(instance_id)
            .then(|| self.instances.get(instance_id.0 - self.first_instance_id))
    }

    /// Iterate the instances owned by this table segment with their local ids.
    pub fn iter_instances(&self) -> impl Iterator<Item = (LocalInstanceId, &Instance)> + '_ {
        self.instances.iter().enumerate().map(|(index, instance)| {
            (
                LocalInstanceId::new(self.first_instance_id + index as u32),
                instance,
            )
        })
    }

    /// Record the materialized type of one template type under one instance.
    pub fn bind_instance_type(
        &mut self,
        instance: LocalInstanceId,
        source: GlobalTypeId,
        resolved: GlobalTypeId,
        is_evaluated: bool,
    ) {
        self.instance_types
            .insert((instance, source), (resolved, is_evaluated));
    }

    /// Return the materialized type of one template type under one instance.
    pub fn instance_type(
        &self,
        instance: LocalInstanceId,
        source: GlobalTypeId,
    ) -> Option<GlobalTypeId> {
        self.instance_types
            .get(&(instance, source))
            .map(|(resolved, _)| *resolved)
    }

    /// Iterate the materialized types recorded by this segment.
    pub fn iter_instance_types(
        &self,
    ) -> impl Iterator<Item = (LocalInstanceId, GlobalTypeId, GlobalTypeId, bool)> + '_ {
        self.instance_types
            .iter()
            .map(|((instance, source), (resolved, is_evaluated))| {
                (*instance, *source, *resolved, *is_evaluated)
            })
    }

    /// Record the materialized symbol type one instance resolves.
    pub fn bind_instance_symbol(
        &mut self,
        instance: LocalInstanceId,
        symbol: GlobalSymbolId,
        resolved: GlobalTypeId,
    ) {
        self.instance_symbols.insert((instance, symbol), resolved);
    }

    /// Return the materialized symbol type one instance resolves.
    pub fn instance_symbol(
        &self,
        instance: LocalInstanceId,
        symbol: GlobalSymbolId,
    ) -> Option<GlobalTypeId> {
        self.instance_symbols.get(&(instance, symbol)).copied()
    }

    /// Record the interned instance behind one closed application type.
    pub fn bind_application_instance(&mut self, ty: GlobalTypeId, instance: LocalInstanceId) {
        self.application_instances.insert(ty, instance);
    }

    /// Record the region terms one closed application substitutes, in bound order.
    pub fn bind_application_regions(&mut self, ty: GlobalTypeId, regions: Vec<GlobalTypeId>) {
        self.application_regions.insert(ty, regions);
    }

    /// Iterate the application region rows recorded by this segment.
    pub fn iter_application_regions(
        &self,
    ) -> impl Iterator<Item = (GlobalTypeId, &[GlobalTypeId])> + '_ {
        self.application_regions
            .iter()
            .map(|(ty, regions)| (*ty, regions.as_slice()))
    }

    /// Record the implementing selection behind one interface requirement.
    pub fn bind_dispatch_selection(&mut self, requirement: InstanceKey, implementer: InstanceKey) {
        self.dispatch_selections.insert(requirement, implementer);
    }

    /// Iterate the requirement selections recorded by this segment.
    pub fn iter_dispatch_selections(
        &self,
    ) -> impl Iterator<Item = (&InstanceKey, &InstanceKey)> + '_ {
        self.dispatch_selections.iter()
    }

    /// Iterate the application types recorded by this segment.
    pub fn iter_application_instances(
        &self,
    ) -> impl Iterator<Item = (GlobalTypeId, LocalInstanceId)> + '_ {
        self.application_instances
            .iter()
            .map(|(ty, instance)| (*ty, *instance))
    }

    /// Record one instantiation a checked body performs.
    pub fn push_instantiation(&mut self, instantiation: Instantiation) {
        self.instantiations.push(instantiation);
    }

    /// Iterate the instantiations recorded by this segment.
    pub fn iter_instantiations(&self) -> impl Iterator<Item = &Instantiation> + '_ {
        self.instantiations.iter()
    }

    /// Return whether this segment contains the given instance id.
    fn contains_instance_id(&self, instance_id: LocalInstanceId) -> bool {
        instance_id.0 >= self.first_instance_id && instance_id.0 < self.instance_count()
    }

    /// Get a generic template owned by this table segment.
    pub fn get_local_template(
        &self,
        template_id: LocalGenericTemplateId,
    ) -> Option<&GenericTemplate> {
        self.contains_template_id(template_id)
            .then(|| self.templates.get(template_id.0 - self.first_template_id))
    }

    /// Get a generic parameter owned by this table segment.
    pub fn get_local_parameter(
        &self,
        parameter_id: LocalGenericParameterId,
    ) -> Option<&GenericParameterBinding> {
        self.contains_parameter_id(parameter_id).then(|| {
            self.parameters
                .get(parameter_id.0 - self.first_parameter_id)
        })
    }

    /// Return one local template mutably.
    pub fn get_local_template_mut(
        &mut self,
        template_id: LocalGenericTemplateId,
    ) -> Option<&mut GenericTemplate> {
        self.contains_template_id(template_id).then(|| {
            self.templates
                .get_mut(template_id.0 - self.first_template_id)
        })
    }

    /// Return one local parameter binding mutably.
    pub fn get_local_parameter_mut(
        &mut self,
        parameter_id: LocalGenericParameterId,
    ) -> Option<&mut GenericParameterBinding> {
        self.contains_parameter_id(parameter_id).then(|| {
            self.parameters
                .get_mut(parameter_id.0 - self.first_parameter_id)
        })
    }

    /// Return whether this segment contains the given template id.
    fn contains_template_id(&self, template_id: LocalGenericTemplateId) -> bool {
        template_id.0 >= self.first_template_id && template_id.0 < self.template_count()
    }

    /// Return whether this segment contains the given parameter id.
    fn contains_parameter_id(&self, parameter_id: LocalGenericParameterId) -> bool {
        parameter_id.0 >= self.first_parameter_id && parameter_id.0 < self.parameter_count()
    }
}

impl TypeFold for GenericSegment {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        for template in self.templates.iter_mut() {
            template.map_types(map)?;
        }
        for binding in self.parameters.iter_mut() {
            binding.map_types(map)?;
        }
        for instance in self.instances.iter_mut() {
            instance.map_types(map)?;
        }
        // instance type keys reference sealed template types and stay as written
        for resolved in self.instance_types.values_mut() {
            resolved.0 = map(resolved.0)?;
        }
        for instantiation in &mut self.instantiations {
            instantiation.map_types(map)?;
        }
        for regions in self.application_regions.values_mut() {
            for region in regions {
                *region = map(*region)?;
            }
        }

        Ok(())
    }
}
