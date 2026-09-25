use std::sync::Arc;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};
use tspp_source::ModuleId;

use tspp_core::FxIndexMap as IndexMap;

use crate::{
    Arena, GenericParameterBinding, GenericParameterKey, GenericTemplate, GlobalNodeIdAny,
    GlobalSymbolId, GlobalTypeId, Instance, InstanceKey, InstanceOrigin, Instantiation,
    LocalGenericParameterId, LocalGenericTemplateId, LocalInstanceId, LocalScopeId, LocalSymbolId,
    MemoryParameter, SegmentView, TypeFold, TypeListId, VarianceModifier, Witness,
    free_region_name,
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

    /// Return the names of the regions one template declares ahead of one parameter.
    pub fn region_names_before(
        &self,
        parameter: LocalGenericParameterId,
        symbol_name: impl FnMut(GlobalSymbolId) -> String,
    ) -> Vec<String> {
        let template = self.get_parameter(parameter).template;
        let mut names = Vec::new();
        self.push_region_names(template, Some(parameter), &mut names, symbol_name);

        names
    }

    /// Push the names of the regions one template declares ahead of one parameter.
    pub fn push_region_names(
        &self,
        template: LocalGenericTemplateId,
        until: Option<LocalGenericParameterId>,
        names: &mut Vec<String>,
        mut symbol_name: impl FnMut(GlobalSymbolId) -> String,
    ) {
        for candidate in &self.get_template(template).parameters {
            if Some(*candidate) == until {
                return;
            }
            let binding = self.get_parameter(*candidate);
            if binding.memory_parameter() != Some(MemoryParameter::Region) {
                continue;
            }
            let name = match binding.key {
                GenericParameterKey::Symbol(symbol) => symbol_name(symbol),
                GenericParameterKey::Anonymous => {
                    free_region_name(names.iter().map(String::as_str))
                }
            };
            names.push(name);
        }
    }

    /// Return the position of one parameter on its template.
    pub fn parameter_position(&self, parameter_id: LocalGenericParameterId) -> usize {
        let template = self.get_parameter(parameter_id).template;

        self.get_template(template)
            .parameters
            .iter()
            .position(|parameter| *parameter == parameter_id)
            .unwrap_or_else(|| panic!("parameter {parameter_id:?} is missing from its template"))
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

    /// Return the witness one closed type answers one interface application with.
    pub fn witness(&self, ty: GlobalTypeId, interface: GlobalTypeId) -> Option<&Witness> {
        self.segments
            .iter()
            .find_map(|segment| segment.witness(ty, interface))
    }

    /// Return the filled bounds recorded for one parameter.
    pub fn parameter_bounds(&self, parameter: LocalGenericParameterId) -> Option<TypeListId> {
        self.segments
            .iter()
            .find_map(|segment| segment.parameter_bounds(parameter))
    }

    /// Return the filled where-clause bounds one template assumes for a parameter.
    pub fn assumed_bounds(
        &self,
        template: LocalGenericTemplateId,
        parameter: LocalGenericParameterId,
    ) -> Option<TypeListId> {
        self.segments
            .iter()
            .find_map(|segment| segment.assumed_bounds(template, parameter))
    }

    /// Return the instance recorded behind one application type.
    pub fn application_instance(&self, ty: GlobalTypeId) -> Option<LocalInstanceId> {
        self.segments
            .iter()
            .find_map(|segment| segment.application_instance(ty))
    }

    /// Return the dependents recorded for one declaration's signature.
    pub fn symbol_dependents(&self, symbol: LocalSymbolId) -> Option<&[GlobalTypeId]> {
        self.segments
            .iter()
            .find_map(|segment| segment.symbol_dependents(symbol))
    }

    /// Return the allocated instance recorded behind one selection a checked decision wrote.
    pub fn selection_instance(&self, selection: &InstanceKey) -> Option<LocalInstanceId> {
        self.segments
            .iter()
            .find_map(|segment| segment.selection_instance(selection))
    }

    /// Iterate the witnesses recorded across every segment.
    pub fn iter_witnesses(
        &self,
    ) -> impl Iterator<Item = (GlobalTypeId, GlobalTypeId, &Witness)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_witnesses())
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
    /// The first generic instance id owned by this table segment.
    pub(crate) first_instance_id: u32,
    /// Generic instances closed by this segment.
    pub(crate) instances: Arena<Instance>,
    /// Instantiations the bodies perform, open while they mention parameters.
    pub(crate) instantiations: Vec<Instantiation>,
    /// The interned instance behind each closed application type.
    pub(crate) application_instances: IndexMap<GlobalTypeId, LocalInstanceId>,
    /// The dependents each declaration's signature writes, in signature order.
    pub(crate) symbol_dependents: IndexMap<LocalSymbolId, Vec<GlobalTypeId>>,
    /// The allocated instance behind each selection a checked decision wrote.
    pub(crate) selection_instances: IndexMap<InstanceKey, LocalInstanceId>,
    /// The witness each closed type answers each interface application with.
    pub(crate) witnesses: IndexMap<(GlobalTypeId, GlobalTypeId), Witness>,
    /// The bounds each parameter assumes with elided arguments filled.
    pub(crate) parameter_bounds: IndexMap<LocalGenericParameterId, TypeListId>,
    /// The filled where-clause bounds each template assumes for a parameter.
    pub(crate) assumed_bounds:
        IndexMap<(LocalGenericTemplateId, LocalGenericParameterId), TypeListId>,
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
            first_instance_id: 0,
            instances: Arena::new(),
            application_instances: IndexMap::default(),
            symbol_dependents: IndexMap::default(),
            selection_instances: IndexMap::default(),
            witnesses: IndexMap::default(),
            parameter_bounds: IndexMap::default(),
            assumed_bounds: IndexMap::default(),
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
            first_instance_id: base.instance_count(),
            instances: Arena::new(),
            application_instances: IndexMap::default(),
            symbol_dependents: IndexMap::default(),
            selection_instances: IndexMap::default(),
            witnesses: IndexMap::default(),
            parameter_bounds: IndexMap::default(),
            assumed_bounds: IndexMap::default(),
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
            && self.instances.is_empty()
            && self.witnesses.is_empty()
            && self.instantiations.is_empty()
            && self.application_instances.is_empty()
            && self.symbol_dependents.is_empty()
            && self.selection_instances.is_empty()
            && self.parameter_bounds.is_empty()
            && self.assumed_bounds.is_empty()
            && self.instance_symbols.is_empty()
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

    /// Record the dependents one declaration's signature writes.
    pub fn set_symbol_dependents(&mut self, symbol: LocalSymbolId, dependents: Vec<GlobalTypeId>) {
        self.symbol_dependents.insert(symbol, dependents);
    }

    /// Return the dependents recorded for one declaration's signature.
    pub fn symbol_dependents(&self, symbol: LocalSymbolId) -> Option<&[GlobalTypeId]> {
        self.symbol_dependents.get(&symbol).map(Vec::as_slice)
    }

    /// Record the interned instance behind one closed application type.
    pub fn bind_application_instance(&mut self, ty: GlobalTypeId, instance: LocalInstanceId) {
        self.application_instances.insert(ty, instance);
    }

    /// Record the allocated instance behind one selection a checked decision wrote.
    pub fn bind_selection_instance(&mut self, selection: InstanceKey, instance: LocalInstanceId) {
        self.selection_instances.insert(selection, instance);
    }

    /// Return the allocated instance behind one selection a checked decision wrote.
    pub fn selection_instance(&self, selection: &InstanceKey) -> Option<LocalInstanceId> {
        self.selection_instances.get(selection).copied()
    }

    /// Record the witness one closed type answers one interface application with.
    pub fn bind_witness(&mut self, ty: GlobalTypeId, interface: GlobalTypeId, witness: Witness) {
        self.witnesses.insert((ty, interface), witness);
    }

    /// Return the witness recorded for one closed type and interface application.
    pub fn witness(&self, ty: GlobalTypeId, interface: GlobalTypeId) -> Option<&Witness> {
        self.witnesses.get(&(ty, interface))
    }

    /// Record the filled bounds one parameter assumes.
    pub fn set_parameter_bounds(&mut self, parameter: LocalGenericParameterId, bounds: TypeListId) {
        self.parameter_bounds.insert(parameter, bounds);
    }

    /// Return the filled bounds recorded for one parameter.
    pub fn parameter_bounds(&self, parameter: LocalGenericParameterId) -> Option<TypeListId> {
        self.parameter_bounds.get(&parameter).copied()
    }

    /// Record the filled where-clause bounds one template assumes for a parameter.
    pub fn set_assumed_bounds(
        &mut self,
        template: LocalGenericTemplateId,
        parameter: LocalGenericParameterId,
        bounds: TypeListId,
    ) {
        self.assumed_bounds.insert((template, parameter), bounds);
    }

    /// Return the filled where-clause bounds one template assumes for a parameter.
    pub fn assumed_bounds(
        &self,
        template: LocalGenericTemplateId,
        parameter: LocalGenericParameterId,
    ) -> Option<TypeListId> {
        self.assumed_bounds.get(&(template, parameter)).copied()
    }

    /// Iterate the witnesses recorded by this segment.
    pub fn iter_witnesses(
        &self,
    ) -> impl Iterator<Item = (GlobalTypeId, GlobalTypeId, &Witness)> + '_ {
        self.witnesses
            .iter()
            .map(|((ty, interface), witness)| (*ty, *interface, witness))
    }

    /// Return the instance recorded behind one application type.
    pub fn application_instance(&self, ty: GlobalTypeId) -> Option<LocalInstanceId> {
        self.application_instances.get(&ty).copied()
    }

    /// Iterate the application types recorded by this segment.
    pub fn iter_application_instances(
        &self,
    ) -> impl Iterator<Item = (GlobalTypeId, LocalInstanceId)> + '_ {
        self.application_instances
            .iter()
            .map(|(ty, instance)| (*ty, *instance))
    }

    /// Record one instantiation a body performs.
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
        for witness in self.witnesses.values_mut() {
            witness.map_types(map)?;
        }
        for instantiation in &mut self.instantiations {
            instantiation.map_types(map)?;
        }
        for dependents in self.symbol_dependents.values_mut() {
            for dependent in dependents {
                *dependent = map(*dependent)?;
            }
        }

        Ok(())
    }
}
