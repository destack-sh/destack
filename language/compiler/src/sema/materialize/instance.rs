use rustc_hash::FxHashSet;
use tspp_core::{FxIndexMap, FxIndexSet};
use tspp_dir as dir;

use crate::sema::{CheckState, Origin, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

/// The worklist one materialize pass drains to its fixpoint.
#[derive(Default)]
pub(in crate::sema) struct InstanceWorklist {
    /// The interned instances by structural identity.
    seen: FxIndexMap<dir::InstanceKey, dir::LocalInstanceId>,
    /// The instantiation depth of each interned instance, in allocation order.
    depths: Vec<u32>,
    /// The position of the instance whose walk allocates the next instances.
    pub(super) reaching: Option<usize>,
    /// Whether a chain already exceeded the depth limit, reported once.
    overflowed: bool,
    /// The types whose graphs this pass walked.
    pub(super) walked: FxHashSet<dir::GlobalTypeId>,
}

impl InstanceWorklist {
    /// Return the instance allocated at one position, in allocation order.
    pub(super) fn instance_at(
        &self,
        index: usize,
    ) -> Option<(dir::InstanceKey, dir::LocalInstanceId)> {
        self.seen
            .get_index(index)
            .map(|(key, instance)| (key.clone(), *instance))
    }
}

impl CheckState<'_> {
    /// Materialize the instances this module's instantiations reach, to a fixpoint.
    pub(in crate::sema) fn materialize_instances(
        &mut self,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // record every instantiation checking committed, open ones over their enclosing template
        let committed = self
            .committed(self.module_id)?
            .ok_or_else(|| CompilerError::Internal {
                message: "materialize runs over a checked module".to_string(),
            })?;
        let instantiations: Vec<_> = committed.generics.iter_instantiations().cloned().collect();
        for instantiation in instantiations {
            self.intern_instance(
                instantiation.key.symbol,
                instantiation.key.receiver,
                instantiation.key.arguments,
                instantiation.source,
                dir::InstanceOrigin::Instantiation,
                worklist,
            )?;
        }

        Ok(())
    }

    /// Return whether one type mentions a parameter outside the templates enclosing one node.
    fn has_parameter_outside(
        &mut self,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<bool> {
        // read the templates whose parameters the node keeps rigid
        let mut rigid = Vec::new();
        let mut template = self.template_at_node(source)?;
        while let Some(id) = template {
            rigid.push(id);
            template = self.parent_generic_template(id)?;
        }

        // walk the type graph, stopping at the first parameter outside them
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }

            // answer at the first parameter outside the rigid set, and at every erased type
            let kind = self.ty(id)?;
            match &kind {
                dir::Type::Parameter(parameter) if !self.is_lifetime_parameter(*parameter)? => {
                    let template = self
                        .generic_parameter(*parameter)?
                        .map(|binding| binding.template.into_global(parameter.module_id));
                    if !template.is_some_and(|template| rigid.contains(&template)) {
                        return Ok(true);
                    }
                }
                dir::Type::Erased(_) => return Ok(true),
                _ => {}
            }

            self.for_each_type_child(id.module_id, &kind, |child| pending.push(child))?;
        }

        Ok(false)
    }

    /// Return whether one instance binds every type parameter to itself at the owner's self type.
    fn is_identity_instance(
        &mut self,
        template: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        arguments: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<bool> {
        for binding in arguments {
            if !self.binds_itself(binding)? {
                return Ok(false);
            }
        }
        let Some(receiver) = receiver else {
            return Ok(true);
        };
        let Some(owner) = self.member_owner(template)? else {
            return Ok(false);
        };
        let dir::Type::Application(application) = self.ty(receiver)? else {
            return Ok(false);
        };
        if application.symbol != owner {
            return Ok(false);
        }
        let substitution = self.instance_substitution(receiver.module_id, &application)?;
        for binding in &substitution.bindings {
            if !self.binds_itself(binding)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether one binding maps its parameter to that parameter itself.
    fn binds_itself(&self, binding: &dir::GenericArgumentBinding) -> CompilerResult<bool> {
        Ok(matches!(
            self.ty(binding.argument)?,
            dir::Type::Parameter(parameter) if parameter == binding.parameter
        ))
    }

    /// Return whether one symbol names a transparent alias, a rename lowering reads through.
    pub(super) fn is_transparent_alias(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let definition = self.definition(symbol)?;
        let Some(dir::Definition::TypeAlias(alias)) = definition.as_deref() else {
            return Ok(false);
        };
        let value = alias.value;

        Ok(!self.ty(value)?.is_structural())
    }

    /// Close one instance argument at its source, none when it stays open.
    fn close_argument(
        &mut self,
        origin: Origin,
        argument: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        next_position: &mut u32,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let argument = self.deeply_resolve(origin, argument)?;
        let argument = self.erase_argument_regions(argument, &mut Vec::new(), next_position)?;
        let flags = self.type_flags(argument)?;
        if flags.has_variable() || flags.has_infer() {
            return Ok(None);
        }
        if flags.has_this() && !self.is_within_interface(source)? {
            return Ok(None);
        }
        if flags.has_parameter() && self.has_parameter_outside(argument, source)? {
            return Ok(None);
        }

        Ok(Some(argument))
    }

    /// Intern one instance, open over the parameters of the templates enclosing its source.
    pub(super) fn intern_instance(
        &mut self,
        template: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        bindings: Vec<dir::GenericArgumentBinding>,
        source: dir::GlobalNodeIdAny,
        introduced: dir::InstanceOrigin,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<Option<dir::LocalInstanceId>> {
        // close the arguments and the receiver at the source, binding lifetimes by position
        let origin = self.anchored_origin(source)?;
        let mut arguments = Vec::new();
        let mut next_position = 0;
        for binding in bindings {
            let Some(argument) =
                self.close_argument(origin, binding.argument, source, &mut next_position)?
            else {
                return Ok(None);
            };
            arguments.push(dir::GenericArgumentBinding::new(
                binding.parameter,
                argument,
            ));
        }
        let receiver = match receiver {
            Some(receiver) => {
                match self.close_argument(origin, receiver, source, &mut next_position)? {
                    Some(receiver) => Some(receiver),
                    None => return Ok(None),
                }
            }
            None => None,
        };

        // drop every selection leaving a parameter unbound
        if let Some(template_id) = self.symbol_template(template)?
            && let Some(declared) = self.generic_template(template_id)?
        {
            let parameters = declared.parameters.clone();
            for parameter in parameters.iter().copied() {
                let parameter = parameter.into_global(template_id.module_id);
                if arguments
                    .iter()
                    .any(|binding| binding.parameter == parameter)
                {
                    continue;
                }

                // the receiver binds through the key, dependents close below
                let is_implicit = self
                    .generic_parameter(parameter)?
                    .is_some_and(|binding| binding.origin == dir::GenericParameterOrigin::Receiver);
                if is_implicit {
                    continue;
                }

                // an instantiation binding the owner's parameters alone names no instance
                return Ok(None);
            }
        }

        // require a receiver or one argument to close on
        if arguments.is_empty() && receiver.is_none() {
            return Ok(None);
        }

        // answer an interface requirement through the receiver's witness
        if let Some(owner) = self.interface_member_owner(template)? {
            let Some(receiver) = receiver else {
                return Ok(None);
            };

            // an open receiver keeps the requirement instance, its enclosing instance closes it
            let flags = self.type_flags(receiver)?;
            if flags.has_parameter() || flags.has_this() {
                let key = dir::InstanceKey::new(template, arguments).with_receiver(Some(receiver));

                return self.allocate_instance(key, source, introduced, worklist);
            }

            // a closed receiver answers through its witness
            let interface = self.requirement_owner_application(owner, &arguments)?;
            self.write_witness(receiver, interface, source, worklist)?;
            let answer = self
                .module
                .generics_tail
                .witness(receiver, interface)
                .and_then(|witness| {
                    witness
                        .functions
                        .iter()
                        .find(|function| function.member == template)
                        .map(|function| function.function.clone())
                });

            // answer through the witness function the conformance names
            if let Some(key) = answer {
                return Ok(worklist.seen.get(&key).copied());
            }

            // an intrinsic witness answers a requirement without a body
            if self.template_body_expression(template)?.is_none() {
                return Ok(None);
            }
        }

        // instantiate the requirement's default body at the receiver
        let key = dir::InstanceKey::new(template, arguments).with_receiver(receiver);

        self.allocate_instance(key, source, introduced, worklist)
    }

    /// Bind the dependents of one key's declaration and its owners, evaluated at the key.
    fn bind_dependents(
        &mut self,
        mut key: dir::InstanceKey,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::InstanceKey> {
        let origin = self.anchored_origin(source)?;
        let substitution = TypeSubstitution {
            bindings: key.arguments.iter().copied().collect(),
            receiver: key.receiver,
        };

        // evaluate each owner's dependents outermost first, then the declaration's own
        let mut symbols = vec![key.symbol];
        let mut owner = self.member_owner(key.symbol)?;
        while let Some(symbol) = owner {
            symbols.push(symbol);
            owner = self.member_owner(symbol)?;
        }
        symbols.reverse();
        for symbol in symbols {
            for dependent in self.symbol_dependents(symbol)? {
                key.dependents
                    .push(self.close_dependent(origin, dependent, &substitution)?);
            }
        }

        Ok(key)
    }

    /// Return the interface application one requirement instance binds its owner's parameters at.
    fn requirement_owner_application(
        &mut self,
        owner: dir::GlobalSymbolId,
        arguments: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut owner_arguments = Vec::new();
        if let Some(template) = self.symbol_template(owner)? {
            for parameter in self.generic_template_parameters(template)? {
                let Some(binding) = arguments
                    .iter()
                    .find(|binding| binding.parameter == parameter)
                else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "a requirement of '{}' without its interface arguments",
                            self.format_symbol(owner)
                        ),
                    });
                };
                owner_arguments.push(binding.argument);
            }
        }
        let arguments = self.intern_type_ids(&owner_arguments)?;

        self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol: owner,
            arguments,
        }))
    }

    /// Allocate one instance per distinct key, recording the witnesses its bounds reach.
    pub(super) fn allocate_instance(
        &mut self,
        key: dir::InstanceKey,
        source: dir::GlobalNodeIdAny,
        introduced: dir::InstanceOrigin,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<Option<dir::LocalInstanceId>> {
        // an instance at the template's own parameters is the template itself
        if self.is_identity_instance(key.symbol, key.receiver, &key.arguments)? {
            return Ok(None);
        }
        // bind each dependent of the declaration at the arguments, recording what the key reaches
        let key = self.bind_dependents(key, source)?;
        for ty in key
            .receiver
            .into_iter()
            .chain(key.arguments.iter().map(|binding| binding.argument))
            .chain(key.dependents.iter().copied())
        {
            self.walk_type_graph(ty, source, worklist)?;
        }

        // re-close an instance the worklist already admitted
        if let Some(admitted) = worklist.seen.get(&key).copied() {
            // re-close the instance a type application introduced under an instantiation
            if introduced == dir::InstanceOrigin::Instantiation {
                self.module
                    .generics_tail
                    .set_instance_origin(admitted, introduced);
            }

            return Ok(Some(admitted));
        }

        // stop a chain past the depth limit, a polymorphic recursion, reporting it once
        let depth = worklist
            .reaching
            .map_or(0, |parent| worklist.depths[parent] + 1);
        if depth > super::closure::INSTANCE_DEPTH_LIMIT {
            if !worklist.overflowed
                && let Some(parent) = worklist
                    .reaching
                    .and_then(|parent| worklist.instance_at(parent))
            {
                worklist.overflowed = true;
                self.report_instantiation_depth_exceeded(source, parent.0.symbol)?;
            }
            return Ok(None);
        }
        let origin = match worklist.reaching {
            Some(_) => dir::InstanceOrigin::Reached,
            None => introduced,
        };
        let instance = self.module.generics_tail.push_instance(dir::Instance {
            key: key.clone(),
            source,
            origin,
        });
        worklist.seen.insert(key.clone(), instance);
        worklist.depths.push(depth);

        // record the witnesses the arguments' bounds reach for a declaration with a body
        if self.symbol_kind(key.symbol)? != dir::SymbolKind::TypeAlias {
            self.walk_bound_witnesses(&key, source, worklist)?;
        }

        // record the Drop witness destructors read on a conforming nominal
        if key.receiver.is_none() && self.drop_hook_member(key.symbol)?.is_some() {
            let application = self.instance_application(&key)?;
            let drop = self.language_type(dir::LanguageItem::Drop, &[])?;
            self.write_witness(application, drop, source, worklist)?;
        }

        Ok(Some(instance))
    }
}
