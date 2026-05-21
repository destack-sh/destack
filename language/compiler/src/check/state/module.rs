use std::sync::Arc;

use destack_artifact::{
    DiagnosticAnchor, DirBound, DirChecked, DirExpanded, DirExported, DirImported, DirParsed,
    DirResolved,
};
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use indexmap::IndexMap;

use crate::CheckError;

use super::{
    Condition, Constraint, FlowState, InferOrigin, Obligation, StaticInferId, StaticInferOrigin,
    StaticSlot, TypeInferId, TypeSlot,
};

/// Check state for one module.
#[allow(dead_code)]
#[derive(Debug)]
pub(in crate::check) struct CheckModuleState {
    /// The requested module.
    module: ModuleId,
    /// The requested profile.
    profile: ProfileId,

    /// The parsed DIR input.
    parsed: Arc<DirParsed>,
    /// The bound DIR input.
    bound: Arc<DirBound>,
    /// The imported DIR input.
    imported: Arc<DirImported>,
    /// The exported DIR input.
    exported: Arc<DirExported>,
    /// The resolved DIR input.
    resolved: Arc<DirResolved>,
    /// The expanded DIR input.
    expanded: Arc<DirExpanded>,

    /// Checked type segment.
    types: dir::TypeSegment,
    /// Checked static value segment.
    statics: dir::StaticSegment,
    /// Checked generic segment.
    generics: dir::GenericSegment,
    /// Checked resolution segment.
    resolutions: dir::ResolutionSegment,
    /// Checked instance segment.
    instances: dir::InstanceSegment,
    /// Checked relation segment.
    relations: dir::RelationSegment,
    /// Checked extension segment.
    extensions: dir::ExtensionSegment,
    /// Checked layout segment.
    layouts: dir::LayoutSegment,
    /// Checked capture segment.
    captures: dir::CaptureSegment,

    /// The visitor options used while walking this module.
    options: dir::NodeVisitorOptions,

    /// Current flow while walking this module.
    flow: FlowState,

    /// Type inference variables.
    infer_origins: Vec<InferOrigin>,
    /// Type inference slots.
    type_slots: Vec<TypeSlot>,
    /// Static inference variables.
    static_infer_origins: Vec<StaticInferOrigin>,
    /// Static inference slots.
    static_slots: Vec<StaticSlot>,
    /// Type inference variables keyed by node.
    node_infers: IndexMap<dir::GlobalNodeIdAny, TypeInferId>,
    /// Type inference variables keyed by symbol.
    symbol_infers: IndexMap<dir::GlobalSymbolId, TypeInferId>,
    /// Static inference variables keyed by node.
    node_static_infers: IndexMap<dir::GlobalNodeIdAny, StaticInferId>,
    /// Static inference variables keyed by symbol.
    symbol_static_infers: IndexMap<dir::GlobalSymbolId, StaticInferId>,
    /// Imported symbol types copied into this module.
    imported_symbol_types: IndexMap<dir::GlobalSymbolId, dir::LocalTypeId>,

    /// Constraints produced by walking DIR.
    constraints: Vec<Constraint>,
    /// Conditions extracted from source control flow.
    conditions: Vec<Condition>,
    /// Obligations produced by walking DIR.
    obligations: Vec<Obligation>,

    /// Recoverable diagnostics collected while checking.
    diagnostics: Vec<CheckError>,
}

#[allow(dead_code)]
impl CheckModuleState {
    /// Create check state for one requested module.
    pub(in crate::check) fn new(
        module: ModuleId,
        profile: ProfileId,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        imported: Arc<DirImported>,
        exported: Arc<DirExported>,
        resolved: Arc<DirResolved>,
        expanded: Arc<DirExpanded>,
    ) -> Self {
        let types = dir::TypeSegment::from_base(&expanded.types);
        let statics = dir::StaticSegment::from_base(&expanded.statics);
        let generics = dir::GenericSegment::new(module);
        let resolutions = dir::ResolutionSegment::new(module);
        let instances = dir::InstanceSegment::new(module);
        let relations = dir::RelationSegment::new(module);
        let extensions = dir::ExtensionSegment::new(module);
        let layouts = dir::LayoutSegment::new(module);
        let captures = dir::CaptureSegment::new(module);

        Self {
            module,
            profile,
            parsed,
            bound,
            imported,
            exported,
            resolved,
            expanded,
            types,
            statics,
            generics,
            resolutions,
            instances,
            relations,
            extensions,
            layouts,
            captures,
            options: dir::NodeVisitorOptions::default(),
            flow: FlowState::default(),
            infer_origins: Vec::new(),
            type_slots: Vec::new(),
            static_infer_origins: Vec::new(),
            static_slots: Vec::new(),
            node_infers: IndexMap::new(),
            symbol_infers: IndexMap::new(),
            node_static_infers: IndexMap::new(),
            symbol_static_infers: IndexMap::new(),
            imported_symbol_types: IndexMap::new(),
            constraints: Vec::new(),
            conditions: Vec::new(),
            obligations: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Return the requested module.
    pub(in crate::check) fn module(&self) -> ModuleId {
        self.module
    }

    /// Return the requested profile.
    pub(in crate::check) fn profile(&self) -> ProfileId {
        self.profile
    }

    /// Return the parsed DIR input.
    pub(in crate::check) fn parsed(&self) -> &DirParsed {
        &self.parsed
    }

    /// Return the parsed DIR input handle.
    pub(in crate::check) fn parsed_arc(&self) -> Arc<DirParsed> {
        Arc::clone(&self.parsed)
    }

    /// Return the bound DIR input.
    pub(in crate::check) fn bound(&self) -> &DirBound {
        &self.bound
    }

    /// Return the imported DIR input.
    pub(in crate::check) fn imported(&self) -> &DirImported {
        &self.imported
    }

    /// Return the exported DIR input.
    pub(in crate::check) fn exported(&self) -> &DirExported {
        &self.exported
    }

    /// Return the resolved DIR input.
    pub(in crate::check) fn resolved(&self) -> &DirResolved {
        &self.resolved
    }

    /// Return the expanded DIR input.
    pub(in crate::check) fn expanded(&self) -> &DirExpanded {
        &self.expanded
    }

    /// Return the expanded DIR input handle.
    pub(in crate::check) fn expanded_arc(&self) -> Arc<DirExpanded> {
        Arc::clone(&self.expanded)
    }

    /// Return the cumulative binding table visible to check.
    pub(in crate::check) fn binding_table(&self) -> dir::BindingTable<'static> {
        self.expanded.binding_table(&self.bound)
    }

    /// Return the DIR visitor options.
    pub(in crate::check) fn visitor_options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    /// Return the cumulative type table visible to check inputs.
    pub(in crate::check) fn input_type_table(&self) -> dir::TypeTable<'static> {
        self.expanded.type_table(&self.bound)
    }

    /// Return the cumulative static table visible to check inputs.
    pub(in crate::check) fn input_static_table(&self) -> dir::StaticTable<'static> {
        self.expanded.static_table(&self.bound)
    }

    /// Return the checked type tail.
    pub(in crate::check) fn types(&self) -> &dir::TypeSegment {
        &self.types
    }

    /// Return the checked type tail mutably.
    pub(in crate::check) fn types_mut(&mut self) -> &mut dir::TypeSegment {
        &mut self.types
    }

    /// Return the checked static value tail.
    pub(in crate::check) fn statics(&self) -> &dir::StaticSegment {
        &self.statics
    }

    /// Return the checked static value tail mutably.
    pub(in crate::check) fn statics_mut(&mut self) -> &mut dir::StaticSegment {
        &mut self.statics
    }

    /// Return the checked generic tail mutably.
    pub(in crate::check) fn generics_mut(&mut self) -> &mut dir::GenericSegment {
        &mut self.generics
    }

    /// Return the checked resolution tail mutably.
    pub(in crate::check) fn resolutions_mut(&mut self) -> &mut dir::ResolutionSegment {
        &mut self.resolutions
    }

    /// Return the checked resolution tail.
    pub(in crate::check) fn resolutions(&self) -> &dir::ResolutionSegment {
        &self.resolutions
    }

    /// Return the checked instance tail mutably.
    pub(in crate::check) fn instances_mut(&mut self) -> &mut dir::InstanceSegment {
        &mut self.instances
    }

    /// Return the checked relation tail mutably.
    pub(in crate::check) fn relations_mut(&mut self) -> &mut dir::RelationSegment {
        &mut self.relations
    }

    /// Return the checked extension tail mutably.
    pub(in crate::check) fn extensions_mut(&mut self) -> &mut dir::ExtensionSegment {
        &mut self.extensions
    }

    /// Return the checked extension tail.
    pub(in crate::check) fn extensions(&self) -> &dir::ExtensionSegment {
        &self.extensions
    }

    /// Return the checked layout tail mutably.
    pub(in crate::check) fn layouts_mut(&mut self) -> &mut dir::LayoutSegment {
        &mut self.layouts
    }

    /// Return the checked capture tail mutably.
    pub(in crate::check) fn captures_mut(&mut self) -> &mut dir::CaptureSegment {
        &mut self.captures
    }

    /// Resolve one local symbol through an import when needed.
    pub(in crate::check) fn resolve_imported_symbol(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> dir::GlobalSymbolId {
        if let Some(dir::ImportTarget::Symbol(target)) = self.resolved.imports.symbol_target(symbol)
        {
            return target;
        }

        symbol.into_global(self.module)
    }

    /// Return imported global symbols for one key.
    pub(in crate::check) fn global_symbols(
        &self,
        key: dir::StaticKey,
    ) -> Option<&[dir::GlobalSymbolId]> {
        self.resolved.imports.global_symbols(key)
    }

    /// Return the current flow.
    pub(in crate::check) fn flow(&self) -> &FlowState {
        &self.flow
    }

    /// Return the current flow mutably.
    pub(in crate::check) fn flow_mut(&mut self) -> &mut FlowState {
        &mut self.flow
    }

    /// Return the effective checked type id for one node.
    pub(in crate::check) fn node_type_id(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> Option<dir::LocalTypeId> {
        self.types
            .get_node_type_id(node_id)
            .or_else(|| self.input_type_table().get_node_type_id(node_id))
    }

    /// Return the effective checked type id for one symbol.
    pub(in crate::check) fn symbol_type_id(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::LocalTypeId> {
        if let Some(type_id) = self.types.get_symbol_type_id(symbol_id) {
            return Some(type_id);
        }

        if let Some(type_id) = self.imported_symbol_types.get(&symbol_id).copied() {
            return Some(type_id);
        }

        self.input_type_table().get_symbol_type_id(symbol_id)
    }

    /// Copy imported symbol types solved by one dependency module.
    pub(in crate::check) fn import_symbol_types_from(&mut self, source: &CheckModuleState) {
        let symbol_targets = self
            .resolved
            .imports
            .symbol_targets
            .iter()
            .map(|(local_symbol, target)| (*local_symbol, *target))
            .collect::<Vec<_>>();
        let mut copied_types = IndexMap::new();

        // copy each symbol selected from this source module
        for (local_symbol, target) in symbol_targets {
            let dir::ImportTarget::Symbol(target_symbol) = target else {
                continue;
            };
            if target_symbol.module_id != source.module() {
                continue;
            }

            let Some(source_type) = source.symbol_type_id(target_symbol) else {
                continue;
            };
            let copied_type = self.copy_imported_type(source, source_type, &mut copied_types);
            let local_symbol = local_symbol.into_global(self.module);

            self.types.set_symbol_type(local_symbol, copied_type);
            self.imported_symbol_types
                .insert(target_symbol, copied_type);
            if let Some(infer) = self.symbol_infers.get(&target_symbol).copied() {
                self.replace_infer_type(infer, copied_type);
            }
            if let Some(infer) = self.symbol_infers.get(&local_symbol).copied() {
                self.replace_infer_type(infer, copied_type);
            }
        }
    }

    /// Return the symbol selected for one resolved expression node.
    pub(in crate::check) fn resolution_symbol(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        self.resolutions.symbol_resolution(node_id)
    }

    /// Return one visible type by id.
    pub(in crate::check) fn get_type(&self, type_id: dir::LocalTypeId) -> dir::Type {
        if let Some(ty) = self.types.get_type_maybe(type_id) {
            return ty.clone();
        }

        self.input_type_table().get_type(type_id).clone()
    }

    /// Return a local type id for one primitive type.
    pub(in crate::check) fn primitive_type_id(
        &mut self,
        primitive: dir::PrimitiveType,
    ) -> dir::LocalTypeId {
        let ty = dir::Type::Primitive(primitive);
        let input_types = self.input_type_table();
        for type_id in input_types.iter_type_ids() {
            if input_types.get_type(type_id) == &ty {
                return type_id;
            }
        }

        for type_id in self.types.iter_type_ids() {
            if self.types.get_type(type_id) == &ty {
                return type_id;
            }
        }

        self.types.insert_type_from_any(ty, self.bound.module_node)
    }

    /// Return the source node that produced one visible type id.
    pub(in crate::check) fn type_source(&self, type_id: dir::LocalTypeId) -> dir::LocalNodeIdAny {
        if self.types.get_type_maybe(type_id).is_some() {
            return self.types.get_type_source(type_id);
        }

        self.input_type_table().get_type_source(type_id)
    }

    /// Add one type inference variable.
    pub(in crate::check) fn push_infer(&mut self, origin: InferOrigin) -> TypeInferId {
        let id = TypeInferId(self.infer_origins.len() as u32);
        self.infer_origins.push(origin);
        self.type_slots.push(TypeSlot::default());

        id
    }

    /// Return the origin for one type inference variable.
    pub(in crate::check) fn infer_origin(&self, infer: TypeInferId) -> &InferOrigin {
        &self.infer_origins[infer.0 as usize]
    }

    /// Return the solved type for one inference variable.
    pub(in crate::check) fn infer_type_id(&self, infer: TypeInferId) -> Option<dir::LocalTypeId> {
        self.type_slots[infer.0 as usize].ty
    }

    /// Return the number of type inference slots.
    pub(in crate::check) fn type_slot_count(&self) -> usize {
        self.type_slots.len()
    }

    /// Set the solved type for one inference variable.
    pub(in crate::check) fn set_infer_type(
        &mut self,
        infer: TypeInferId,
        type_id: dir::LocalTypeId,
    ) -> bool {
        let slot = &mut self.type_slots[infer.0 as usize];
        if slot.ty.is_some() {
            return false;
        }

        slot.ty = Some(type_id);

        true
    }

    /// Set the solved type for one inference variable.
    pub(in crate::check) fn replace_infer_type(
        &mut self,
        infer: TypeInferId,
        type_id: dir::LocalTypeId,
    ) {
        let slot = &mut self.type_slots[infer.0 as usize];

        slot.ty = Some(type_id);
    }

    /// Record a copied imported symbol type.
    pub(in crate::check) fn set_imported_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        type_id: dir::LocalTypeId,
    ) {
        self.imported_symbol_types.insert(symbol, type_id);
    }

    /// Return the best source node for a type produced by one inference variable.
    pub(in crate::check) fn infer_source_node(&self, infer: TypeInferId) -> dir::LocalNodeIdAny {
        match self.infer_origin(infer) {
            InferOrigin::Node(node) if node.module_id == self.module => node.local_id,
            InferOrigin::Symbol(symbol) if symbol.module_id == self.module => {
                let local_symbol = symbol.local_id;
                let binding_table = self.binding_table();
                let symbol = binding_table.get_symbol(local_symbol);

                symbol
                    .declaration
                    .filter(|declaration| declaration.module_id == self.module)
                    .map(|declaration| declaration.local_id)
                    .unwrap_or(self.bound.module_node)
            }
            _ => self.bound.module_node,
        }
    }

    /// Commit solved inference variables into checked type tables.
    pub(in crate::check) fn commit_solved_types(&mut self) {
        for index in 0..self.type_slots.len() {
            let Some(type_id) = self.type_slots[index].ty else {
                continue;
            };
            let infer = TypeInferId(index as u32);

            match self.infer_origin(infer) {
                InferOrigin::Node(node) => self.types.set_node_type(*node, type_id),
                InferOrigin::Symbol(symbol) if symbol.module_id == self.module => {
                    self.types.set_symbol_type(*symbol, type_id);
                }
                InferOrigin::Symbol(_) => {}
                InferOrigin::Synthetic => {}
            }
        }
    }

    /// Return the inference variable for one node.
    pub(in crate::check) fn infer_node(&mut self, node: dir::GlobalNodeIdAny) -> TypeInferId {
        if let Some(id) = self.node_infers.get(&node).copied() {
            return id;
        }

        let id = self.push_infer(InferOrigin::Node(node));
        self.node_infers.insert(node, id);

        id
    }

    /// Return the inference variable for one symbol.
    pub(in crate::check) fn infer_symbol(&mut self, symbol: dir::GlobalSymbolId) -> TypeInferId {
        if let Some(id) = self.symbol_infers.get(&symbol).copied() {
            return id;
        }

        let id = self.push_infer(InferOrigin::Symbol(symbol));
        if let Some(type_id) = self.symbol_type_id(symbol) {
            self.set_infer_type(id, type_id);
        }
        self.symbol_infers.insert(symbol, id);

        id
    }

    /// Return a fresh synthetic type inference variable.
    pub(in crate::check) fn infer_synthetic(&mut self) -> TypeInferId {
        self.push_infer(InferOrigin::Synthetic)
    }

    /// Add one static inference variable.
    pub(in crate::check) fn push_static_infer(
        &mut self,
        origin: StaticInferOrigin,
    ) -> StaticInferId {
        let id = StaticInferId(self.static_infer_origins.len() as u32);
        self.static_infer_origins.push(origin);
        self.static_slots.push(StaticSlot::default());

        id
    }

    /// Return the static inference variable for one node.
    pub(in crate::check) fn infer_static_node(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> StaticInferId {
        if let Some(id) = self.node_static_infers.get(&node).copied() {
            return id;
        }

        let id = self.push_static_infer(StaticInferOrigin::Node(node));
        self.node_static_infers.insert(node, id);

        id
    }

    /// Return the static inference variable for one symbol.
    pub(in crate::check) fn infer_static_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> StaticInferId {
        if let Some(id) = self.symbol_static_infers.get(&symbol).copied() {
            return id;
        }

        let id = self.push_static_infer(StaticInferOrigin::Symbol(symbol));
        self.symbol_static_infers.insert(symbol, id);

        id
    }

    /// Return a fresh synthetic static inference variable.
    pub(in crate::check) fn infer_static_synthetic(&mut self) -> StaticInferId {
        self.push_static_infer(StaticInferOrigin::Synthetic)
    }

    /// Add one constraint.
    pub(in crate::check) fn push_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Return collected constraints.
    pub(in crate::check) fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    /// Add one condition.
    pub(in crate::check) fn push_condition(&mut self, condition: Condition) {
        self.conditions.push(condition);
    }

    /// Return collected conditions.
    pub(in crate::check) fn conditions(&self) -> &[Condition] {
        &self.conditions
    }

    /// Add one obligation.
    pub(in crate::check) fn push_obligation(&mut self, obligation: Obligation) {
        self.obligations.push(obligation);
    }

    /// Return collected obligations.
    pub(in crate::check) fn obligations(&self) -> &[Obligation] {
        &self.obligations
    }

    /// Return the source anchor for one local node.
    pub(in crate::check) fn anchor_node(&self, node_id: dir::LocalNodeIdAny) -> DiagnosticAnchor {
        self.parsed
            .tree
            .get_span_by_id(node_id.id)
            .map(DiagnosticAnchor::from)
            .unwrap_or_else(|| DiagnosticAnchor::from(self.module))
    }

    /// Add one recoverable check diagnostic.
    pub(in crate::check) fn push_diagnostic(&mut self, diagnostic: CheckError) {
        self.diagnostics.push(diagnostic);
    }

    /// Drain recoverable check diagnostics.
    pub(in crate::check) fn take_diagnostics(&mut self) -> Vec<CheckError> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Copy one imported type into this module.
    fn copy_imported_type(
        &mut self,
        source: &CheckModuleState,
        type_id: dir::LocalTypeId,
        copied_types: &mut IndexMap<dir::LocalTypeId, dir::LocalTypeId>,
    ) -> dir::LocalTypeId {
        if let Some(type_id) = copied_types.get(&type_id).copied() {
            return type_id;
        }

        let ty = source.get_type(type_id);
        let ty = self.copy_imported_type_value(source, ty, copied_types);
        let copied_type = self
            .types
            .insert_imported_type_from_any(ty, self.bound.module_node);

        copied_types.insert(type_id, copied_type);

        copied_type
    }

    /// Copy local type references inside one imported type value.
    fn copy_imported_type_value(
        &mut self,
        source: &CheckModuleState,
        ty: dir::Type,
        copied_types: &mut IndexMap<dir::LocalTypeId, dir::LocalTypeId>,
    ) -> dir::Type {
        match ty {
            dir::Type::Form(mut form) => {
                form.value = self.copy_imported_type(source, form.value, copied_types);

                dir::Type::Form(form)
            }
            dir::Type::ErasedAny(mut erased) => {
                erased.constraint =
                    self.copy_imported_type(source, erased.constraint, copied_types);

                dir::Type::ErasedAny(erased)
            }
            dir::Type::Predicate(mut predicate) => {
                predicate.target = predicate
                    .target
                    .map(|target| self.copy_imported_type(source, target, copied_types));

                dir::Type::Predicate(predicate)
            }
            dir::Type::FixedArray(mut array) => {
                array.element = self.copy_imported_type(source, array.element, copied_types);

                dir::Type::FixedArray(array)
            }
            dir::Type::Slice(mut slice) => {
                slice.element = self.copy_imported_type(source, slice.element, copied_types);

                dir::Type::Slice(slice)
            }
            dir::Type::Tuple(mut tuple) => {
                for element in &mut tuple.elements {
                    element.ty = self.copy_imported_type(source, element.ty, copied_types);
                }

                dir::Type::Tuple(tuple)
            }
            dir::Type::Shape(mut shape) => {
                for field in &mut shape.fields {
                    field.ty = self.copy_imported_type(source, field.ty, copied_types);
                }
                for signature in &mut shape.call_signatures {
                    *signature = self.copy_imported_type(source, *signature, copied_types);
                }
                for signature in &mut shape.construct_signatures {
                    *signature = self.copy_imported_type(source, *signature, copied_types);
                }
                for signature in &mut shape.index_signatures {
                    signature.key_type =
                        self.copy_imported_type(source, signature.key_type, copied_types);
                    signature.value_type =
                        self.copy_imported_type(source, signature.value_type, copied_types);
                }

                dir::Type::Shape(shape)
            }
            dir::Type::Function(mut function) => {
                for parameter in &mut function.generic_parameters {
                    *parameter = self.copy_imported_type(source, *parameter, copied_types);
                }
                function.this_parameter = function
                    .this_parameter
                    .map(|parameter| self.copy_imported_type(source, parameter, copied_types));
                for parameter in &mut function.parameters {
                    *parameter = self.copy_imported_type(source, *parameter, copied_types);
                }
                function.return_type = function
                    .return_type
                    .map(|return_type| self.copy_imported_type(source, return_type, copied_types));

                dir::Type::Function(function)
            }
            dir::Type::Closure(mut closure) => {
                closure.function = self.copy_imported_type(source, closure.function, copied_types);
                closure.environment =
                    self.copy_imported_type(source, closure.environment, copied_types);

                dir::Type::Closure(closure)
            }
            dir::Type::Union(mut union) => {
                for element in &mut union.elements {
                    *element = self.copy_imported_type(source, *element, copied_types);
                }

                dir::Type::Union(union)
            }
            dir::Type::Intersection(mut intersection) => {
                for element in &mut intersection.elements {
                    *element = self.copy_imported_type(source, *element, copied_types);
                }

                dir::Type::Intersection(intersection)
            }
            _ => ty,
        }
    }

    /// Finish check state into checked DIR.
    pub(in crate::check) fn finish(self) -> DirChecked {
        DirChecked {
            types: Arc::new(self.types),
            statics: Arc::new(self.statics),
            generics: Arc::new(self.generics),
            resolutions: Arc::new(self.resolutions),
            instances: Arc::new(self.instances),
            relations: Arc::new(self.relations),
            extensions: Arc::new(self.extensions),
            layouts: Arc::new(self.layouts),
            captures: Arc::new(self.captures),
        }
    }
}
