use std::collections::VecDeque;

use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{CheckState, Origin, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

/// Cap on instantiation chain depth, guarding polymorphic recursion.
const INSTANCE_DEPTH_LIMIT: u32 = 128;

/// One instance identity: the template and its closed arguments.
type InstanceIdentity = (dir::GlobalSymbolId, Vec<dir::GenericArgumentBinding>);

/// The worklist one materialize pass drains to its fixpoint.
#[derive(Default)]
pub(in crate::sema) struct InstanceWorklist {
    /// The interned instances by structural identity.
    seen: FxIndexMap<InstanceIdentity, dir::LocalInstanceId>,
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

        // seed from the module-level instantiations check recorded
        for instantiation in checked.generics.iter_instantiations() {
            if instantiation.owner.is_none() {
                let instantiation = instantiation.clone();
                self.intern_instance(
                    instantiation.selection.symbol,
                    instantiation.selection.arguments,
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
    pub(in crate::sema) fn type_reaches_computation(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
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
                    if self.alias_computes(application.symbol)? =>
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
            match &ty {
                dir::Type::Application(application) => {
                    let application = *application;
                    self.intern_application(id.module_id, &application, source, depth, worklist)?;
                }
                // arrays run on the Array representation class
                dir::Type::Array(array) => {
                    let application = dir::GenericApplication {
                        symbol: self.language_symbol(dir::LanguageItem::Array)?,
                        arguments: self.intern_type_ids(&[array.element])?,
                    };
                    self.intern_application(self.module_id, &application, source, depth, worklist)?;
                }
                _ => {}
            }

            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(())
    }

    /// Intern one written application as an instance through its canonical pairing.
    fn intern_application(
        &mut self,
        module: ModuleId,
        application: &dir::GenericApplication,
        source: dir::GlobalNodeIdAny,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // spellings that pair no instance stay open
        let Ok(substitution) = self.instance_substitution(module, application) else {
            return Ok(());
        };

        self.intern_instance(
            application.symbol,
            substitution.bindings.to_vec(),
            source,
            dir::InstanceOrigin::Application,
            depth,
            worklist,
        )
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

        // lifetime spellings all erase to one canonical literal
        if self.written_argument_is_lifetime(id)? {
            return self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Lifetime(
                dir::Lifetime::Frame,
            )));
        }

        visiting.push(id);
        let ty = self.ty(id)?;
        let rebuilt =
            self.map_type_children(id.module_id, self.module_id, ty, &mut |state, child| {
                state.erase_argument_lifetimes(child, visiting)
            })?;
        visiting.pop();

        self.intern_type(rebuilt)
    }

    /// Return whether one type mentions a non-lifetime parameter.
    fn type_has_open_parameter(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
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
        bindings: Vec<dir::GenericArgumentBinding>,
        source: dir::GlobalNodeIdAny,
        introduced: dir::InstanceOrigin,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // reject runaway polymorphic recursion at the depth rustc allows
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

            // leave open instantiations to close under their enclosing instance
            let flags = self.type_flags(argument)?;
            if flags.has_variable() || flags.has_this() {
                return Ok(());
            }
            if flags.has_parameter() && self.type_has_open_parameter(argument)? {
                return Ok(());
            }

            arguments.push(dir::GenericArgumentBinding::new(
                binding.parameter,
                argument,
            ));
        }

        // a selection without type arguments closes nothing
        if arguments.is_empty() {
            return Ok(());
        }

        // partial selections stay open: an instance binds every declared parameter
        if let Some(template_id) = self.symbol_template(template)?
            && let Some(declared) = self.generic_template(template_id)
        {
            let parameters = declared.parameters.clone();
            for parameter in parameters {
                let parameter = parameter.into_global(template_id.module_id);
                if self.is_lifetime_parameter(parameter) {
                    continue;
                }
                if !arguments
                    .iter()
                    .any(|binding| binding.parameter == parameter)
                {
                    return Ok(());
                }
            }
        }

        // allocate one instance per distinct (template, arguments) pair
        let key = (template, arguments.clone());
        if let Some(admitted) = worklist.seen.get(&key).copied() {
            // upgrade the row a type application introduced, re-closing it under the instantiation
            if introduced == dir::InstanceOrigin::Instantiation
                && let Some(row) = self.module.generics_tail.get_local_instance(admitted)
                && row.origin == dir::InstanceOrigin::Application
            {
                self.module
                    .generics_tail
                    .set_instance_origin(admitted, introduced);
                worklist.queue.push_back((admitted, depth));
            }

            return Ok(());
        }

        let instance = self.module.generics_tail.push_instance(dir::Instance {
            selection: dir::Selection::new(template, arguments),
            source,
            origin: introduced,
        });
        worklist.seen.insert(key, instance);
        worklist.queue.push_back((instance, depth));

        Ok(())
    }

    /// Close one instance: admit its template's instantiations and ground its rows.
    fn materialize_instance(
        &mut self,
        instance: dir::LocalInstanceId,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // read the instance and the substitution its arguments select
        let row = self
            .module
            .generics_tail
            .get_local_instance(instance)
            .ok_or_else(|| CompilerError::Internal {
                message: "closed an unallocated instance".to_string(),
            })?
            .clone();
        let origin = Origin::Node(row.source, None);
        let mut substitution = TypeSubstitution {
            bindings: row.selection.arguments.iter().copied().collect(),
            receiver: None,
        };
        substitution.receiver =
            self.instance_receiver(row.selection.symbol, origin, &substitution)?;

        // substitute the template's recorded instantiations under this instance
        let mut reached = Vec::new();
        for instantiation in self.template_instantiations(row.selection.symbol)? {
            let mut arguments = Vec::new();
            for binding in instantiation.selection.arguments {
                arguments.push(dir::GenericArgumentBinding::new(
                    binding.parameter,
                    self.substitute_type(binding.argument, &substitution)?,
                ));
            }
            reached.push((instantiation.selection.symbol, arguments));
        }

        // intern the ones that closed, one chain step deeper
        for (template, arguments) in reached {
            self.intern_instance(
                template,
                arguments,
                row.source,
                row.origin,
                depth + 1,
                worklist,
            )?;
        }

        // materialize the template under the substitution for this instance
        match self.definition(row.selection.symbol)?.cloned() {
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
                row.selection.symbol,
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
        // the receiver `this` denotes inside the body
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
            let origin = Origin::Node(node, None);
            if let Some(ty) = self.committed_template_type(member.module_id, node) {
                let resolved = self.materialize_member_type(origin, ty, &substitution, worklist)?;
                if resolved != ty {
                    self.module.types_tail.set_node_type(node, resolved);
                }
            }

            let decision = self.template_decision(member.module_id, node).cloned();
            if let Some(decision) =
                self.materialize_member_payload(origin, decision, &substitution, worklist)?
            {
                self.module.decisions.set_decision(node, decision);
            }

            let place = self.template_place(member.module_id, node).cloned();
            if let Some(place) =
                self.materialize_member_payload(origin, place, &substitution, worklist)?
            {
                self.module.decisions.set_place_resolution(node, place);
            }

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
        let Some(mut row) = row else {
            return Ok(None);
        };

        // rewrite each type the row carries, keeping the row where one moves
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
        // only receiver references move in a non-generic body
        let flags = self.type_flags(ty)?;
        if !flags.has_this() {
            return Ok(ty);
        }

        let substituted = self.substitute_type(ty, substitution)?;
        let resolved = match self.type_reaches_computation(substituted)? {
            true => self.evaluate_type(origin, substituted)?,
            false => substituted,
        };

        // admit the concrete applications the grounded type reaches
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
        // a type template receives itself applied to the instance arguments
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

        // a member callable receives its owner's target
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

        if self.is_own_module(template.module_id) {
            return self
                .module
                .iter_definitions()
                .find(|(_, definition)| {
                    definition.method_declared_at(member_node) == Some(template)
                })
                .map(|(owner, _)| owner);
        }

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
        // read the own committed rows or the loaded foreign rows
        let owner = Some(template);
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
        } else {
            let external = self
                .external_modules
                .get(&template.module_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("template module {:?} was not loaded", template.module_id),
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
        // materialize the member symbol types lower reads for layouts and slots
        for member in definition.members() {
            let symbol = match member {
                dir::DefinitionMember::Field(field) => Some(field.symbol),
                dir::DefinitionMember::Method(method) => Some(method.symbol),
                _ => None,
            };
            if let Some(symbol) = symbol
                && let Some(ty) = self.template_symbol_type(symbol)
            {
                self.materialize_instance_type(
                    instance,
                    origin,
                    ty,
                    substitution,
                    depth,
                    worklist,
                )?;
            }
        }

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
            self.materialize_instance_type(instance, origin, ty, substitution, depth, worklist)?;
        }

        // bodiless templates contribute no rows beyond their own
        let Some(nodes) = self.template_body_nodes(template)? else {
            return Ok(());
        };

        // fold each body node's committed type and rows under the substitution
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
        // fold for the materialized types alone; instance reads keep the written payloads
        let decision = self.template_decision(module, node).cloned();
        if let Some(mut decision) = decision {
            dir::TypeFold::map_types(&mut decision, &mut |ty| {
                self.materialize_instance_type(instance, origin, ty, substitution, depth, worklist)
            })?;
        }

        let place = self.template_place(module, node).cloned();
        if let Some(mut place) = place {
            dir::TypeFold::map_types(&mut place, &mut |ty| {
                self.materialize_instance_type(instance, origin, ty, substitution, depth, worklist)
            })?;
        }

        let coercion = self.template_coercion(module, node).cloned();
        if let Some(mut coercion) = coercion {
            dir::TypeFold::map_types(&mut coercion, &mut |ty| {
                self.materialize_instance_type(instance, origin, ty, substitution, depth, worklist)
            })?;
        }

        Ok(())
    }

    /// Substitute one template type, recording the materialized type it becomes.
    fn materialize_instance_type(
        &mut self,
        instance: dir::LocalInstanceId,
        origin: Origin,
        ty: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
        depth: u32,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // closed types are identical across instances and stay as written
        let flags = self.type_flags(ty)?;
        if !flags.has_parameter() && !flags.has_this() {
            return Ok(ty);
        }

        // record the materialized type wherever the instance moves the written one
        let substituted = self.substitute_type(ty, substitution)?;
        let resolved = match self.type_reaches_computation(substituted)? {
            true => self.evaluate_type(origin, substituted)?,
            false => substituted,
        };
        if resolved != ty {
            let is_evaluated = resolved != substituted;
            self.module
                .generics_tail
                .bind_instance_type(instance, ty, resolved, is_evaluated);
        }

        // admit the concrete applications the grounded type reaches
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
            options: dir::NodeVisitorOptions::default(),
            module: template.module_id,
            nodes: Vec::new(),
        };
        dir::NodeVisitor::visit_expression(&mut collector, tree, body, tree.get(body));

        Ok(Some(collector.nodes))
    }

    /// Return one template module's parsed tree.
    fn template_tree(&self, module: ModuleId) -> CompilerResult<&dir::Tree> {
        if self.is_own_module(module) {
            return Ok(&self.module.parsed.tree);
        }

        self.external_modules
            .get(&module)
            .map(|external| &external.parsed.tree)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("template module {module:?} was not loaded"),
            })
    }

    /// Return one template body node's committed type.
    fn committed_template_type(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        if self.is_own_module(module) {
            return self.module.types.get_node_type_id(node);
        }

        self.external_modules
            .get(&module)?
            .types
            .get_node_type_id(node)
    }

    /// Return one template symbol's committed type.
    fn template_symbol_type(&self, symbol: dir::GlobalSymbolId) -> Option<dir::GlobalTypeId> {
        if self.is_own_module(symbol.module_id) {
            return self.module.types.get_symbol_type_id(symbol);
        }

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
        if self.is_own_module(module) {
            return self.module.checked.as_ref()?.decisions.decision(node);
        }

        self.external_modules.get(&module)?.decisions.decision(node)
    }

    /// Return one template body node's committed place resolution.
    fn template_place(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> Option<&dir::PlaceResolution> {
        if self.is_own_module(module) {
            return self
                .module
                .checked
                .as_ref()?
                .decisions
                .place_resolution(node);
        }

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
        if self.is_own_module(module) {
            return self.module.checked.as_ref()?.coercions.coercion(node);
        }

        self.external_modules.get(&module)?.coercions.coercion(node)
    }
}

/// Visitor collecting every node in one body subtree.
struct BodyNodeCollector {
    /// The visitor options.
    options: dir::NodeVisitorOptions,
    /// The visited module.
    module: ModuleId,
    /// Every collected node, of any kind.
    nodes: Vec<dir::GlobalNodeIdAny>,
}

impl dir::NodeVisitor for BodyNodeCollector {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_any(&mut self, _tree: &dir::Tree, ty: dir::NodeType, id: u32) {
        self.nodes.push(dir::GlobalNodeIdAny {
            module_id: self.module,
            local_id: dir::LocalNodeIdAny::new(id, ty),
        });
    }
}
