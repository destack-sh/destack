use destack_artifact::DiagnosticLike;
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    CallableImplementation, FunctionDefinition, LifetimeParameters, ModuleLowerer, Reachable,
    insert_local_reference,
};
use crate::{CompilerError, CompilerResult};

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
    /// The concrete generic arguments.
    pub(in crate::lower) arguments: Vec<mir::StaticId>,
}

impl GenericInstanceKey {
    /// Create the instance key of one non-generic declaration.
    pub(in crate::lower) fn non_generic(symbol: dir::GlobalSymbolId) -> Self {
        Self {
            symbol,
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
        // seed from the instances sema closed, routing intrinsic and binding callables
        let mut reachable = Reachable::default();
        let closed: Vec<_> = self
            .state(self.module)?
            .generics
            .iter_instances()
            .filter(|(_, instance)| instance.origin == dir::InstanceOrigin::Instantiation)
            .map(|(_, instance)| {
                (
                    instance.selection.symbol,
                    instance.selection.arguments.clone(),
                )
            })
            .collect();
        for (template, arguments) in closed {
            // keep the callable instances; nominal ones declare through their representations
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
                None => reachable.instances.push((template, arguments)),
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
            let (symbol, bindings) = reachable.instances[index].clone();
            index += 1;
            let body = match self.declare_instance(builder, symbol, &bindings) {
                Ok(Some(body)) => body,
                Ok(None) => continue,
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.bank_failed_callable(Some(symbol), diagnostic, errors);

                    continue;
                }
                Err(error) => return Err(error),
            };
            if let Some(owner) = body.constructs {
                self.collect_constructor_initializers(owner, body.instance, &mut reachable)?;
            }
            for default in body.defaults.iter().flatten() {
                self.collect_body(symbol.module_id, *default, body.instance, &mut reachable)?;
            }
            match self.collect_body(
                symbol.module_id,
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
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Option<FunctionDefinition>> {
        // find the sema instance materializing this body's types
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let specialization = self.specialization_of(symbol, &arguments)?;

        // key the instance by its runtime representation
        let pointer_bytes = builder.pointer_bytes();
        let lifetime_parameters = LifetimeParameters::default();
        let key = self
            .type_lowerer(builder.tree_mut(), pointer_bytes, &lifetime_parameters)
            .with_instance(specialization)
            .generic_instance_key(symbol, &arguments)?;
        if self.functions.contains_key(&key) {
            return Ok(None);
        }

        // declare under the instance's concrete types and polymorphic lifetimes
        let lifetime_parameters = self.lifetime_parameters(self.symbol_type(symbol)?)?;
        let declared =
            self.declare_instance_header(builder, &key, specialization, &lifetime_parameters);

        declared.map(Some)
    }

    /// Declare the header of one generic instance and queue its body.
    fn declare_instance_header(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<FunctionDefinition> {
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
            return self.declare_member_instance_header(
                builder,
                key,
                specialization,
                lifetime_parameters,
                member,
            );
        }
        let Ok(declaration) = declaration.local_id.try_into_typed::<dir::Declaration>() else {
            return Err(CompilerError::Internal {
                message: "an instantiated non-declaration callable".to_string(),
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
        let parameter_nodes = function.signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        let mut defaults = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            defaults.push(
                self.state(symbol.module_id)?
                    .tree()
                    .get(parameter)
                    .default_value(),
            );
            let node = parameter.into_global_any(symbol.module_id);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }
        let declared = self.instance_type(specialization, self.symbol_type(symbol)?)?;
        let (parameters, result) =
            self.lower_signature(builder, declared, specialization, lifetime_parameters)?;
        if parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }

        self.declare_instance_function(
            builder,
            key,
            specialization,
            parameters,
            result,
            symbols,
            false,
            lifetime_parameters,
            expression,
            None,
            defaults,
        )
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
        let state = self.state(symbol.module_id)?;

        // find the owner declaring this member
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
        let role = signature.role;
        let parameter_nodes = signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        let mut defaults = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            defaults.push(
                self.state(symbol.module_id)?
                    .tree()
                    .get(parameter)
                    .default_value(),
            );
            let node = parameter.into_global_any(symbol.module_id);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }
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
                    .type_lowerer(builder.tree_mut(), pointer_bytes, lifetime_parameters)
                    .with_instance(specialization)
                    .lower_nominal(owner)?;

                Some(constructor_receiver_type(
                    builder.tree_mut(),
                    nominal.storage,
                ))
            }
            // pass this at the declared receiver type
            _ => {
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

                Some(
                    self.type_lowerer(builder.tree_mut(), pointer_bytes, lifetime_parameters)
                        .with_instance(specialization)
                        .lower(this_type)?,
                )
            }
        };
        let has_this = this.is_some();
        if let Some(this) = this {
            parameters.insert(0, this);
        }

        // give constructors a void result
        if role == Some(dir::FunctionRole::Constructor) {
            result = builder.tree_mut().intern_type(mir::Type::Void);
        }

        self.declare_instance_function(
            builder,
            key,
            specialization,
            parameters,
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
        parameters: Vec<mir::TypeId>,
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
        let base = mir::Symbol::named(builder.intern(&name));
        let instance = base.instantiate(&key.arguments, builder.tree());
        let header = builder
            .function_header(&name)
            .arguments(key.arguments.iter().cloned())
            .symbol(instance);
        let header = lifetime_parameters.declare(header);
        let header = header.parameters(parameters).result(result);
        let function = builder.declare_function(header);
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
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let specialization = self.specialization_of(class, &arguments)?;

        // key the constructor by the class's runtime representation
        let pointer_bytes = builder.pointer_bytes();
        let lifetime_parameters = LifetimeParameters::default();
        let key = self
            .type_lowerer(builder.tree_mut(), pointer_bytes, &lifetime_parameters)
            .with_instance(specialization)
            .generic_instance_key(class, &arguments)?;
        if self.functions.contains_key(&key) {
            return Ok(());
        }

        // receive an exclusive borrow of the constructed storage
        let source = self.symbol_type(class)?;
        let nominal = self
            .type_lowerer(builder.tree_mut(), pointer_bytes, &lifetime_parameters)
            .with_instance(specialization)
            .lower_nominal(source)?;
        let this = constructor_receiver_type(builder.tree_mut(), nominal.storage);
        let void = builder.tree_mut().intern_type(mir::Type::Void);
        let name = format!("{}.constructor", self.symbol_path(class)?);

        // import the constructor a foreign class defines beside itself
        if class.module_id != self.module && arguments.is_empty() {
            let header = lifetime_parameters.declare(builder.function_header(&name));
            let header = header.parameters(vec![this]).result(void);
            let function = builder.external_function(header);
            self.functions
                .insert(key, FunctionDeclaration::Declared(function));

            return Ok(());
        }

        // define generic instances and own-class constructors locally
        let base = mir::Symbol::named(builder.intern(&name));
        let instance = base.instantiate(&key.arguments, builder.tree());
        let header = builder
            .function_header(&name)
            .arguments(key.arguments.iter().cloned())
            .symbol(instance);
        let header = lifetime_parameters.declare(header);
        let header = header.parameters(vec![this]).result(void);
        let function = builder.declare_function(header);
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
    storage: mir::LocalNodeId<mir::Type>,
) -> mir::LocalNodeId<mir::Type> {
    let pointee = tree.intern_type(mir::Type::Uninit { value: storage });

    insert_local_reference(
        tree,
        mir::ReferenceKind::Borrowed,
        mir::Access::Exclusive,
        pointee,
    )
}
