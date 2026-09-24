use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_dir::{InstanceKeyVisit, TypeFold};
use destack_source::ModuleId;

use crate::sema::{CheckModuleState, CheckState, Origin, ProjectionMemo};
use crate::{CompilerError, CompilerResult};

impl<'a> CheckState<'a> {
    /// Write every commit since the last write into the module tail.
    pub(in crate::sema) fn write_back(&mut self) -> CompilerResult<()> {
        // share normalized heads across the write, keyed by assuming scope
        let mut normalized: FxIndexMap<
            (Option<dir::GlobalGenericTemplateId>, dir::GlobalTypeId),
            dir::GlobalTypeId,
        > = FxIndexMap::default();

        // resolve committed node types, storing their reduced heads
        for node in self.node_types.nodes() {
            let ty = self.node_types.get(&node).expect("collected node type");
            let resolved = self.fully_resolve(ty)?;
            let resolved = match self.is_checking() {
                true => match self.node_origin_maybe(node) {
                    Some(origin) => {
                        self.deeply_resolve_shared(origin, resolved, &mut normalized)?
                    }
                    None => resolved,
                },
                false => resolved,
            };
            self.node_types.insert(node, resolved);
            let committed = self
                .module
                .types_tail
                .get_node_type_id(node)
                .or_else(|| self.module.types.get_node_type_id(node));
            if committed != Some(resolved) {
                self.module.types_tail.set_node_type(node, resolved);
            }
        }

        // commit coroutine creations after their captured types settle
        if self.is_checking() {
            self.commit_coroutine_creations()?;
        }

        // resolve this module's declaration types
        for index in 0..self.declaration_types.len() {
            let (symbol, ty) = self
                .declaration_types
                .get_index(index)
                .map(|(k, v)| (*k, *v))
                .expect("indexed entry");
            if !self.is_own_module(symbol.module_id) {
                continue;
            }
            let resolved = self.fully_resolve(ty)?;
            let resolved = match self.is_checking() {
                true => {
                    self.deeply_resolve_shared(Origin::Symbol(symbol), resolved, &mut normalized)?
                }
                false => resolved,
            };
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
            let resolved = self.fully_resolve(ty)?;
            let resolved = match self.is_checking() {
                true => {
                    self.deeply_resolve_shared(Origin::Symbol(symbol), resolved, &mut normalized)?
                }
                false => resolved,
            };
            self.binding_types[index] = resolved;
            self.write_symbol_type(symbol, resolved);
        }

        // resolve the types in every segment this pass wrote
        let module = self.module_id;
        self.resolve_segment_types(dir::DecisionSegment::new(module), |state| {
            &mut state.decisions_tail
        })?;
        if self.is_checking() {
            self.settle_decision_projections()?;
        }
        self.resolve_segment_types(dir::DecoratorSegment::new(module), |state| {
            &mut state.decorators_tail
        })?;
        self.resolve_segment_types(dir::StaticSegment::new(module), |state| {
            &mut state.statics_tail
        })?;
        self.resolve_segment_types(dir::CoercionSegment::new(module), |state| {
            &mut state.coercions_tail
        })?;
        self.resolve_segment_types(dir::CaptureSegment::new(module), |state| {
            &mut state.captures
        })?;
        self.resolve_segment_types(dir::GenericSegment::new(module), |state| {
            &mut state.generics_tail
        })?;

        // commit the instantiations the committed bodies perform
        if self.is_checking() {
            self.commit_instantiations()?;
        }

        Ok(())
    }

    /// Commit every instantiation the committed decisions and conversions perform.
    fn commit_instantiations(&mut self) -> CompilerResult<()> {
        // collect the selections that bind generic arguments under their template, one per key
        let mut instantiations = FxIndexMap::default();
        let mut selections = Vec::new();
        for (node, decision) in self.module.decisions_tail.decision_entries() {
            decision.visit_instance_keys(&mut |selection| {
                if !selection.arguments.is_empty() || selection.receiver.is_some() {
                    selections.push((node, selection.clone()));
                }
            });
        }
        for (node, coercion) in self.module.coercions_tail.coercions() {
            coercion.visit_instance_keys(&mut |selection| {
                if !selection.arguments.is_empty() || selection.receiver.is_some() {
                    selections.push((node, selection.clone()));
                }
            });
        }
        for (node, selection) in selections {
            let owner = self.governing_template_symbol(node)?;
            instantiations.entry((owner, selection)).or_insert(node);
        }

        // collect instantiating conversions behind callable references
        let mut instantiating = Vec::new();
        for (node, coercion) in self.module.coercions_tail.coercions() {
            for adjustment in &coercion.adjustments {
                let dir::CoercionAdjustment::Instantiate { arguments, .. } = adjustment else {
                    continue;
                };
                let Some(dir::Decision::Function(dir::OperationResolution::One(value))) =
                    self.module.decisions_tail.decision(node)
                else {
                    continue;
                };
                let Some(symbol) = value.target.symbol() else {
                    continue;
                };
                instantiating.push((node, symbol, arguments.clone()));
            }
        }
        for (node, symbol, arguments) in instantiating {
            let owner = self.governing_template_symbol(node)?;
            instantiations
                .entry((owner, dir::InstanceKey::new(symbol, arguments)))
                .or_insert(node);
        }

        // keep the whole instantiations without records of reported failures
        for ((owner, key), source) in instantiations {
            // require one exact value where an inferred closed argument fills a consumed parameter
            let mut next_index = 0;
            for binding in &key.arguments {
                if self.is_lifetime_parameter(binding.parameter)? {
                    continue;
                }
                let index = next_index;
                next_index += 1;
                if !self.is_static_const_parameter(binding.parameter)?
                    || self.generic_argument_node(source, index).is_some()
                    || matches!(self.ty(binding.argument)?, dir::Type::Parameter(_))
                {
                    continue;
                }
                let origin = Origin::Node(source, None);
                if !self.has_one_cardinality(origin, binding.argument)? {
                    self.report_argument_not_exact_value(
                        source,
                        binding.argument,
                        binding.parameter,
                    )?;
                }
            }

            let mut poisoned = false;
            let receiver = key.receiver.into_iter();
            let arguments = key.arguments.iter();
            for ty in receiver.chain(arguments.map(|binding| binding.argument)) {
                if self.type_flags(ty)?.has_error() {
                    poisoned = true;
                    break;
                }
            }
            if !poisoned {
                self.module
                    .generics_tail
                    .push_instantiation(dir::Instantiation { owner, key, source });
            }
        }

        Ok(())
    }

    /// Return the innermost parameterized declaration enclosing one node.
    fn governing_template_symbol(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // climb from the node through its structural parents to a parameterized declaration
        let tree = &self.module.parsed.tree;
        let mut current = Some(node.local_id);
        let mut method = None;
        while let Some(parent) = current {
            if let Some(symbol) = self.module.declaration_symbol(parent) {
                // resolve parameterized owners through their selecting method
                let template = self.symbol_template(symbol)?;
                let parameters = match template {
                    Some(template) => self
                        .generic_template(template)?
                        .map(|template| template.parameters.clone())
                        .unwrap_or_default(),
                    None => Vec::new(),
                };
                let mut is_parameterized = false;
                for parameter in parameters {
                    if self
                        .generic_parameter(parameter.into_global(symbol.module_id))?
                        .is_some_and(|binding| binding.is_instance_parameter())
                    {
                        is_parameterized = true;
                        break;
                    }
                }
                if is_parameterized {
                    return Ok(Some(method.unwrap_or(symbol)));
                }

                // keep the innermost method awaiting a parameterized owner
                let is_method = parent
                    .try_into_typed::<dir::Member>()
                    .is_ok_and(|member| matches!(tree.get(member), dir::Member::Method { .. }));
                if method.is_none() && is_method {
                    method = Some(symbol);
                }
            }

            current = tree.get_parent(parent.id);
        }

        Ok(None)
    }

    /// Settle every type of one module segment.
    fn resolve_segment_types<S: TypeFold>(
        &mut self,
        replacement: S,
        select: impl for<'m> Fn(&'m mut CheckModuleState<'a>) -> &'m mut S,
    ) -> CompilerResult<()> {
        // fold the segment outside the module, where resolving reads the whole state
        let mut segment = std::mem::replace(select(&mut self.module), replacement);

        // selections keep the types they chose, so only their variables resolve
        segment.map_types(&mut |ty| self.fully_resolve(ty))?;
        *select(&mut self.module) = segment;

        Ok(())
    }

    /// Settle the projections the committed decisions and places hold.
    fn settle_decision_projections(&mut self) -> CompilerResult<()> {
        let decisions: Vec<_> = self
            .module
            .decisions_tail
            .decision_entries()
            .map(|(node, decision)| (node, decision.clone()))
            .collect();
        let places: Vec<_> = self
            .module
            .decisions_tail
            .place_entries()
            .map(|(node, place)| (node, *place))
            .collect();
        let mut memo = ProjectionMemo::default();
        for (node, mut decision) in decisions {
            if self.settle_node_projections(node, &mut decision, &mut memo)? {
                self.module.decisions_tail.set_decision(node, decision);
            }
        }
        for (node, mut place) in places {
            if self.settle_node_projections(node, &mut place, &mut memo)? {
                self.module.decisions_tail.set_place_resolution(node, place);
            }
        }

        Ok(())
    }

    /// Settle the projections one node's record holds.
    fn settle_node_projections<T: TypeFold>(
        &mut self,
        node: dir::GlobalNodeIdAny,
        record: &mut T,
        memo: &mut ProjectionMemo,
    ) -> CompilerResult<bool> {
        let Some(origin) = self.node_origin_maybe(node) else {
            return Ok(false);
        };
        let mut changed = false;
        record.map_types(&mut |ty| {
            let settled = self.settle_projections(origin, ty, memo)?;
            changed |= settled != ty;

            Ok::<_, CompilerError>(settled)
        })?;

        Ok(changed)
    }

    /// Write one resolved symbol type over the entry the artifact holds.
    fn write_symbol_type(&mut self, symbol: dir::GlobalSymbolId, resolved: dir::GlobalTypeId) {
        let committed = self
            .module
            .types_tail
            .get_symbol_type_id(symbol)
            .or_else(|| self.module.types.get_symbol_type_id(symbol));
        if committed != Some(resolved) {
            self.module.types_tail.set_symbol_type(symbol, resolved);
        }
    }

    /// Settle one committed type into the canonical form writeback requires.
    pub(in crate::sema) fn fully_resolve(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // track the roots on the active path to break cycles, reusing resolved subgraphs
        let ty = self.shallow_resolve(ty)?;
        let mut active = FxIndexSet::default();
        let mut memo = FxIndexMap::default();
        let resolved = self.resolve_open_type(ty, &mut active, &mut memo)?;

        // require the write to close over solutions and holes
        if self.type_flags(resolved)?.has_variable() {
            let survivors = self.type_variables(resolved)?;
            let roles = survivors
                .iter()
                .map(|variable| {
                    format!(
                        "{:?}={:?}",
                        variable,
                        self.infer.variable(*variable).map(|v| v.kind)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

            return Err(CompilerError::Internal {
                message: format!(
                    "check variable survived the write of {} as {resolved:?}: {roles}",
                    self.format_type(resolved)
                ),
            });
        }

        Ok(resolved)
    }

    /// Settle one open type graph, cycling through solved variable roots.
    fn resolve_open_type(
        &mut self,
        id: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
        memo: &mut FxIndexMap<dir::GlobalTypeId, dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // return a closed graph with no computation heads as it stands
        let flags = self.type_flags(id)?;
        if !flags.has_variable()
            && !flags.has_member()
            && !flags.has_operation()
            && !flags.has_reference()
        {
            return Ok(id);
        }

        // reuse the form this walk already resolved for the root
        if let Some(done) = memo.get(&id).copied() {
            return Ok(done);
        }

        // stop at a root already on the active path
        if !active.insert(id) {
            return Ok(id);
        }

        // resolve a variable through its root, erroring unsolved ones
        let ty = self.ty_raw(id)?;
        let resolved = if let dir::Type::Variable(variable) = ty {
            match self.infer.solution(variable)? {
                Some(solution) => {
                    let solution = self.shallow_resolve(solution)?;

                    self.resolve_open_type(solution, active, memo)?
                }
                // write every type of a clean declaration
                None if self.is_declaring()
                    && self.module(self.module_id).diagnostics.is_empty() =>
                {
                    let origin = self.infer.origin(self.infer.variable(variable)?.origin);
                    return Err(CompilerError::Internal {
                        message: format!(
                            "declaration variable {variable:?} at {} left unsolved",
                            self.node_label(self.origin_source(origin)?),
                        ),
                    });
                }
                None => self.intern_type(dir::Type::Error)?,
            }
        }
        // keep foreign types for their module to write back
        else if !self.is_own_module(id.module_id) {
            id
        }
        // rebuild a composite around its resolved children
        else {
            let rebuilt = self.map_type_children(id.module_id, ty, &mut |state, child| {
                state.resolve_open_type(child, active, memo)
            })?;

            // renormalize solved unions like any other construction
            let rebuilt = match rebuilt {
                dir::Type::Union(union) => {
                    let elements = self.type_ids(id.module_id, union.elements)?;

                    self.normalized_union_type(elements.iter().copied())?
                }
                dir::Type::Intersection(intersection) => {
                    let elements = self.type_ids(id.module_id, intersection.elements)?;

                    self.normalized_intersection_type(elements.iter().copied())?
                }
                rebuilt => self.intern_type(rebuilt)?,
            };

            self.resolve_computation(rebuilt)?
        };
        active.swap_remove(&id);
        memo.insert(id, resolved);

        Ok(resolved)
    }

    /// Normalize one closed computation head to the type it reduces to.
    fn resolve_computation(&mut self, id: dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId> {
        // resolve typeof and associated projections at check, a variant named on an enum now
        if self.is_declaring() && !self.is_variant_selection(id)? {
            return Ok(id);
        }

        // leave parameter-dependent computations open for monomorphization
        let flags = self.type_flags(id)?;
        if flags.has_parameter() || flags.has_this() || flags.has_variable() {
            return Ok(id);
        }

        // normalize closed projections and operations, keeping every other head
        if !matches!(self.ty(id)?, dir::Type::Operation(_) | dir::Type::Member(_)) {
            return Ok(id);
        }

        self.normalize_closed(id)
    }

    /// Return whether one type names a variant on an enum declaration.
    fn is_variant_selection(&mut self, id: dir::GlobalTypeId) -> CompilerResult<bool> {
        let dir::Type::Member(member) = self.ty(id)? else {
            return Ok(false);
        };
        let member = self.type_member(id.module_id, member)?;
        if member.qualifier.is_some() {
            return Ok(false);
        }
        let Some(symbol) = self.ty(member.owner)?.symbol() else {
            return Ok(false);
        };

        Ok(self.symbol_kind(symbol)? == dir::SymbolKind::Enum)
    }

    /// Settle each recorded narrowing on the solved members it keeps, dropping the vacuous ones.
    pub(in crate::sema) fn settle_narrowings(&mut self, module: ModuleId) -> CompilerResult<()> {
        let entries: Vec<_> = self
            .module(module)
            .decisions_tail
            .narrowing_entries()
            .map(|(node, narrowing)| (node, narrowing.clone()))
            .collect();
        for (node, narrowing) in entries {
            let origin = Origin::Node(node, None);
            let union = self.fully_resolve(narrowing.union)?;
            let union = self.normalize_type(origin, union)?;
            let narrowed = self.fully_resolve(narrowing.arms[0])?;
            let narrowed = self.normalize_type(origin, narrowed)?;

            // keep the declared members the narrowed type still names
            let members = self.canonical_union_members(origin, union)?;
            let arms = match self.canonical_union_members(origin, narrowed)? {
                Some(arms) => arms.to_vec(),
                None => vec![narrowed],
            };
            let is_projection = members.as_ref().is_some_and(|members| {
                arms.len() < members.len() && arms.iter().all(|arm| members.contains(arm))
            });
            let decisions = &mut self.module_mut(module).decisions_tail;
            match is_projection {
                true => decisions.set_narrowing(node, dir::Narrowing { union, arms }),
                false => decisions.remove_narrowing(node),
            }
        }

        Ok(())
    }
}
