use destack_artifact::DiagnosticLike;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::r#type::LoweredSignature;
use crate::lower::{
    FunctionDefinition, LifetimeParameters, ModuleLowerer, Reachable, ReceiverBinding,
    TypeSubstitution,
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
        // collect calls from the concrete bodies queued for lowering
        let mut reachable = Reachable::default();
        for body in bodies {
            match self.collect_body(
                body.source,
                body.expression,
                &body.type_substitution,
                &mut reachable,
            ) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // collect calls from the module initializer expressions
        let substitution = TypeSubstitution::default();
        let expressions: Vec<_> = self
            .initializers
            .iter()
            .map(|(_, expression)| *expression)
            .collect();
        for expression in expressions {
            match self.collect_body(self.module, expression, &substitution, &mut reachable) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // collect calls recursively from each declared instance body
        let mut instances = Vec::new();
        let mut index = 0;
        while index < reachable.instances.len() {
            let (symbol, bindings, enclosing) = reachable.instances[index].clone();
            index += 1;
            let body = match self.declare_instance(builder, symbol, &bindings, &enclosing) {
                Ok(Some(body)) => body,
                Ok(None) => continue,
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.bank_failed_callable(Some(symbol), diagnostic, errors);

                    continue;
                }
                Err(error) => return Err(error),
            };
            match self.collect_body(
                symbol.module_id,
                body.expression,
                &body.type_substitution,
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
        enclosing: &TypeSubstitution,
    ) -> CompilerResult<Option<FunctionDefinition>> {
        // key the instance by its runtime representation, closing collected
        //  arguments through the enclosing scope's substitution
        let pointer_bytes = builder.pointer_bytes();
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let lifetime_parameters = LifetimeParameters::default();
        let key = self
            .type_lowerer(
                builder.tree_mut(),
                pointer_bytes,
                enclosing,
                &lifetime_parameters,
            )
            .generic_instance_key(symbol, &arguments)?;
        if self.functions.contains_key(&key) {
            return Ok(None);
        }

        // declare under the instance's concrete types and polymorphic
        //  lifetimes, keeping the enclosing bindings for nested resolution
        let lifetime_parameters = self.lifetime_parameters(self.symbol_type(symbol)?)?;
        let mut type_substitution = enclosing.clone();
        for binding in bindings {
            type_substitution.insert(binding.parameter, binding.argument);
        }
        let declared =
            self.declare_instance_header(builder, &key, &type_substitution, &lifetime_parameters);

        declared.map(Some)
    }

    /// Declare the header of one generic instance and queue its body.
    fn declare_instance_header(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        type_substitution: &TypeSubstitution,
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
                type_substitution,
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
        for parameter in parameter_nodes {
            let node = parameter.into_global_any(symbol.module_id);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }
        let declared = self.symbol_type(symbol)?;
        let signature =
            self.lower_signature(builder, declared, type_substitution, lifetime_parameters)?;
        if signature.parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }

        self.declare_instance_function(
            builder,
            key,
            signature,
            symbols,
            false,
            type_substitution.clone(),
            lifetime_parameters,
            expression,
        )
    }

    /// Declare the header of one member instance and queue its body.
    fn declare_member_instance_header(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        type_substitution: &TypeSubstitution,
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

        // receive extension members at their target
        let receiver = match self.definition(owner)? {
            Some(dir::Definition::Extension(extension)) => {
                ReceiverBinding::Type(extension.target.r#type())
            }
            _ => ReceiverBinding::Application(dir::GenericApplication {
                symbol: owner,
                arguments: dir::TypeListId::EMPTY,
            }),
        };
        let type_substitution = type_substitution.clone().with_receiver(receiver);

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
        for parameter in parameter_nodes {
            let node = parameter.into_global_any(symbol.module_id);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }
        let declared = self.symbol_type(symbol)?;
        let mut signature =
            self.lower_signature(builder, declared, &type_substitution, lifetime_parameters)?;
        if signature.parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }

        // prepend the receiver of instance members
        let pointer_bytes = builder.pointer_bytes();
        let this = match role {
            // take no receiver for static members
            _ if is_static => None,
            // receive an exclusive borrow in constructors
            Some(dir::FunctionRole::Constructor) => {
                let nominal = self
                    .type_lowerer(
                        builder.tree_mut(),
                        pointer_bytes,
                        &type_substitution,
                        lifetime_parameters,
                    )
                    .lower_nominal(owner, &[])?;

                Some(builder.tree_mut().intern_type(mir::Type::Reference {
                    kind: mir::ReferenceKind::Borrowed,
                    lifetime: mir::Lifetime::empty(),
                    storage: mir::Storage::Heap(mir::Space::Local),
                    access: mir::Access::Exclusive,
                    pointee: nominal.storage,
                    nullability: mir::Nullability::None,
                }))
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
                    self.type_lowerer(
                        builder.tree_mut(),
                        pointer_bytes,
                        &type_substitution,
                        lifetime_parameters,
                    )
                    .lower(this_type)?,
                )
            }
        };
        let has_this = this.is_some();
        if let Some(this) = this {
            signature.parameters.insert(0, this);
        }

        // give constructors a void result
        if role == Some(dir::FunctionRole::Constructor) {
            signature.result = builder.tree_mut().intern_type(mir::Type::Void);
        }

        self.declare_instance_function(
            builder,
            key,
            signature,
            symbols,
            has_this,
            type_substitution,
            lifetime_parameters,
            expression,
        )
    }

    /// Declare one instance header under its canonical name and queue its body.
    fn declare_instance_function(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        signature: LoweredSignature,
        symbols: Vec<dir::LocalSymbolId>,
        has_this: bool,
        type_substitution: TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
        expression: dir::LocalNodeId<dir::Expression>,
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
        let header = header
            .parameters(signature.parameters)
            .result(signature.result);
        let function = builder.declare_function(header);
        self.functions
            .insert(key.clone(), FunctionDeclaration::Declared(function));

        Ok(FunctionDefinition {
            function,
            symbol,
            has_this,
            parameters: symbols,
            type_substitution,
            lifetime_parameters: lifetime_parameters.clone(),
            source: symbol.module_id,
            expression,
        })
    }
}
