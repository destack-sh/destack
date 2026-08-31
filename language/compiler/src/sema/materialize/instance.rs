use std::collections::VecDeque;

use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{CheckState, Origin, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

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
                )?;
            }
        }

        // drain the worklist until no new instance appears
        while let Some((instance, depth)) = worklist.queue.pop_front() {
            self.materialize_instance(instance, depth, worklist)?;
        }

        Ok(())
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

        let instance = self.intern_instance(
            application.symbol,
            None,
            substitution.bindings.to_vec(),
            source,
            dir::InstanceOrigin::Application,
            depth,
            worklist,
        )?;

        // record the instance this closed application selects
        if let Some(instance) = instance {
            self.module
                .generics_tail
                .bind_application_instance(id, instance);
        }

        Ok(instance)
    }

    /// Rewrite one instance argument with every lifetime erased to the frame literal.
    fn erase_argument_lifetimes(
        &mut self,
        id: dir::GlobalTypeId,
        visiting: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep a revisited cycle member as written
        if visiting.contains(&id) {
            return Ok(id);
        }

        // lifetime terms all erase to one canonical literal
        if self.memory_kind(id)? == Some(dir::MemoryParameter::Region) {
            return self.lifetime_literal(dir::Lifetime::Frame);
        }

        // rebuild the children with their lifetimes erased
        visiting.push(id);
        let ty = self.ty(id)?;
        let rebuilt = self.map_type_children(id.module_id, ty, &mut |state, child| {
            state.erase_argument_lifetimes(child, visiting)
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
        for binding in bindings {
            if self.is_lifetime_parameter(binding.parameter) {
                continue;
            }

            let argument = self.deeply_resolve(origin, binding.argument)?;
            let argument = self.erase_argument_lifetimes(argument, &mut Vec::new())?;
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
                let resolved = self.erase_argument_lifetimes(resolved, &mut Vec::new())?;
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
                    )?;
                }
                None => {
                    self.report_unmapped_drop_conformance(interned.source);
                }
            }
        }

        // materialize the template under the substitution for this instance
        match self.definition(interned.key.symbol)?.cloned() {
            Some(definition) => self.materialize_instance_definition(
                instance,
                origin,
                definition,
                &substitution,
                depth,
                worklist,
            ),
            None => self.materialize_instance_body(
                instance,
                origin,
                interned.key.symbol,
                &substitution,
                depth,
                worklist,
            ),
        }
    }

    /// Materialize one non-generic member body under its owner receiver.
    pub(in crate::sema) fn materialize_member_body(
        &mut self,
        owner: dir::GlobalSymbolId,
        member: dir::GlobalSymbolId,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // resolve what `this` denotes inside the body
        let receiver = match self.definition(owner)?.cloned() {
            Some(dir::Definition::Extension(extension)) => extension.target.r#type(),
            _ => self.intern_type(dir::Type::Application(dir::GenericApplication {
                symbol: owner,
                arguments: dir::TypeListId::EMPTY,
            }))?,
        };
        let substitution = TypeSubstitution::default().with_receiver(receiver);

        // materialize the member's own declared type at its declaration
        let declaration = self
            .binding_table(member.module_id)
            .get_symbol(member.local_id)
            .declaration;
        if let Some(declaration) = declaration
            && let Some(ty) = self.template_symbol_type(member)
        {
            let origin = Origin::Node(declaration, None);
            let resolved = self.materialize_member_type(origin, ty, &substitution, worklist)?;
            if resolved != ty {
                self.module.types_tail.set_symbol_type(member, resolved);
            }
        }

        // materialize each body node's committed type and payloads
        let Some(nodes) = self.template_body_nodes(member)? else {
            return Ok(());
        };
        for node in nodes {
            // rewrite the committed type
            let origin = Origin::Node(node, None);
            if let Some(ty) = self.committed_template_type(member.module_id, node) {
                let resolved = self.materialize_member_type(origin, ty, &substitution, worklist)?;
                if resolved != ty {
                    self.module.types_tail.set_node_type(node, resolved);
                }
            }

            // rewrite the committed decision
            let decision = self.template_decision(member.module_id, node).cloned();
            if let Some(decision) =
                self.materialize_member_payload(origin, decision, &substitution, worklist)?
            {
                self.module.decisions.set_decision(node, decision);
            }

            // rewrite the committed place resolution
            let place = self.template_place(member.module_id, node).cloned();
            if let Some(place) =
                self.materialize_member_payload(origin, place, &substitution, worklist)?
            {
                self.module.decisions.set_place_resolution(node, place);
            }

            // rewrite the committed coercion
            let coercion = self.template_coercion(member.module_id, node).cloned();
            if let Some(coercion) =
                self.materialize_member_payload(origin, coercion, &substitution, worklist)?
            {
                self.module.coercions.bind_coercion(node, coercion);
            }
        }

        Ok(())
    }

    /// Materialize the types one committed payload carries, returning it when they move.
    fn materialize_member_payload<T: dir::TypeFold>(
        &mut self,
        origin: Origin,
        row: Option<T>,
        substitution: &TypeSubstitution,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<Option<T>> {
        // leave an absent payload alone
        let Some(mut row) = row else {
            return Ok(None);
        };

        // rewrite each type the payload carries, keeping the payload where one moves
        let mut is_moved = false;
        dir::TypeFold::map_types(&mut row, &mut |ty| -> CompilerResult<_> {
            let resolved = self.materialize_member_type(origin, ty, substitution, worklist)?;
            is_moved |= resolved != ty;

            Ok(resolved)
        })?;

        Ok(is_moved.then_some(row))
    }

    /// Materialize one member body type under the owner receiver.
    fn materialize_member_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // move receiver references and open computations alone in a non generic body
        let flags = self.type_flags(ty)?;
        if !flags.has_this() && !self.has_reachable_computation(ty)? {
            return Ok(ty);
        }

        // substitute the receiver, evaluating any computation the result reaches
        let substituted = self.substitute_type(ty, substitution)?;
        let resolved = match self.has_reachable_computation(substituted)? {
            true => self.evaluate_type(origin, substituted)?,
            false => substituted,
        };

        // admit the concrete applications the closed type reaches
        let source = match origin {
            Origin::Node(node, _) => node,
            _ => return Ok(resolved),
        };
        self.intern_applications(resolved, source, 0, worklist)?;

        Ok(resolved)
    }

    /// Return the concrete receiver type one instance binds `this` to.
    fn instance_receiver(
        &mut self,
        template: dir::GlobalSymbolId,
        origin: Origin,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // give a type template itself applied to the instance arguments
        if self.definition(template)?.is_some() {
            let arguments: Vec<_> = substitution
                .bindings
                .iter()
                .map(|binding| binding.argument)
                .collect();
            let receiver = if arguments.is_empty() {
                match self.template_symbol_type(template) {
                    Some(ty) => ty,
                    None => return Ok(None),
                }
            } else {
                let arguments = self.intern_type_ids(&arguments)?;
                self.intern_type(dir::Type::Application(dir::GenericApplication {
                    symbol: template,
                    arguments,
                }))?
            };

            return Ok(Some(self.evaluate_type(origin, receiver)?));
        }

        // give a member callable its owner's target
        let Some(owner) = self.template_member_owner(template) else {
            return Ok(None);
        };
        let target = match self.definition(owner)?.cloned() {
            Some(dir::Definition::Extension(extension)) => extension.target.r#type(),
            _ => match self.template_symbol_type(owner) {
                Some(ty) => ty,
                None => return Ok(None),
            },
        };
        let receiver = self.substitute_type(target, substitution)?;

        Ok(Some(self.evaluate_type(origin, receiver)?))
    }

    /// Return the definition owner declaring one member callable.
    fn template_member_owner(&self, template: dir::GlobalSymbolId) -> Option<dir::GlobalSymbolId> {
        // search owners only for member declarations
        let declaration = self
            .binding_table(template.module_id)
            .get_symbol(template.local_id)
            .declaration?;
        let member = declaration.local_id.try_into_typed::<dir::Member>().ok()?;
        let member_node = member.into_global_any(template.module_id);

        // search the own module's definitions
        if self.is_own_module(template.module_id) {
            return self
                .module
                .iter_definitions()
                .find(|(_, definition)| {
                    definition.method_declared_at(member_node) == Some(template)
                })
                .map(|(owner, _)| owner);
        }

        // otherwise search the loaded foreign module
        self.external_modules
            .get(&template.module_id)?
            .definitions
            .iter_definitions()
            .find(|(_, definition)| definition.method_declared_at(member_node) == Some(template))
            .map(|(owner, _)| owner)
    }

    /// Return one template's recorded instantiations, arguments as written.
    fn template_instantiations(
        &self,
        template: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::Instantiation>> {
        // read the instantiations this template owns
        let owner = Some(template);

        // read the committed instantiations of the own module
        if self.is_own_module(template.module_id) {
            let checked = self
                .module
                .checked
                .as_ref()
                .ok_or_else(|| CompilerError::Internal {
                    message: "materialize runs over a checked module".to_string(),
                })?;

            Ok(checked
                .generics
                .iter_instantiations()
                .filter(|instantiation| instantiation.owner == owner)
                .cloned()
                .collect())
        }
        // otherwise read the instantiations loaded from the foreign module
        else {
            let external = self
                .external_modules
                .get(&template.module_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("an unloaded template module {:?}", template.module_id),
                })?;

            Ok(external
                .generics
                .instantiations_of(owner)
                .cloned()
                .collect())
        }
    }

    /// Bind the materialized member types of one type template's definition.
    fn materialize_instance_definition(
        &mut self,
        instance: dir::LocalInstanceId,
        origin: Origin,
        definition: dir::Definition,
        substitution: &TypeSubstitution,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // materialize the member symbol types the lower stage reads for layouts and slots
        for member in definition.members() {
            let symbol = match member {
                dir::DefinitionMember::Field(field) => Some(field.symbol),
                dir::DefinitionMember::Method(method) => Some(method.symbol),
                _ => None,
            };
            if let Some(symbol) = symbol
                && let Some(ty) = self.template_symbol_type(symbol)
            {
                let resolved = self.materialize_instance_type(
                    instance,
                    origin,
                    ty,
                    substitution,
                    depth,
                    worklist,
                )?;
                self.module
                    .generics_tail
                    .bind_instance_symbol(instance, symbol, resolved);
            }
        }

        // fold every type the definition carries
        let mut folded = definition;
        dir::TypeFold::map_types(&mut folded, &mut |ty| {
            self.materialize_instance_type(instance, origin, ty, substitution, depth, worklist)
        })?;

        Ok(())
    }

    /// Bind the materialized types of one callable template's body.
    fn materialize_instance_body(
        &mut self,
        instance: dir::LocalInstanceId,
        origin: Origin,
        template: dir::GlobalSymbolId,
        substitution: &TypeSubstitution,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // materialize the template's own declared type
        if let Some(ty) = self.template_symbol_type(template) {
            let resolved = self.materialize_instance_type(
                instance,
                origin,
                ty,
                substitution,
                depth,
                worklist,
            )?;
            self.module
                .generics_tail
                .bind_instance_symbol(instance, template, resolved);
        }

        // stop at a bodiless template
        let Some(nodes) = self.template_body_nodes(template)? else {
            return Ok(());
        };

        // fold each body node's committed type and payloads under the substitution
        for node in nodes {
            if let Some(ty) = self.committed_template_type(template.module_id, node) {
                self.materialize_instance_type(
                    instance,
                    origin,
                    ty,
                    substitution,
                    depth,
                    worklist,
                )?;
            }

            self.materialize_instance_payloads(
                instance,
                origin,
                template.module_id,
                node,
                substitution,
                depth,
                worklist,
            )?;
        }

        Ok(())
    }

    /// Materialize the types carried by one template node's committed payloads.
    fn materialize_instance_payloads(
        &mut self,
        instance: dir::LocalInstanceId,
        origin: Origin,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
        substitution: &TypeSubstitution,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // fold the committed decision, which records the materialized types alone
        let decision = self.template_decision(module, node).cloned();
        if let Some(mut decision) = decision {
            dir::TypeFold::map_types(&mut decision, &mut |ty| {
                self.materialize_instance_type(instance, origin, ty, substitution, depth, worklist)
            })?;
        }

        // fold the committed place resolution
        let place = self.template_place(module, node).cloned();
        if let Some(mut place) = place {
            dir::TypeFold::map_types(&mut place, &mut |ty| {
                self.materialize_instance_type(instance, origin, ty, substitution, depth, worklist)
            })?;
        }

        // fold the committed coercion
        let coercion = self.template_coercion(module, node).cloned();
        if let Some(mut coercion) = coercion {
            dir::TypeFold::map_types(&mut coercion, &mut |ty| {
                self.materialize_instance_type(instance, origin, ty, substitution, depth, worklist)
            })?;
        }

        Ok(())
    }

    /// Return the self application type of one interface member's owner.
    fn interface_owner_self(
        &mut self,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // serve the memo
        if let Some(base) = self.interface_owners.get(&member) {
            return Ok(*base);
        }

        // find the interface declaring this member
        let base = self
            .interface_member_owner(member)
            .and_then(|owner| self.template_symbol_type(owner));
        self.interface_owners.insert(member, base);

        Ok(base)
    }

    /// Return the interface declaring one interface member symbol.
    fn interface_member_owner(&self, member: dir::GlobalSymbolId) -> Option<dir::GlobalSymbolId> {
        // match interfaces holding this member's method slot
        let declares = |definition: &dir::Definition| match definition {
            dir::Definition::Interface(interface) => interface.members.iter().any(|declared| {
                matches!(
                    declared,
                    dir::DefinitionMember::Method(method) if method.symbol == member
                )
            }),
            _ => false,
        };

        // search the own module's definitions
        if self.is_own_module(member.module_id) {
            return self
                .module
                .iter_definitions()
                .find(|(_, definition)| declares(definition))
                .map(|(owner, _)| owner);
        }

        // otherwise search the loaded foreign module
        self.external_modules
            .get(&member.module_id)?
            .definitions
            .iter_definitions()
            .find(|(_, definition)| declares(definition))
            .map(|(owner, _)| owner)
    }

    /// Substitute one template type, committing the materialized type it becomes.
    fn materialize_instance_type(
        &mut self,
        instance: dir::LocalInstanceId,
        origin: Origin,
        ty: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // interface owners spell the receiver as their self application
        let owner_self = match substitution.receiver {
            Some(_) => {
                let member = self
                    .module
                    .generics_tail
                    .get_local_instance(instance)
                    .map(|interned| interned.key.symbol);
                match member {
                    Some(member) => self.interface_owner_self(member)?,
                    None => None,
                }
            }
            None => None,
        };

        // closed types are identical across instances and stay as written
        let flags = self.type_flags(ty)?;
        if !flags.has_parameter() && !flags.has_this() && owner_self.is_none() {
            return Ok(ty);
        }

        // commit the materialized type wherever the instance moves the written one
        let substituted = self.substitute_type(ty, substitution)?;

        // close the self application at the instance receiver
        let substituted = match (substitution.receiver, owner_self) {
            (Some(receiver), Some(base)) => {
                self.replace_type(self.module_id, substituted, base, receiver)?
            }
            _ => substituted,
        };

        // evaluate any computation the substituted type reaches
        let resolved = match self.has_reachable_computation(substituted)? {
            true => self.evaluate_type(origin, substituted)?,
            false => substituted,
        };
        if resolved != ty {
            let is_evaluated = resolved != substituted;
            self.module
                .generics_tail
                .bind_instance_type(instance, ty, resolved, is_evaluated);
        }

        // admit the concrete applications the closed type reaches
        let source = match origin {
            Origin::Node(node, _) => node,
            _ => return Ok(resolved),
        };
        self.intern_applications(resolved, source, depth + 1, worklist)?;

        Ok(resolved)
    }

    /// Return every node inside one callable template's body, when it has one.
    fn template_body_nodes(
        &mut self,
        template: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<Vec<dir::GlobalNodeIdAny>>> {
        // find the template's declaration in its defining module
        let declaration = self
            .binding_table(template.module_id)
            .get_symbol(template.local_id)
            .declaration;
        let Some(declaration) = declaration else {
            return Ok(None);
        };
        let tree = self.template_tree(template.module_id)?;

        // read the body expression behind a plain or member function declaration
        let body = if let Ok(member) = declaration.local_id.try_into_typed::<dir::Member>() {
            match tree.get(member) {
                dir::Member::Method { body, .. } => *body,
                _ => None,
            }
        } else if let Ok(declaration) = declaration.local_id.try_into_typed::<dir::Declaration>() {
            match tree.get(declaration) {
                dir::Declaration::Function(function) => function.body,
                _ => None,
            }
        } else {
            None
        };
        let Some(body) = body else {
            return Ok(None);
        };

        // collect every node in the body subtree
        let mut collector = BodyNodeCollector {
            module: template.module_id,
            nodes: Vec::new(),
        };
        dir::NodeVisitor::visit_expression(&mut collector, tree, body, tree.get(body));

        Ok(Some(collector.nodes))
    }

    /// Return one template module's parsed tree.
    fn template_tree(&self, module: ModuleId) -> CompilerResult<&dir::Tree> {
        // read the own module's tree
        if self.is_own_module(module) {
            return Ok(&self.module.parsed.tree);
        }

        // otherwise read the loaded foreign module
        self.external_modules
            .get(&module)
            .map(|external| &external.parsed.tree)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("an unloaded template module {module:?}"),
            })
    }

    /// Return one template body node's committed type.
    fn committed_template_type(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        // read the own module's node types
        if self.is_own_module(module) {
            return self.module.types.get_node_type_id(node);
        }

        // otherwise read the loaded foreign module
        self.external_modules
            .get(&module)?
            .types
            .get_node_type_id(node)
    }

    /// Return one template symbol's committed type.
    fn template_symbol_type(&self, symbol: dir::GlobalSymbolId) -> Option<dir::GlobalTypeId> {
        // read the own module's symbol types
        if self.is_own_module(symbol.module_id) {
            return self.module.types.get_symbol_type_id(symbol);
        }

        // otherwise read the loaded foreign module
        self.external_modules
            .get(&symbol.module_id)?
            .types
            .get_symbol_type_id(symbol)
    }

    /// Return one template body node's committed decision.
    fn template_decision(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> Option<&dir::Decision> {
        // read the own module's decisions
        if self.is_own_module(module) {
            return self.module.checked.as_ref()?.decisions.decision(node);
        }

        // otherwise read the loaded foreign module
        self.external_modules.get(&module)?.decisions.decision(node)
    }

    /// Return one template body node's committed place resolution.
    fn template_place(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> Option<&dir::PlaceResolution> {
        // read the own module's place resolutions
        if self.is_own_module(module) {
            return self
                .module
                .checked
                .as_ref()?
                .decisions
                .place_resolution(node);
        }

        // otherwise read the loaded foreign module
        self.external_modules
            .get(&module)?
            .decisions
            .place_resolution(node)
    }

    /// Return one template body node's committed coercion.
    fn template_coercion(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> Option<&dir::Coercion> {
        // read the own module's coercions
        if self.is_own_module(module) {
            return self.module.checked.as_ref()?.coercions.coercion(node);
        }

        // otherwise read the loaded foreign module
        self.external_modules.get(&module)?.coercions.coercion(node)
    }
}

/// Visitor collecting every node in one body subtree.
struct BodyNodeCollector {
    /// The visited module.
    module: ModuleId,
    /// Every collected node, of any kind.
    nodes: Vec<dir::GlobalNodeIdAny>,
}

impl dir::NodeVisitor for BodyNodeCollector {
    fn visit_any(&mut self, _tree: &dir::Tree, ty: dir::NodeType, id: u32) {
        self.nodes.push(dir::GlobalNodeIdAny {
            module_id: self.module,
            local_id: dir::LocalNodeIdAny::new(id, ty),
        });
    }
}
