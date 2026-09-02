use std::collections::VecDeque;

use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;

use crate::sema::{CheckState, Origin, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

use super::entry::Materialization;

/// Cap on instantiation chain depth, guarding polymorphic recursion.
const INSTANCE_DEPTH_LIMIT: u32 = 128;

/// The worklist one materialize pass drains to its fixpoint.
#[derive(Default)]
pub(in crate::sema) struct InstanceWorklist {
    /// The interned instances by structural identity.
    seen: FxIndexMap<dir::InstanceKey, dir::LocalInstanceId>,
    /// The interned instances awaiting materialization.
    queue: VecDeque<(dir::LocalInstanceId, u32)>,
}

impl CheckState<'_> {
    /// Materialize the instances this module's instantiations reach, to a fixpoint.
    pub(in crate::sema) fn materialize_instances(
        &mut self,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // read the checked stage this pass materializes
        let checked = self
            .module
            .checked
            .clone()
            .ok_or_else(|| CompilerError::Internal {
                message: "materialize runs over a checked module".to_string(),
            })?;

        // seed from the module level instantiations that checking recorded
        let instantiations: Vec<_> = checked.generics.iter_instantiations().cloned().collect();
        for instantiation in instantiations {
            let is_root = match instantiation.owner {
                None => true,
                Some(owner) => self.symbol_template_is_region_only(owner)?,
            };
            if is_root {
                self.intern_instance(
                    instantiation.key.symbol,
                    instantiation.key.receiver,
                    instantiation.key.arguments,
                    instantiation.source,
                    dir::InstanceOrigin::Instantiation,
                    0,
                    worklist,
                    None,
                )?;
            }
        }

        // drain the worklist until no new instance appears
        while let Some((instance, depth)) = worklist.queue.pop_front() {
            self.materialize_instance(instance, depth, worklist)?;
        }

        Ok(())
    }

    /// Evaluate the computations one substituted type reaches, reporting any a closed type keeps.
    pub(super) fn evaluate_closed_type(
        &mut self,
        origin: Origin,
        substituted: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !self.has_reachable_computation(substituted)? {
            return Ok(substituted);
        }
        let resolved = self.evaluate_type(origin, substituted)?;

        // report an operation surviving on a fully closed type
        let flags = self.type_flags(resolved)?;
        let is_closed = !flags.has_parameter()
            && !flags.has_variable()
            && !flags.has_infer()
            && !flags.has_this()
            && !flags.has_error();
        if is_closed && let Some(operation) = self.reachable_operation(resolved)? {
            self.report_type_computation_not_reduced(origin, operation)?;
        }

        Ok(resolved)
    }

    /// Return the first type operation one type reaches, when any survives.
    fn reachable_operation(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let kind = self.ty(id)?;
            if matches!(kind, dir::Type::Operation(_)) {
                return Ok(Some(id));
            }
            self.for_each_type_child(id.module_id, &kind, |child| pending.push(child))?;
        }

        Ok(None)
    }

    /// Return whether one type reaches a computation needing evaluation.
    pub(in crate::sema) fn has_reachable_computation(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // walk the type graph, stopping at the first computation
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }

            let kind = self.ty(id)?;
            match &kind {
                dir::Type::Operation(_) | dir::Type::Member(_) | dir::Type::Intersection(_) => {
                    return Ok(true);
                }
                dir::Type::Application(application)
                    if self
                        .language_item(application.symbol)?
                        .is_some_and(crate::sema::language::is_type_computation)
                        || self.is_computed_alias(application.symbol)?
                        || self.is_partial_application(id.module_id, application)? =>
                {
                    return Ok(true);
                }
                _ => {}
            }

            self.for_each_type_child(id.module_id, &kind, |child| pending.push(child))?;
        }

        Ok(false)
    }

    /// Intern an instance for every concrete application inside one type.
    pub(in crate::sema) fn intern_applications(
        &mut self,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // scan the type graph, interning every applied generic template
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }

            let ty = self.ty(id)?;
            if let dir::Type::Application(application) = &ty {
                self.intern_application(id, application, source, depth, worklist)?;
            }
            // pair a bare reference to a defaulted template like an empty application
            else if let dir::Type::Reference(reference) = &ty
                && self.symbol_template(reference.symbol)?.is_some()
            {
                let application = dir::GenericApplication {
                    symbol: reference.symbol,
                    arguments: dir::TypeListId::EMPTY,
                };
                self.intern_application(id, &application, source, depth, worklist)?;
            }

            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(())
    }

    /// Intern one written application as an instance through its canonical pairing.
    fn intern_application(
        &mut self,
        id: dir::GlobalTypeId,
        application: &dir::GenericApplication,
        source: dir::GlobalNodeIdAny,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<Option<dir::LocalInstanceId>> {
        // skip an application that pairs with no instance
        let Ok(substitution) = self.instance_substitution(id.module_id, application) else {
            return Ok(None);
        };

        let mut regions = Vec::new();
        let instance = self.intern_instance(
            application.symbol,
            None,
            substitution.bindings.to_vec(),
            source,
            dir::InstanceOrigin::Application,
            depth,
            worklist,
            Some(&mut regions),
        )?;

        // record the instance this closed application selects and the regions it substitutes
        if let Some(instance) = instance {
            self.module
                .generics_tail
                .bind_application_instance(id, instance);
            self.module
                .generics_tail
                .bind_application_regions(id, regions);
        }

        Ok(instance)
    }

    /// Rewrite one instance argument with every lifetime bound to its positional literal.
    fn erase_argument_lifetimes(
        &mut self,
        id: dir::GlobalTypeId,
        visiting: &mut Vec<dir::GlobalTypeId>,
        regions: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep a revisited cycle member as written
        if visiting.contains(&id) {
            return Ok(id);
        }

        // bind a lifetime term to the region at its position in the argument list
        if self.memory_kind(id)? == Some(dir::MemoryParameter::Region) {
            let position = regions.len() as u32;
            regions.push(id);

            return self.lifetime_literal(dir::Lifetime::Bound(position));
        }

        // rebuild the children with their lifetimes bound
        visiting.push(id);
        let ty = self.ty(id)?;
        let rebuilt = self.map_type_children(id.module_id, ty, &mut |state, child| {
            state.erase_argument_lifetimes(child, visiting, regions)
        })?;
        visiting.pop();

        self.intern_type(rebuilt)
    }

    /// Ground the induced place and space parameters one argument carries at local.
    fn ground_induced_memory_argument(
        &mut self,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // collect the induced place terms the argument reaches
        let mut induced = Vec::new();
        let mut pending = vec![argument];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }

            let kind = self.ty(id)?;
            if let dir::Type::Parameter(parameter) = &kind
                && self.generic_parameter(*parameter).is_some_and(|binding| {
                    matches!(
                        binding.induced_memory_parameter(),
                        Some(dir::MemoryParameter::Place | dir::MemoryParameter::Space)
                    )
                })
            {
                induced.push(id);
            }

            self.for_each_type_child(id.module_id, &kind, |child| pending.push(child))?;
        }

        // replace each induced place term with the ambient local space
        let mut argument = argument;
        for from in induced {
            let to = self.local_place()?;
            argument = self.replace_type(argument.module_id, argument, from, to)?;
        }

        Ok(argument)
    }

    /// Return whether one type mentions an open type or place parameter.
    pub(in crate::sema) fn has_open_parameter(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // walk the type graph, stopping at the first open parameter
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }

            let kind = self.ty(id)?;
            match &kind {
                dir::Type::Parameter(parameter) if !self.is_lifetime_parameter(*parameter) => {
                    return Ok(true);
                }
                dir::Type::Erased(_) => return Ok(true),
                _ => {}
            }

            self.for_each_type_child(id.module_id, &kind, |child| pending.push(child))?;
        }

        Ok(false)
    }

    /// Intern one instantiation as an instance, deduplicating structurally.
    fn intern_instance(
        &mut self,
        template: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        bindings: Vec<dir::GenericArgumentBinding>,
        source: dir::GlobalNodeIdAny,
        introduced: dir::InstanceOrigin,
        depth: u32,
        worklist: &mut InstanceWorklist,
        substituted: Option<&mut Vec<dir::GlobalTypeId>>,
    ) -> CompilerResult<Option<dir::LocalInstanceId>> {
        // reject runaway polymorphic recursion past the depth limit
        if depth > INSTANCE_DEPTH_LIMIT {
            return Err(CompilerError::Internal {
                message: format!(
                    "instantiation chain exceeded depth {INSTANCE_DEPTH_LIMIT} closing {template:?}"
                ),
            });
        }

        // resolve the arguments, keeping type parameters and dropping lifetimes
        let origin = Origin::Node(source, None);
        let mut arguments = Vec::new();
        let mut regions = Vec::new();
        for binding in bindings {
            if self.is_lifetime_parameter(binding.parameter) {
                continue;
            }

            let argument = self.deeply_resolve(origin, binding.argument)?;
            let argument =
                self.erase_argument_lifetimes(argument, &mut Vec::new(), &mut regions)?;
            let argument = self.ground_induced_memory_argument(argument)?;

            // leave open instantiations to close under their enclosing instance
            let flags = self.type_flags(argument)?;
            if flags.has_variable() || flags.has_this() || flags.has_infer() {
                return Ok(None);
            }
            if flags.has_parameter() && self.has_open_parameter(argument)? {
                return Ok(None);
            }

            arguments.push(dir::GenericArgumentBinding::new(
                binding.parameter,
                argument,
            ));
        }

        // resolve the receiver like an argument, leaving open ones to their enclosing instance
        let receiver = match receiver {
            Some(receiver) => {
                let resolved = self.deeply_resolve(origin, receiver)?;
                let resolved =
                    self.erase_argument_lifetimes(resolved, &mut Vec::new(), &mut regions)?;
                let resolved = self.ground_induced_memory_argument(resolved)?;
                let flags = self.type_flags(resolved)?;
                if flags.has_variable() || flags.has_this() || flags.has_infer() {
                    return Ok(None);
                }
                if flags.has_parameter() && self.has_open_parameter(resolved)? {
                    return Ok(None);
                }

                Some(resolved)
            }
            None => None,
        };

        // hand the substituted region terms to an application recording them
        let bound = (0..regions.len() as u32)
            .map(|position| self.lifetime_literal(dir::Lifetime::Bound(position)))
            .collect::<CompilerResult<Vec<_>>>()?;
        let bound = self.intern_type_ids(&bound)?;
        if let Some(substituted) = substituted {
            *substituted = regions;
        }

        // require a receiver or one type argument to close on
        if arguments.is_empty() && receiver.is_none() {
            return Ok(None);
        }

        // close unbound place parameters at local, dropping every other unbound selection
        if let Some(template_id) = self.symbol_template(template)?
            && let Some(declared) = self.generic_template(template_id)
        {
            let parameters = declared.parameters.clone();
            for parameter in parameters {
                let parameter = parameter.into_global(template_id.module_id);
                if self.is_lifetime_parameter(parameter) {
                    continue;
                }
                if arguments
                    .iter()
                    .any(|binding| binding.parameter == parameter)
                {
                    continue;
                }

                // close an unbound ambient place parameter at local
                let is_place = self.generic_parameter(parameter).is_some_and(|binding| {
                    matches!(
                        binding.memory_parameter(),
                        Some(dir::MemoryParameter::Place | dir::MemoryParameter::Space)
                    )
                });
                if is_place {
                    let local = self.local_place()?;
                    arguments.push(dir::GenericArgumentBinding::new(parameter, local));

                    continue;
                }

                // drop the selection at every other unbound parameter
                return Ok(None);
            }
        }

        // resolve an interface requirement to the member its receiver conformance selects
        if self.interface_member_owner(template).is_some() {
            // require a receiver to close a requirement into a static callable
            let Some(receiver_type) = receiver else {
                return Ok(None);
            };

            // intern the implementing member in place of the requirement
            if let Some((member, member_arguments)) =
                self.requirement_implementation(origin, template, receiver_type)?
            {
                let redirected = self.intern_instance(
                    member,
                    receiver,
                    member_arguments,
                    source,
                    introduced,
                    depth + 1,
                    worklist,
                    None,
                )?;

                // record the selection so lowering dispatches through the implementer
                if let Some(redirected) = redirected
                    && let Some(implementer) = self
                        .module
                        .generics_tail
                        .get_local_instance(redirected)
                        .map(|instance| instance.key.clone())
                {
                    let requirement =
                        dir::InstanceKey::new(template, arguments).with_receiver(receiver);
                    self.module
                        .generics_tail
                        .bind_dispatch_selection(requirement, implementer);
                }

                return Ok(redirected);
            }
        }

        // allocate one instance per distinct closed identity
        let key = dir::InstanceKey::new(template, arguments.clone()).with_receiver(receiver);
        if let Some(admitted) = worklist.seen.get(&key).copied() {
            // upgrade the instance a type application introduced, re-closing it under the instantiation
            if introduced == dir::InstanceOrigin::Instantiation
                && let Some(interned) = self.module.generics_tail.get_local_instance(admitted)
                && interned.origin == dir::InstanceOrigin::Application
            {
                self.module
                    .generics_tail
                    .set_instance_origin(admitted, introduced);
                worklist.queue.push_back((admitted, depth));
            }

            return Ok(Some(admitted));
        }

        // allocate and queue the new instance
        let instance = self.module.generics_tail.push_instance(dir::Instance {
            conformances: dir::AutoInterfaceSet::new(),
            key: dir::InstanceKey::new(template, arguments).with_receiver(receiver),
            source,
            origin: introduced,
            regions: bound,
        });
        worklist.seen.insert(key, instance);
        worklist.queue.push_back((instance, depth));

        Ok(Some(instance))
    }

    /// Return the member and arguments one receiver's conformance selects for a requirement.
    fn requirement_implementation(
        &mut self,
        origin: Origin,
        requirement: dir::GlobalSymbolId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::GlobalSymbolId, Vec<dir::GenericArgumentBinding>)>> {
        // read the interface the requirement belongs to
        let Some(owner) = self.interface_member_owner(requirement) else {
            return Ok(None);
        };
        let Some(dir::Definition::Interface(interface)) = self.definition(owner)?.cloned() else {
            return Ok(None);
        };

        // read the slot key the requirement declares
        let Some(slot) = interface
            .members
            .iter()
            .find_map(|declared| match declared {
                dir::DefinitionMember::Method(method) if method.symbol == requirement => {
                    Some(method.slot)
                }
                _ => None,
            })
        else {
            return Ok(None);
        };
        let dir::MemberSlot::Key(key) = slot else {
            return Ok(None);
        };

        // look the slot up on the concrete receiver
        let module = origin.module();
        let subject =
            self.member_subject(origin, receiver, receiver, dir::MemberSpace::Instance)?;
        let lookup = self.lookup_member(origin, module, subject, key)?;

        // take the one candidate implementing this requirement
        let mut selected = None;
        for candidate in &lookup {
            let Some(declared) = candidate.declaration() else {
                continue;
            };
            if declared.requirement != Some(owner) || !declared.site_parameters.is_empty() {
                continue;
            }

            if selected.is_some() {
                return Err(CompilerError::Internal {
                    message: "a requirement selected two implementations".to_string(),
                });
            }
            selected = Some((declared.symbol, declared.generic_arguments.clone()));
        }

        Ok(selected)
    }

    /// Close one instance: admit its template's instantiations and resolve its types.
    fn materialize_instance(
        &mut self,
        instance: dir::LocalInstanceId,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // read the instance and the substitution its arguments select
        let interned = self
            .module
            .generics_tail
            .get_local_instance(instance)
            .ok_or_else(|| CompilerError::Internal {
                message: "closed an unallocated instance".to_string(),
            })?
            .clone();
        let origin = Origin::Node(interned.source, None);
        let mut substitution = TypeSubstitution {
            bindings: interned.key.arguments.iter().copied().collect(),
            receiver: None,
        };
        substitution.receiver = match interned.key.receiver {
            Some(receiver) => Some(receiver),
            None => self.instance_receiver(interned.key.symbol, origin, &substitution)?,
        };

        // substitute the template's recorded instantiations under this instance
        let mut reached = Vec::new();
        for instantiation in self.template_instantiations(interned.key.symbol)? {
            let mut arguments = Vec::new();
            for binding in instantiation.key.arguments {
                arguments.push(dir::GenericArgumentBinding::new(
                    binding.parameter,
                    self.substitute_type(binding.argument, &substitution)?,
                ));
            }
            let receiver = match instantiation.key.receiver {
                Some(receiver) => Some(self.substitute_type(receiver, &substitution)?),
                None => None,
            };
            reached.push((instantiation.key.symbol, receiver, arguments));
        }

        // intern each substituted instantiation, one chain step deeper
        for (template, receiver, arguments) in reached {
            self.intern_instance(
                template,
                receiver,
                arguments,
                interned.source,
                interned.origin,
                depth + 1,
                worklist,
                None,
            )?;
        }

        // close the hook instance destructors call on this nominal
        if let Some(member) = self.drop_hook_member(interned.key.symbol)? {
            match self.bind_drop_hook(member, &interned.key.arguments)? {
                Some(bindings) => {
                    self.intern_instance(
                        member,
                        None,
                        bindings,
                        interned.source,
                        dir::InstanceOrigin::Instantiation,
                        depth + 1,
                        worklist,
                        None,
                    )?;
                }
                None => {
                    self.report_unmapped_drop_conformance(interned.source);
                }
            }
        }

        // an interface member's entries spell the receiver as the owner's self application
        let owner_self = match substitution.receiver {
            Some(_) => self.interface_owner_self(interned.key.symbol)?,
            None => None,
        };

        // materialize the template's entries under the substitution, one chain step deeper
        let materialization = Materialization {
            substitution: Some(&substitution),
            owner_self,
            instance: Some(instance),
            anchor: Some(interned.source),
            depth: depth + 1,
        };
        self.materialize_template_entries(&materialization, interned.key.symbol, worklist)
    }
}
