use destack_artifact::DiagnosticLike;
use destack_dir as dir;
use destack_mir as mir;
use destack_source::{ModuleId, ProvenanceId};

use crate::lower::{
    CallableImplementation, FunctionDefinition, LifetimeParameters, ModuleLowerer, Reachable,
    insert_reference_type,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// The declaration outcome behind one callable instance key.
pub(in crate::lower) enum FunctionDeclaration {
    /// The declared MIR function.
    Declared(mir::FunctionId),
    /// The declaration failed with a reported diagnostic.
    Failed,
}

/// One concrete declaration instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::lower) struct GenericInstanceKey {
    /// The instantiated declaration.
    pub(in crate::lower) symbol: dir::GlobalSymbolId,
    /// The concrete receiver closing an interface member's `this`.
    pub(in crate::lower) receiver: Option<mir::StaticId>,
    /// The concrete generic arguments.
    pub(in crate::lower) arguments: Vec<mir::StaticId>,
}

impl GenericInstanceKey {
    /// Create the instance key of one non-generic declaration.
    pub(in crate::lower) fn non_generic(symbol: dir::GlobalSymbolId) -> Self {
        Self {
            symbol,
            receiver: None,
            arguments: Vec::new(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl ModuleLowerer<'_> {
    /// Declare every concrete generic instance reachable from the bodies.
    ///
    /// A body whose collection or instance declaration fails keeps its diagnostic and the
    /// remaining bodies keep their collected reachable set.
    pub(in crate::lower) fn declare_reachable_instances(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        bodies: &[FunctionDefinition],
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<(Vec<FunctionDefinition>, Reachable)> {
        // seed the reachable set
        let mut reachable = Reachable::default();

        // require this module's ambient drop hooks, which destructors reach
        //  type-generic hooks close through their owner's instances
        let hooks: Vec<_> = self
            .drop_hooks
            .iter()
            .map(|(owner, member)| (*owner, *member))
            .collect();
        for (owner, member) in hooks {
            if member.module_id == self.module
                && !self.owner_has_instance_parameters(owner)?
                && !self.signature_has_parameters_beyond_memory(self.symbol_type(member)?)?
            {
                reachable
                    .instances
                    .push(dir::InstanceKey::new(member, Vec::new()));
            }
        }

        // collect every closed instantiation sema already materialized
        let closed: Vec<_> = self
            .state(self.module)?
            .generics
            .iter_instances()
            .filter(|(_, instance)| instance.origin == dir::InstanceOrigin::Instantiation)
            .map(|(_, instance)| {
                (
                    instance.key.symbol,
                    instance.key.receiver,
                    instance.key.arguments.clone(),
                )
            })
            .collect();
        // key instances like call sites: regions erase from the selection
        let closed: Vec<_> = closed
            .into_iter()
            .map(|(symbol, receiver, arguments)| {
                Ok((symbol, receiver, self.instance_bindings(&arguments, None)?))
            })
            .collect::<CompilerResult<_>>()?;
        // keep the callable instances; nominal ones declare through their representations
        for (template, receiver, arguments) in closed {
            let Some(ty) = self.types(template.module_id)?.get_symbol_type_id(template) else {
                continue;
            };
            if !matches!(
                self.ty(ty)?,
                dir::Type::Function(_)
                    | dir::Type::FunctionSignature(_)
                    | dir::Type::FunctionPointer(_)
            ) {
                continue;
            }

            match self.callable_implementation(template)? {
                Some(CallableImplementation::Binding { .. }) => {
                    reachable.bindings.insert(template);
                }
                Some(CallableImplementation::Intrinsic { .. }) => {}
                None => reachable
                    .instances
                    .push(dir::InstanceKey::new(template, arguments).with_receiver(receiver)),
            }
        }

        // collect calls from the concrete bodies queued for lowering
        for body in bodies {
            if let Some(owner) = body.constructs {
                self.collect_constructor_initializers(owner, body.instance, &mut reachable)?;
            }
            for default in body.defaults.iter().flatten() {
                self.collect_body(body.source, *default, body.instance, &mut reachable)?;
            }

            match self.collect_body(body.source, body.expression, body.instance, &mut reachable) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // collect calls from the module initializer expressions
        let expressions: Vec<_> = self
            .initializers
            .iter()
            .map(|(_, expression)| *expression)
            .collect();
        for expression in expressions {
            match self.collect_body(self.module, expression, None, &mut reachable) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // collect calls recursively from each declared instance body
        let mut instances = Vec::new();
        let mut index = 0;
        while index < reachable.instances.len() {
            let key = reachable.instances[index].clone();
            index += 1;
            let body =
                match self.declare_instance(builder, key.symbol, key.receiver, &key.arguments) {
                    Ok(Some(body)) => body,
                    Ok(None) => continue,
                    Err(CompilerError::Diagnostic(diagnostic)) => {
                        self.bank_failed_callable(Some(key.symbol), diagnostic, errors);

                        continue;
                    }
                    Err(error) => return Err(error),
                };

            if let Some(owner) = body.constructs {
                self.collect_constructor_initializers(owner, body.instance, &mut reachable)?;
            }
            for default in body.defaults.iter().flatten() {
                self.collect_body(
                    key.symbol.module_id,
                    *default,
                    body.instance,
                    &mut reachable,
                )?;
            }

            match self.collect_body(
                key.symbol.module_id,
                body.expression,
                body.instance,
                &mut reachable,
            ) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
            instances.push(body);
        }

        Ok((instances, reachable))
    }

    /// Declare one concrete instance unless its representation is already declared.
    fn declare_instance(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Option<FunctionDefinition>> {
        // find the sema instance materializing this body's types
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let specialization = self.specialization_of(symbol, receiver, &arguments)?;

        // require the materialized instance behind every generic selection
        if specialization.is_none() && !arguments.is_empty() {
            let path = self.symbol_path(symbol)?;

            return Err(CompilerError::Internal {
                message: format!("an instance of '{path}' was never materialized"),
            });
        }

        // key the instance by its runtime representation
        let pointer_bytes = builder.pointer_bytes();
        let lifetime_parameters = LifetimeParameters::default();
        let key = self
            .type_lowerer(builder.split_mut(), pointer_bytes, &lifetime_parameters)
            .with_instance(specialization)
            .generic_instance_key(symbol, receiver, &arguments)?;

        // skip an instance whose representation is already declared
        if self.functions.contains_key(&key) {
            return Ok(None);
        }

        // declare under the instance's concrete types and polymorphic lifetimes
        let lifetime_parameters = self.lifetime_parameters(self.symbol_type(symbol)?)?;

        self.declare_instance_header(builder, &key, specialization, &lifetime_parameters)
    }

    /// Declare the header of one generic instance and queue its body.
    fn declare_instance_header(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<Option<FunctionDefinition>> {
        let symbol = key.symbol;

        // find the declaration's source function in its defining module's tree
        let Some(declaration) = self
            .state(symbol.module_id)?
            .bindings
            .get_symbol(symbol.local_id)
            .declaration
        else {
            return Err(CompilerError::Internal {
                message: "an instantiated callable without a declaration".to_string(),
            });
        };

        // declare member callables through their owner's receiver
        if let Ok(member) = declaration.local_id.try_into_typed::<dir::Member>() {
            return self
                .declare_member_instance_header(
                    builder,
                    key,
                    specialization,
                    lifetime_parameters,
                    member,
                )
                .map(Some);
        }

        // declare interface members through their declared receiver
        if let Ok(member) = declaration.local_id.try_into_typed::<dir::TypeMember>() {
            return self.declare_interface_instance_header(
                builder,
                key,
                specialization,
                lifetime_parameters,
                member,
            );
        }

        // require a plain function declaration for everything else
        let Ok(declaration) = declaration.local_id.try_into_typed::<dir::Declaration>() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "instantiated callable {symbol:?} declares through {:?} instead of a declaration",
                    declaration.local_id
                ),
            });
        };

        // resolve the source body and parameter symbols
        let dir::Declaration::Function(function) =
            self.state(symbol.module_id)?.tree().get(declaration)
        else {
            return Err(CompilerError::Internal {
                message: "an instantiated non-function declaration".to_string(),
            });
        };
        let Some(expression) = function.body else {
            return Err(CompilerError::Internal {
                message: "an instantiated bodiless function".to_string(),
            });
        };

        // collect each parameter's default expression and declared symbol
        let parameter_nodes = function.signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        let mut defaults = Vec::with_capacity(parameter_nodes.len());
        let mut parameter_provenance = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            defaults.push(
                self.state(symbol.module_id)?
                    .tree()
                    .get(parameter)
                    .default_value(),
            );
            let node = parameter.into_global_any(symbol.module_id);
            parameter_provenance.push(self.node_provenance(node)?);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }

        // lower the signature at the instance's concrete types
        let declared = self.instance_type(specialization, self.symbol_type(symbol)?)?;
        let (parameters, result) =
            self.lower_signature(builder, declared, specialization, lifetime_parameters)?;
        if parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }
        let source = self.node_provenance(declaration.into_global_any(symbol.module_id))?;

        self.declare_instance_function(
            builder,
            key,
            specialization,
            source,
            parameters,
            parameter_provenance,
            result,
            symbols,
            false,
            lifetime_parameters,
            expression,
            None,
            defaults,
        )
        .map(Some)
    }

    /// Declare the header of one instantiated interface member and queue its default body.
    fn declare_interface_instance_header(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
        member: dir::LocalNodeId<dir::TypeMember>,
    ) -> CompilerResult<Option<FunctionDefinition>> {
        let symbol = key.symbol;

        // resolve the interface member's signature and default body
        let state = self.state(symbol.module_id)?;
        let dir::TypeMember::Method {
            signature,
            body,
            is_static,
            ..
        } = state.tree().get(member)
        else {
            return Err(CompilerError::Internal {
                message: "an instantiated non-method interface member".to_string(),
            });
        };
        let is_static = *is_static;
        let body = *body;
        let this_parameter = signature.this_parameter;
        let parameter_nodes = signature.parameters.to_vec();

        // synthesize the builtin implementation behind a bodiless requirement
        let Some(expression) = body else {
            return self
                .declare_builtin_member_instance(
                    builder,
                    key,
                    specialization,
                    lifetime_parameters,
                    member,
                )
                .map(|()| None);
        };

        // collect each parameter's default expression and declared symbol
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        let mut defaults = Vec::with_capacity(parameter_nodes.len());
        let mut parameter_provenance = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            defaults.push(
                self.state(symbol.module_id)?
                    .tree()
                    .get(parameter)
                    .default_value(),
            );
            let node = parameter.into_global_any(symbol.module_id);
            parameter_provenance.push(self.node_provenance(node)?);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }

        // lower the signature at the instance's concrete types
        let declared = self.instance_type(specialization, self.symbol_type(symbol)?)?;
        let (mut parameters, result) =
            self.lower_signature(builder, declared, specialization, lifetime_parameters)?;
        if parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }

        // prepend the declared receiver of instance members
        let has_this = !is_static;
        let source = self.node_provenance(member.into_global_any(symbol.module_id))?;
        if has_this {
            let this = self.declared_receiver_type(
                builder,
                declared,
                specialization,
                lifetime_parameters,
            )?;
            parameters.insert(0, this);
            let source = match this_parameter {
                Some(parameter) => {
                    self.node_provenance(parameter.into_global_any(symbol.module_id))?
                }
                None => source,
            };
            parameter_provenance.insert(0, source);
        }

        self.declare_instance_function(
            builder,
            key,
            specialization,
            source,
            parameters,
            parameter_provenance,
            result,
            symbols,
            has_this,
            lifetime_parameters,
            expression,
            None,
            defaults,
        )
        .map(Some)
    }

    /// Declare one bodiless interface requirement through its builtin implementation.
    fn declare_builtin_member_instance(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
        member: dir::LocalNodeId<dir::TypeMember>,
    ) -> CompilerResult<()> {
        let symbol = key.symbol;

        // recognize the canonical member the requirement declares
        let language_member = self.declared_language_member(symbol)?;
        if language_member != Some(dir::LanguageItem::Clone.member("clone")) {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "an instantiated bodiless interface member".to_string(),
            }
            .into());
        }

        // lower the clone signature at the instance's concrete receiver
        let declared = self.instance_type(specialization, self.symbol_type(symbol)?)?;
        let (mut parameters, result) =
            self.lower_signature(builder, declared, specialization, lifetime_parameters)?;
        let this =
            self.declared_receiver_type(builder, declared, specialization, lifetime_parameters)?;
        parameters.insert(0, this);

        // collect the receiver and parameter occurrences
        let (this_parameter, parameter_nodes) = {
            let state = self.state(symbol.module_id)?;
            let dir::TypeMember::Method { signature, .. } = state.tree().get(member) else {
                return Err(CompilerError::Internal {
                    message: "a builtin implementation attached to a non-method member".to_string(),
                });
            };

            (signature.this_parameter, signature.parameters.to_vec())
        };
        if parameters.len() != parameter_nodes.len() + 1 {
            return Err(CompilerError::Internal {
                message: "builtin parameters disagree with the declared signature".to_string(),
            });
        }
        let receiver_provenance = match this_parameter {
            Some(parameter) => self.node_provenance(parameter.into_global_any(symbol.module_id))?,
            None => self.node_provenance(member.into_global_any(symbol.module_id))?,
        };
        let mut parameter_provenance = parameter_nodes
            .into_iter()
            .map(|parameter| self.node_provenance(parameter.into_global_any(symbol.module_id)))
            .collect::<CompilerResult<Vec<_>>>()?;
        parameter_provenance.insert(0, receiver_provenance);

        // name the specialized clone instance
        let name = self.symbol_path(symbol)?;
        let mut display = match specialization {
            Some((module, instance)) => {
                self.specialized_display_arguments(builder, key, module, instance)?
            }
            None => key.arguments.clone(),
        };
        // name the closed receiver ahead of the type arguments
        if let Some(receiver) = key.receiver {
            display.insert(0, receiver);
        }
        let mut source = self.node_provenance(member.into_global_any(symbol.module_id))?;
        if let Some(specialization) = specialization {
            let site = self.instance_provenance(specialization)?;
            let (_, mut provenance) = builder.split_mut();
            source = provenance.expand(source, site);
            for parameter in &mut parameter_provenance {
                *parameter = provenance.expand(*parameter, site);
            }
        }

        // declare the header under its instantiated symbol
        let base = mir::Symbol::declared(builder.intern(&name), Self::symbol_identity(symbol));
        let instance = base.instantiate(&display, builder.tree());
        let header = builder
            .function_header(&name)
            .arguments(display)
            .symbol(instance);
        let header = lifetime_parameters.declare(header);
        let parameters = parameters.into_iter().zip(parameter_provenance);
        let header = header.parameters(parameters).result(result);
        let function = builder.declare_function(header, &[source]);
        self.index_language_declaration(function, symbol)?;
        self.functions
            .insert(key.clone(), FunctionDeclaration::Declared(function));

        // queue the receiver copy the conformance supplies
        self.synthesized_clones.push((function, result));

        Ok(())
    }

    /// Lower the receiver type one instantiated signature declares.
    fn declared_receiver_type(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declared: dir::GlobalTypeId,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<mir::TypeId> {
        let (declared_signature, signature_module) = self.signature(declared)?;
        let declared_this = self
            .types(signature_module)?
            .signature(declared_signature)
            .this_parameter;
        let Some(this_type) = declared_this else {
            return Err(CompilerError::Internal {
                message: "an instance method without a receiver".to_string(),
            });
        };
        let pointer_bytes = builder.pointer_bytes();
        let this = self
            .type_lowerer(builder.split_mut(), pointer_bytes, lifetime_parameters)
            .with_instance(specialization)
            .lower(this_type)?;

        Ok(this)
    }

    /// Declare the header of one member instance and queue its body.
    fn declare_member_instance_header(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
        member: dir::LocalNodeId<dir::Member>,
    ) -> CompilerResult<FunctionDefinition> {
        let symbol = key.symbol;
        let member_node = member.into_global_any(symbol.module_id);

        // find the owner declaring this member
        let state = self.state(symbol.module_id)?;
        let owner = state
            .definitions
            .iter_definitions()
            .find(|(_, definition)| definition.method_declared_at(member_node) == Some(symbol))
            .map(|(owner, _)| owner)
            .ok_or_else(|| CompilerError::Internal {
                message: "an instantiated member without an owner".to_string(),
            })?;

        // classify the member space
        let is_static = match self.definition(owner)? {
            Some(definition) => definition.members().iter().any(|candidate| {
                matches!(
                    candidate,
                    dir::DefinitionMember::Method(method)
                        if method.symbol == symbol && method.space == dir::MemberSpace::Static
                )
            }),
            None => false,
        };

        // resolve the member body and parameter symbols
        let state = self.state(symbol.module_id)?;
        let dir::Member::Method {
            signature, body, ..
        } = state.tree().get(member)
        else {
            return Err(CompilerError::Internal {
                message: "an instantiated non-method member".to_string(),
            });
        };
        let Some(expression) = *body else {
            return Err(CompilerError::Internal {
                message: "an instantiated bodiless member".to_string(),
            });
        };

        // collect each parameter's default expression and declared symbol
        let role = signature.role;
        let this_parameter = signature.this_parameter;
        let parameter_nodes = signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        let mut defaults = Vec::with_capacity(parameter_nodes.len());
        let mut parameter_provenance = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            defaults.push(
                self.state(symbol.module_id)?
                    .tree()
                    .get(parameter)
                    .default_value(),
            );
            let node = parameter.into_global_any(symbol.module_id);
            parameter_provenance.push(self.node_provenance(node)?);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }

        // lower the signature at the instance's concrete types
        let declared = self.instance_type(specialization, self.symbol_type(symbol)?)?;
        let (mut parameters, mut result) =
            self.lower_signature(builder, declared, specialization, lifetime_parameters)?;
        if parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }

        // prepend the receiver of instance members
        let pointer_bytes = builder.pointer_bytes();
        let this = match role {
            // take no receiver for static members
            _ if is_static => None,
            // receive an exclusive borrow of uninitialized storage
            Some(dir::FunctionRole::Constructor) => {
                let owner = self.symbol_type(owner)?;
                let nominal = self
                    .type_lowerer(builder.split_mut(), pointer_bytes, lifetime_parameters)
                    .with_instance(specialization)
                    .lower_nominal(owner)?;

                // the instance's place argument names the construction space
                let receiver_storage = match self.constructor_instance_space(specialization)? {
                    Some(space) => mir::Storage::heap(ModuleLowerer::mir_space(space)),
                    None => nominal_receiver_storage(builder.tree(), nominal.value),
                };

                Some(constructor_receiver_type(
                    builder.tree_mut(),
                    nominal.storage,
                    receiver_storage,
                ))
            }
            // pass this at the declared receiver type
            _ => Some(self.declared_receiver_type(
                builder,
                declared,
                specialization,
                lifetime_parameters,
            )?),
        };
        let has_this = this.is_some();
        let source = self.node_provenance(member.into_global_any(symbol.module_id))?;
        if let Some(this) = this {
            parameters.insert(0, this);
            let source = match this_parameter {
                Some(parameter) => {
                    self.node_provenance(parameter.into_global_any(symbol.module_id))?
                }
                None => source,
            };
            parameter_provenance.insert(0, source);
        }

        // give constructors a void result
        if role == Some(dir::FunctionRole::Constructor) {
            result = builder.tree_mut().intern_type(mir::Type::Void);
        }

        self.declare_instance_function(
            builder,
            key,
            specialization,
            source,
            parameters,
            parameter_provenance,
            result,
            symbols,
            has_this,
            lifetime_parameters,
            expression,
            (role == Some(dir::FunctionRole::Constructor)).then_some(owner),
            defaults,
        )
    }

    /// Declare one instance header under its canonical name and queue its body.
    fn declare_instance_function(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        mut source: ProvenanceId,
        parameters: Vec<mir::TypeId>,
        mut parameter_provenance: Vec<ProvenanceId>,
        result: mir::TypeId,
        symbols: Vec<dir::LocalSymbolId>,
        has_this: bool,
        lifetime_parameters: &LifetimeParameters,
        expression: dir::LocalNodeId<dir::Expression>,
        constructs: Option<dir::GlobalSymbolId>,
        defaults: Vec<Option<dir::LocalNodeId<dir::Expression>>>,
    ) -> CompilerResult<FunctionDefinition> {
        let symbol = key.symbol;
        let name = self.symbol_path(symbol)?;

        // name every specialized memory argument explicitly
        let mut display = match specialization {
            Some((module, instance)) => {
                self.specialized_display_arguments(builder, key, module, instance)?
            }
            None => key.arguments.clone(),
        };
        // name the closed receiver ahead of the type arguments
        if let Some(receiver) = key.receiver {
            display.insert(0, receiver);
        }

        // derive the instantiated function and parameters at the selected instance
        if let Some(specialization) = specialization {
            let site = self.instance_provenance(specialization)?;
            let (_, mut provenance) = builder.split_mut();
            source = provenance.expand(source, site);
            for parameter in &mut parameter_provenance {
                *parameter = provenance.expand(*parameter, site);
            }
        }

        // declare the header under its instantiated symbol
        let base = mir::Symbol::declared(builder.intern(&name), Self::symbol_identity(symbol));
        let instance = base.instantiate(&display, builder.tree());
        let header = builder
            .function_header(&name)
            .arguments(display)
            .symbol(instance);
        let header = lifetime_parameters.declare(header);
        let parameters = parameters.into_iter().zip(parameter_provenance);
        let header = header.parameters(parameters).result(result);
        let function = builder.declare_function(header, &[source]);
        self.index_language_declaration(function, symbol)?;
        self.functions
            .insert(key.clone(), FunctionDeclaration::Declared(function));

        Ok(FunctionDefinition {
            function,
            symbol,
            has_this,
            parameters: symbols,
            instance: specialization,
            lifetime_parameters: lifetime_parameters.clone(),
            source: symbol.module_id,
            expression,
            constructs,
            defaults,
        })
    }

    /// Return the display arguments naming one specialized instance's spaces and types.
    fn specialized_display_arguments(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        module: ModuleId,
        instance: dir::LocalInstanceId,
    ) -> CompilerResult<Vec<mir::StaticId>> {
        // pair every selected parameter with its bound argument
        let pairs = {
            let instance = self.state(module)?.generics.get_instance(instance);

            instance
                .key
                .arguments
                .iter()
                .map(|binding| (binding.parameter, binding.argument))
                .collect::<Vec<_>>()
        };
        let mut bindings = Vec::with_capacity(pairs.len());
        for (parameter, argument) in pairs {
            let kind = self
                .state(parameter.module_id)?
                .generics
                .get_parameter(parameter.local_id)
                .memory_parameter();
            bindings.push((kind, argument));
        }

        // key arguments hold the non-local spaces and the type arguments in
        //  binding order, so the walk below consumes them in lockstep
        let mut remaining = key.arguments.iter().cloned();
        let mut display = Vec::with_capacity(bindings.len());
        for (kind, argument) in bindings {
            match kind {
                Some(dir::MemoryParameter::Place | dir::MemoryParameter::Space) => {
                    // lifetime halves of a place erase like regions
                    if self.type_is_lifetime(argument)? {
                        continue;
                    }

                    let Some(space) = self.place_space(argument)? else {
                        return Err(CompilerError::Internal {
                            message: "an instance carries an unplaced space argument".to_string(),
                        });
                    };

                    // the ambient local space elides from every display name
                    if space != dir::Space::Local {
                        remaining.next();
                        let space = ModuleLowerer::mir_space(space);
                        display.push(builder.tree_mut().intern_static(mir::Static::Space(space)));
                    }
                }
                Some(_) => {}
                None => {
                    let Some(argument) = remaining.next() else {
                        return Err(CompilerError::Internal {
                            message: "an instance key misses one type argument".to_string(),
                        });
                    };
                    display.push(argument);
                }
            }
        }

        Ok(display)
    }

    /// Declare the synthesized constructors of this module's own classes.
    pub(in crate::lower) fn declare_default_constructors(
        &mut self,
        builder: &mut mir::ModuleBuilder,
    ) -> CompilerResult<()> {
        // collect the concrete constructor-less classes with field initializers
        let mut classes = Vec::new();
        for (symbol, definition) in self.local().definitions.iter_definitions() {
            let dir::Definition::Class(class) = definition else {
                continue;
            };
            if symbol.module_id != self.module {
                continue;
            }
            let declares_constructor = class.members.iter().any(|member| {
                matches!(
                    member,
                    dir::DefinitionMember::Method(method)
                        if method.role == Some(dir::FunctionRole::Constructor)
                )
            });
            if declares_constructor {
                continue;
            }

            classes.push(symbol);
        }

        // define each constructor beside its class declaration
        for symbol in classes {
            let Some(definition) = self.definition(symbol)?.cloned() else {
                continue;
            };
            if self.definition_is_parameterized(symbol.module_id, &definition)? {
                continue;
            }
            if !self.class_has_field_initializers(symbol)? {
                continue;
            }

            self.declare_default_constructor(builder, symbol, &[])?;
        }

        Ok(())
    }

    /// Declare one synthesized default constructor and queue its prologue body.
    pub(in crate::lower) fn declare_default_constructor(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        class: dir::GlobalSymbolId,
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<()> {
        // find the sema instance materializing the class's types
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let specialization = self.specialization_of(class, None, &arguments)?;

        // key the constructor by the class's runtime representation
        let pointer_bytes = builder.pointer_bytes();
        let lifetime_parameters = LifetimeParameters::default();
        let key = self
            .type_lowerer(builder.split_mut(), pointer_bytes, &lifetime_parameters)
            .with_instance(specialization)
            .generic_instance_key(class, None, &arguments)?;

        // skip a constructor whose representation is already declared
        if self.functions.contains_key(&key) {
            return Ok(());
        }

        // receive an exclusive borrow of the constructed storage
        let source = self.symbol_type(class)?;
        let nominal = self
            .type_lowerer(builder.split_mut(), pointer_bytes, &lifetime_parameters)
            .with_instance(specialization)
            .lower_nominal(source)?;
        let receiver_storage = nominal_receiver_storage(builder.tree(), nominal.value);
        let this = constructor_receiver_type(builder.tree_mut(), nominal.storage, receiver_storage);
        let void = builder.tree_mut().intern_type(mir::Type::Void);
        let name = format!("{}.constructor", self.symbol_path(class)?);
        let declaration = self.declaration(class)?;
        let mut source = self.node_provenance(declaration)?;
        if let Some(specialization) = specialization {
            let site = self.instance_provenance(specialization)?;
            let (_, mut provenance) = builder.split_mut();
            source = provenance.expand(source, site);
        }

        // import the constructor a foreign class defines beside itself
        if class.module_id != self.module && arguments.is_empty() {
            let header = lifetime_parameters.declare(builder.function_header(&name));
            let header = header.parameter(this, source).result(void);
            let function = builder.external_function(header, &[source]);
            self.functions
                .insert(key, FunctionDeclaration::Declared(function));

            return Ok(());
        }

        // define generic instances and own-class constructors locally
        let base = mir::Symbol::declared(builder.intern(&name), Self::symbol_identity(class));
        let instance = base.instantiate(&key.arguments, builder.tree());
        let header = builder
            .function_header(&name)
            .arguments(key.arguments.iter().cloned())
            .symbol(instance);
        let header = lifetime_parameters.declare(header);
        let header = header.parameter(this, source).result(void);
        let function = builder.declare_function(header, &[source]);
        self.functions
            .insert(key, FunctionDeclaration::Declared(function));
        self.synthesized_constructors
            .push((class, specialization, function));

        Ok(())
    }
}

/// Intern one constructor receiver: an exclusive borrow of the uninitialized constructed storage.
pub(in crate::lower) fn constructor_receiver_type(
    tree: &mut mir::Tree,
    storage: mir::TypeId,
    receiver_storage: mir::Storage,
) -> mir::TypeId {
    let pointee = tree.intern_type(mir::Type::Uninit { value: storage });

    insert_reference_type(
        tree,
        mir::ReferenceKind::Borrowed,
        mir::Access::Exclusive,
        receiver_storage,
        pointee,
    )
}

/// Return the storage one nominal's constructor receiver borrows.
///
/// Reference nominals construct into their own allocation, while owned
/// nominals construct in place inside a frame.
pub(in crate::lower) fn nominal_receiver_storage(
    tree: &mir::Tree,
    value: mir::TypeId,
) -> mir::Storage {
    tree.ty(value)
        .reference_storage()
        .unwrap_or(mir::Storage::Frame)
}
