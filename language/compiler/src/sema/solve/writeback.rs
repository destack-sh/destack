use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_dir::{TypeFold, WalkSelections};

use crate::sema::{CheckModuleState, CheckState};
use crate::{CompilerError, CompilerResult};

/// One proved symbol use, recorded against the occurrence node that proved it.
struct SymbolUse {
    /// The node the use occurred at, when the use has one.
    node: Option<dir::GlobalNodeIdAny>,
    /// The used symbol.
    symbol: dir::GlobalSymbolId,
    /// How the symbol was used.
    binding: dir::BindingUse,
}

impl CheckState<'_> {
    /// Write every commit since the last write into the module tail.
    pub(in crate::sema) fn write_back(&mut self) -> CompilerResult<()> {
        let failed = self.failed_generic_applications()?;

        // resolve committed node types, storing their reduced heads
        for node in self.node_types.nodes() {
            let ty = self.node_types.get(&node).expect("collected node type");
            let resolved = self.fully_resolve(ty, &failed)?;
            let resolved = match self.is_checking() {
                true => match self.node_origin_maybe(node) {
                    Some(origin) => self.deeply_resolve(origin, resolved)?,
                    None => resolved,
                },
                false => resolved,
            };
            self.node_types.insert(node, resolved);
            let written = self
                .module
                .types_tail
                .get_node_type_id(node)
                .or_else(|| self.module.types.get_node_type_id(node));
            if written != Some(resolved) {
                self.module.types_tail.set_node_type(node, resolved);
            }
        }

        // resolve contextual expectations that differ from their node types
        for node in self.expected_types.nodes() {
            let ty = self
                .expected_types
                .get(&node)
                .expect("collected expected type");
            let resolved = self.fully_resolve(ty, &failed)?;
            self.expected_types.insert(node, resolved);
            if self.node_types.get(&node) != Some(resolved) {
                self.module.types_tail.set_expected_type(node, resolved);
            }
        }

        // resolve declaration types and normalize their declared entries
        for index in 0..self.declaration_types.len() {
            let (symbol, ty) = self
                .declaration_types
                .get_index(index)
                .map(|(k, v)| (*k, *v))
                .expect("indexed entry");
            let resolved = self.fully_resolve(ty, &failed)?;
            self.declaration_types[index] = resolved;
            self.write_symbol_type(symbol, resolved);
        }

        // resolve binding types
        for index in 0..self.binding_types.len() {
            let (symbol, ty) = self
                .binding_types
                .get_index(index)
                .map(|(k, v)| (*k, *v))
                .expect("indexed entry");
            let resolved = self.fully_resolve(ty, &failed)?;
            self.binding_types[index] = resolved;
            self.write_symbol_type(symbol, resolved);
        }

        // resolve the types every segment this pass wrote carries
        let module = self.module_id;
        self.resolve_segment_types(dir::DecisionSegment::new(module), |state| {
            &mut state.decisions
        })?;
        self.resolve_segment_types(dir::DefinitionSegment::new(module), |state| {
            &mut state.definitions_tail
        })?;
        self.resolve_segment_types(dir::DecoratorSegment::new(module), |state| {
            &mut state.decorators_tail
        })?;
        self.resolve_segment_types(dir::StaticSegment::new(module), |state| {
            &mut state.statics_tail
        })?;
        self.resolve_segment_types(dir::CoercionSegment::new(module), |state| {
            &mut state.coercions
        })?;
        self.resolve_segment_types(dir::CaptureSegment::new(module), |state| {
            &mut state.captures
        })?;
        self.resolve_segment_types(dir::AutoSegment::new(module), |state| &mut state.auto)?;
        self.resolve_segment_types(dir::GenericSegment::new(module), |state| {
            &mut state.generics_tail
        })?;

        // record the instantiations the committed bodies perform
        if self.is_checking() {
            self.record_instantiations();
        }

        Ok(())
    }

    /// Record every instantiation the committed decisions and conversions perform.
    fn record_instantiations(&mut self) {
        // collect selections that bind generic arguments, with their governing template
        let mut instantiations = Vec::new();
        for (node, decision) in self.module.decisions.decision_entries() {
            decision.for_each_selection(&mut |selection| {
                if selection.arguments.is_empty() {
                    return;
                }

                instantiations.push(dir::Instantiation {
                    owner: self.governing_template_symbol(node),
                    selection: selection.clone(),
                    source: node,
                });
            });
        }

        // collect instantiating conversions behind callable references
        for (node, coercion) in self.module.coercions.coercions() {
            for adjustment in &coercion.adjustments {
                let dir::CoercionAdjustment::Instantiate { arguments, .. } = adjustment else {
                    continue;
                };
                let Some(dir::Decision::Function(dir::OperationResolution::One(value))) =
                    self.module.decisions.decision(node)
                else {
                    continue;
                };
                let Some(symbol) = value.target.symbol() else {
                    continue;
                };

                instantiations.push(dir::Instantiation {
                    owner: self.governing_template_symbol(node),
                    selection: dir::Selection::new(symbol, arguments.clone()),
                    source: node,
                });
            }
        }

        // write the collected instantiations into this pass's segment
        for instantiation in instantiations {
            self.module.generics_tail.push_instantiation(instantiation);
        }
    }

    /// Return the innermost parameterized declaration enclosing one node.
    fn governing_template_symbol(&self, node: dir::GlobalNodeIdAny) -> Option<dir::GlobalSymbolId> {
        // climb structural parents until a parameterized declaration owns the node
        let tree = &self.module.parsed.tree;
        let mut current = node.local_id.id;
        while let Some(parent) = tree.get_parent(current) {
            if let Some(symbol) = self.module.declaration_symbol(parent)
                && let Some(template) = self.loaded_symbol_template(symbol)
                && let Some(template) = self.generic_template(template)
                && template.parameters.iter().any(|parameter| {
                    !self.is_lifetime_parameter(parameter.into_global(symbol.module_id))
                })
            {
                return Some(symbol);
            }

            current = parent.id;
        }

        None
    }

    /// Record the symbol uses this pass proved in the flow segment.
    pub(in crate::sema) fn write_flows(&mut self) {
        let mut uses: Vec<SymbolUse> = Vec::new();

        // collect named references from every carried segment, covering reads by name
        let declared = self.module.declared.clone();
        let elaborated = self.module.elaborated.clone();
        let carried = [
            declared.as_deref().map(|declared| &declared.resolutions),
            elaborated
                .as_deref()
                .map(|elaborated| &elaborated.resolutions),
        ];
        for segment in carried.into_iter().flatten() {
            for (node, resolution) in segment.name_entries() {
                if let Some(symbol) = resolution.single_symbol() {
                    uses.push(SymbolUse {
                        node: Some(node),
                        symbol,
                        binding: dir::BindingUse::READ,
                    });
                }
            }
        }
        for (node, resolution) in self.module.resolutions.name_entries() {
            if let Some(symbol) = resolution.single_symbol() {
                uses.push(SymbolUse {
                    node: Some(node),
                    symbol,
                    binding: dir::BindingUse::READ,
                });
            }
        }

        // collect keyed member selections and written targets from the decisions
        for (node, decision) in self.module.decisions.decision_entries() {
            match decision {
                dir::Decision::Member(member) => match member {
                    dir::OperationResolution::One(access) => {
                        collect_member_target_uses(
                            node,
                            &access.target,
                            dir::BindingUse::READ,
                            &mut uses,
                        );
                    }
                    dir::OperationResolution::Union { arms, .. } => {
                        for access in arms {
                            collect_member_target_uses(
                                node,
                                &access.target,
                                dir::BindingUse::READ,
                                &mut uses,
                            );
                        }
                    }
                },
                dir::Decision::Assignment(assignment) => match &assignment.write {
                    dir::WriteResolution::Binding { symbol, .. } => {
                        uses.push(SymbolUse {
                            node: Some(node),
                            symbol: *symbol,
                            binding: dir::BindingUse::WRITTEN,
                        });
                    }
                    dir::WriteResolution::Member(member) => match member {
                        dir::OperationResolution::One(access) => {
                            collect_member_target_uses(
                                node,
                                &access.target,
                                dir::BindingUse::WRITTEN,
                                &mut uses,
                            );
                        }
                        dir::OperationResolution::Union { arms, .. } => {
                            for access in arms {
                                collect_member_target_uses(
                                    node,
                                    &access.target,
                                    dir::BindingUse::WRITTEN,
                                    &mut uses,
                                );
                            }
                        }
                    },
                    _ => {}
                },
                _ => {}
            }
        }

        // collect closure uses from the captures
        for capture in self.module.captures.capture_by_function.values() {
            for binding in &capture.captures {
                uses.push(SymbolUse {
                    node: None,
                    symbol: binding.symbol(),
                    binding: dir::BindingUse::CAPTURED,
                });
            }
        }

        // record each use against its owning module, keeping the occurrence node
        for SymbolUse {
            node,
            symbol,
            binding: binding_use,
        } in uses
        {
            if symbol.module_id == self.module_id {
                self.module.flows.record_use(symbol.local_id, binding_use);
                if let Some(node) = node
                    && node.module_id == self.module_id
                {
                    self.module.flows.record_occurrence(
                        node.local_id,
                        symbol.local_id,
                        binding_use,
                    );
                }
            } else {
                self.module.flows.record_foreign_use(symbol, binding_use);
            }
        }
    }

    /// Resolve every type one written module segment carries.
    fn resolve_segment_types<S: TypeFold>(
        &mut self,
        replacement: S,
        select: impl Fn(&mut CheckModuleState) -> &mut S,
    ) -> CompilerResult<()> {
        // fold the segment outside the module, since resolving reads the rest of the state
        let mut segment = std::mem::replace(select(&mut self.module), replacement);

        // selections keep the types they chose, so only their variables resolve
        let intact = FxIndexSet::default();
        segment.map_types(&mut |ty| self.fully_resolve(ty, &intact))?;
        *select(&mut self.module) = segment;

        Ok(())
    }

    /// Write one settled symbol type over whatever entry the artifact already carries.
    fn write_symbol_type(&mut self, symbol: dir::GlobalSymbolId, resolved: dir::GlobalTypeId) {
        let written = self
            .module
            .types_tail
            .get_symbol_type_id(symbol)
            .or_else(|| self.module.types.get_symbol_type_id(symbol));
        if written != Some(resolved) {
            self.module.types_tail.set_symbol_type(symbol, resolved);
        }
    }

    /// Resolve one committed type into the canonical form the write boundary requires.
    ///
    /// Every type a retained reference reaches after the write carries no inference variable,
    /// settles every closed computation to its result, normalizes every union and intersection,
    /// composes every carrier in canonical order, keeps the written face of plain-valued alias
    /// applications, and leaves parameter-dependent computations open for monomorphization.
    pub(in crate::sema) fn fully_resolve(
        &mut self,
        ty: dir::GlobalTypeId,
        failed: &FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // track the roots on the active path to break cycles, replaying settled subgraphs
        let ty = self.shallow_resolve(ty)?;
        let mut active = FxIndexSet::default();
        let mut settled = FxIndexMap::default();
        let resolved = self.resolve_open_type(ty, failed, &mut active, &mut settled)?;

        // require the write to close, since a pass exports solutions and holes only
        if self.type_flags(resolved)?.has_variable() {
            return Err(CompilerError::Internal {
                message: format!("check variable survived the write of {resolved:?}"),
            });
        }

        Ok(resolved)
    }

    /// Resolve one open type graph, cycling through solved variable roots.
    fn resolve_open_type(
        &mut self,
        id: dir::GlobalTypeId,
        failed: &FxIndexSet<dir::GlobalTypeId>,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
        settled: &mut FxIndexMap<dir::GlobalTypeId, dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // poison a generic application whose declared argument bound failed
        if failed.contains(&id) {
            return self.intern_type(dir::Type::Error);
        }

        // replay the form this walk already resolved for the root
        if let Some(done) = settled.get(&id).copied() {
            return Ok(done);
        }

        // stop at a root already on the active path
        if !active.insert(id) {
            return Ok(id);
        }

        // resolve a variable through its root, erroring unsolved ones
        let ty = self.ty(id)?;
        let resolved = if let dir::Type::Variable(variable) = ty {
            match self.infer.solution(variable)? {
                Some(solution) => {
                    let solution = self.shallow_resolve(solution)?;

                    self.resolve_open_type(solution, failed, active, settled)?
                }
                // a declaration writes every type it carries, so nothing stays open
                None if self.is_declaration() => {
                    return Err(CompilerError::Internal {
                        message: format!("declaration variable {variable:?} left unsolved"),
                    });
                }
                None => self.intern_type(dir::Type::Error)?,
            }
        }
        // keep foreign types; their own module writes them back
        else if !self.is_own_module(id.module_id) {
            id
        }
        // rebuild a composite around its resolved children
        else {
            let rebuilt =
                self.map_type_children(id.module_id, id.module_id, ty, &mut |state, child| {
                    state.resolve_open_type(child, failed, active, settled)
                })?;

            // renormalize solved unions like any other construction
            let rebuilt = match rebuilt {
                dir::Type::Union(union) => {
                    let elements = self.type_ids(id.module_id, union.elements)?.to_vec();

                    self.normalized_union_type(elements)?
                }
                dir::Type::Intersection(intersection) => {
                    let elements = self.type_ids(id.module_id, intersection.elements)?.to_vec();

                    self.normalized_intersection_type(elements)?
                }
                rebuilt => self.intern_type(rebuilt)?,
            };

            self.settle_computation(rebuilt)?
        };
        active.swap_remove(&id);
        settled.insert(id, resolved);

        Ok(resolved)
    }

    /// Settle one closed computation head to the type it reduces to.
    fn settle_computation(&mut self, id: dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId> {
        // typeof and associated projections settle at check, where value types exist
        if self.is_declaration() {
            return Ok(id);
        }

        // leave parameter-dependent computations open for monomorphization
        let flags = self.type_flags(id)?;
        if flags.has_parameter() || flags.has_this() || flags.has_variable() {
            return Ok(id);
        }

        // settle written projections and operations, plus the alias names whose values reach one
        let computes = match self.ty(id)? {
            dir::Type::Operation(_) | dir::Type::Member(_) => true,
            dir::Type::Application(instance) => self.alias_computes(instance.symbol)?,
            dir::Type::Reference(reference) => self.alias_computes(reference.symbol)?,
            _ => false,
        };
        if !computes {
            return Ok(id);
        }

        self.normalize_closed(id)
    }

    /// Return whether one type alias declares a computation as its value.
    pub(in crate::sema) fn alias_computes(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let mut named = FxIndexSet::default();

        self.name_computes(symbol, &mut named)
    }

    /// Return whether the family one declared name stands for reaches a computation.
    fn name_computes(
        &mut self,
        symbol: dir::GlobalSymbolId,
        named: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<bool> {
        if !named.insert(symbol) {
            return Ok(false);
        }

        // only type aliases stand for a family; every other declaration names itself
        let value = match self.definition(symbol)? {
            Some(dir::Definition::TypeAlias(alias)) => alias.value,
            _ => return Ok(false),
        };

        self.value_computes(value, named)
    }

    /// Return whether one declared alias value reaches a computation.
    fn value_computes(
        &mut self,
        value: dir::GlobalTypeId,
        named: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<bool> {
        match self.ty(value)? {
            dir::Type::Operation(_) | dir::Type::Member(_) => Ok(true),
            dir::Type::Application(instance) => self.name_computes(instance.symbol, named),
            dir::Type::Reference(reference) => self.name_computes(reference.symbol, named),

            // a composition reaches a computation through any of its members
            dir::Type::Union(union) => {
                let elements = self.type_ids(value.module_id, union.elements)?.to_vec();
                for element in elements {
                    if self.value_computes(element, named)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            dir::Type::Intersection(intersection) => {
                let elements = self
                    .type_ids(value.module_id, intersection.elements)?
                    .to_vec();
                for element in elements {
                    if self.value_computes(element, named)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            _ => Ok(false),
        }
    }
}

/// Collect the declared symbols one member target selects.
fn collect_member_target_uses(
    node: dir::GlobalNodeIdAny,
    target: &dir::MemberTarget,
    binding: dir::BindingUse,
    uses: &mut Vec<SymbolUse>,
) {
    match target {
        // record the declared field a nominal access selects
        dir::MemberTarget::Field(field) => {
            if let dir::FieldTarget::Member { symbol, .. } = field.target {
                uses.push(SymbolUse {
                    node: Some(node),
                    symbol,
                    binding,
                });
            }
        }
        // record the declared member a symbol access selects
        dir::MemberTarget::Symbol(candidate) => uses.push(SymbolUse {
            node: Some(node),
            symbol: candidate.selection.symbol,
            binding,
        }),
        // walk grouped targets member by member
        dir::MemberTarget::Existential(targets) | dir::MemberTarget::Intersection(targets) => {
            for target in targets {
                collect_member_target_uses(node, target, binding, uses);
            }
        }
        dir::MemberTarget::Projection { .. }
        | dir::MemberTarget::Call(_)
        | dir::MemberTarget::Index(_) => {}
    }
}
