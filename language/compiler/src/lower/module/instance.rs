use destack_artifact::DiagnosticLike;
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{FunctionDefinition, LifetimeParameters, LowerState};
use crate::{CompilerError, CompilerResult, LowerError};

/// The declaration outcome behind one callable instance key.
pub(in crate::lower) enum FunctionDeclaration {
    /// The declared MIR function.
    Declared(mir::FunctionId),
    /// The failed declaration behind a reported diagnostic.
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
impl LowerState<'_> {
    /// Declare one function per callable instance sema materialized.
    pub(in crate::lower) fn declare_instances(
        &mut self,
        tree: &mut mir::Tree,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<Vec<FunctionDefinition>> {
        // read the module's own interned instances
        let interned: Vec<_> = self
            .state(self.module)?
            .generics
            .iter_instances()
            .filter(|(_, instance)| instance.origin == dir::InstanceOrigin::Instantiation)
            .map(|(id, instance)| (id, instance.key.clone()))
            .collect();

        // declare each callable instance under its concrete bindings
        let mut definitions = Vec::new();
        for (id, key) in interned {
            // skip the nominal instances, which declare through their representations
            let Some(ty) = self
                .types(key.symbol.module_id)?
                .get_symbol_type_id(key.symbol)
            else {
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

            // skip bindings and intrinsics, which declare when a body reaches them
            if self.callable_implementation(key.symbol)?.is_some() {
                continue;
            }

            // key the instance like a call site: regions erase from the selection
            let bindings = self.instance_bindings(&key.arguments, None)?;
            let instance = (self.module, id);
            match self.declare_instance(tree, key.symbol, key.receiver, &bindings, instance) {
                Ok(Some(body)) => definitions.push(body),
                Ok(None) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.bank_failed_callable(Some(key.symbol), diagnostic, errors);
                }
                Err(error) => return Err(error),
            }
        }

        Ok(definitions)
    }

    /// Return whether one class declares instance fields with initializers.
    pub(in crate::lower) fn class_has_field_initializers(
        &self,
        class: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let Some(definition) = self.definition(class)? else {
            return Ok(false);
        };

        Ok(self
            .instance_fields(definition.members())
            .iter()
            .any(|field| field.initializer.is_some()))
    }

    /// Declare one concrete instance unless its representation is already declared.
    fn declare_instance(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        bindings: &[dir::GenericArgumentBinding],
        specialization: (ModuleId, dir::LocalInstanceId),
    ) -> CompilerResult<Option<FunctionDefinition>> {
        // key the instance by its runtime representation
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let pointer_bytes = self.pointer_bytes;
        let lifetime_parameters = LifetimeParameters::default();
        let key = self
            .type_lowerer(tree, pointer_bytes, &lifetime_parameters)
            .with_instance(Some(specialization))
            .generic_instance_key(symbol, receiver, &arguments)?;

        // skip an instance whose representation is already declared
        if self.functions.contains_key(&key) {
            return Ok(None);
        }

        // declare under the instance's concrete types and polymorphic lifetimes
        let lifetime_parameters = self.lifetime_parameters(self.symbol_type(symbol)?)?;

        self.declare_instance_header(tree, &key, Some(specialization), &lifetime_parameters)
    }

    /// Declare the header of one generic instance and queue its body.
    fn declare_instance_header(
        &mut self,
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<Option<FunctionDefinition>> {
        // find the declaration's source function in its defining module's tree
        let symbol = key.symbol;
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
                    tree,
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
                tree,
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
                    "an instantiated callable {symbol:?} declaring through {:?}",
                    declaration.local_id
                ),
            });
        };

        // resolve the source function's body
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

        // collect each parameter's default expression alongside its symbol
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
                    message: "a missing symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }

        // lower the signature at the instance's materialized identity
        let declared = self.instance_symbol_type(specialization, symbol)?;
        let (parameters, result) =
            self.lower_signature(tree, declared, specialization, lifetime_parameters)?;
        if parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }

        self.declare_instance_function(
            tree,
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
        .map(Some)
    }

    /// Declare the header of one instantiated interface member and queue its default body.
    fn declare_interface_instance_header(
        &mut self,
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
        member: dir::LocalNodeId<dir::TypeMember>,
    ) -> CompilerResult<Option<FunctionDefinition>> {
        // resolve the interface member's signature and default body
        let symbol = key.symbol;
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
        let parameter_nodes = signature.parameters.to_vec();

        // synthesize the builtin implementation behind a bodiless requirement
        let Some(expression) = body else {
            return self
                .declare_builtin_member_instance(tree, key, specialization, lifetime_parameters)
                .map(|()| None);
        };

        // collect each parameter's default expression alongside its symbol
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
                    message: "a missing symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }

        // lower the signature at the instance's materialized identity
        let declared = self.instance_symbol_type(specialization, symbol)?;
        let (mut parameters, result) =
            self.lower_signature(tree, declared, specialization, lifetime_parameters)?;
        if parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }

        // prepend the declared receiver of instance members
        let has_this = !is_static;
        if has_this {
            let this =
                self.declared_receiver_type(tree, declared, specialization, lifetime_parameters)?;
            parameters.insert(0, this);
        }

        self.declare_instance_function(
            tree,
            key,
            specialization,
            parameters,
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
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<()> {
        // leave an erased receiver to dynamic dispatch
        if let Some((module, instance)) = specialization
            && let Some(receiver) = self
                .state(module)?
                .generics
                .get_instance(instance)
                .key
                .receiver
            && let dir::Type::Application(application) = self.ty(receiver)?
            && matches!(
                self.definition(application.symbol)?,
                Some(dir::Definition::Interface(_))
            )
        {
            return Ok(());
        }

        // recognize the canonical member the requirement declares
        let symbol = key.symbol;
        let member = self.declared_language_member(symbol)?;
        if member != Some(dir::LanguageItem::Clone.member("clone")) {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "an instantiated bodiless interface member".to_string(),
            }
            .into());
        }

        // lower the clone signature at the instance's concrete receiver
        let declared = self.instance_symbol_type(specialization, symbol)?;
        let (mut parameters, result) =
            self.lower_signature(tree, declared, specialization, lifetime_parameters)?;
        let this =
            self.declared_receiver_type(tree, declared, specialization, lifetime_parameters)?;
        parameters.insert(0, this);

        // name every specialized memory argument explicitly
        let name = self.symbol_path(symbol)?;
        let mut display = match specialization {
            Some((module, instance)) => {
                self.specialized_display_arguments(tree, key, module, instance)?
            }
            None => key.arguments.clone(),
        };

        // name the closed receiver ahead of the type arguments
        if let Some(receiver) = key.receiver {
            display.insert(0, receiver);
        }

        // declare the header under its instantiated symbol
        let base = mir::Symbol::declared(self.strings.intern(&name), Self::symbol_identity(symbol));
        let instance = base.instantiate(&display, tree);
        let header = mir::FunctionHeaderBuilder::new(self.strings, &name)
            .arguments(display)
            .symbol(instance);
        let header = lifetime_parameters.declare(header);
        let header = header.parameters(parameters).result(result);
        let function = tree.insert(header.declared());
        self.index_language_declaration(function, symbol)?;

        // record the declaration so later call sites resolve to it
        self.functions
            .insert(key.clone(), FunctionDeclaration::Declared(function));

        // queue the receiver copy the conformance supplies
        self.synthesized_clones.push((function, result));

        Ok(())
    }

    /// Lower the receiver type one instantiated signature declares.
    fn declared_receiver_type(
        &mut self,
        tree: &mut mir::Tree,
        declared: dir::GlobalTypeId,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<mir::TypeId> {
        // read the receiver the signature declares
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

        // lower it at the instance's concrete types
        let pointer_bytes = self.pointer_bytes;
        let this = self
            .type_lowerer(tree, pointer_bytes, lifetime_parameters)
            .with_instance(specialization)
            .lower(this_type)?;

        Ok(this)
    }

    /// Declare the header of one member instance and queue its body.
    fn declare_member_instance_header(
        &mut self,
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        specialization: Option<(ModuleId, dir::LocalInstanceId)>,
        lifetime_parameters: &LifetimeParameters,
        member: dir::LocalNodeId<dir::Member>,
    ) -> CompilerResult<FunctionDefinition> {
        // find the owner declaring this member
        let symbol = key.symbol;
        let member_node = member.into_global_any(symbol.module_id);
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

        // resolve the member's body
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

        // collect each parameter's default expression alongside its symbol
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
                    message: "a missing symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }

        // lower the signature at the instance's materialized identity
        let declared = self.instance_symbol_type(specialization, symbol)?;
        let (mut parameters, mut result) =
            self.lower_signature(tree, declared, specialization, lifetime_parameters)?;
        if parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }

        // classify the receiver the member declares
        let pointer_bytes = self.pointer_bytes;
        let this = match role {
            // take no receiver for static members
            _ if is_static => None,
            // receive an exclusive borrow of uninitialized storage
            Some(dir::FunctionRole::Constructor) => {
                let owner = self.symbol_type(owner)?;
                let nominal = self
                    .type_lowerer(tree, pointer_bytes, lifetime_parameters)
                    .with_instance(specialization)
                    .lower_nominal(owner)?;

                // take the construction space from the instance's place argument
                let receiver_storage = match self.constructor_instance_space(specialization)? {
                    Some(space) => mir::Storage::heap(LowerState::mir_space(space)),
                    None => nominal_receiver_storage(tree, nominal.value),
                };

                Some(constructor_receiver_type(
                    tree,
                    nominal.storage,
                    receiver_storage,
                ))
            }
            // pass this at the declared receiver type
            _ => Some(self.declared_receiver_type(
                tree,
                declared,
                specialization,
                lifetime_parameters,
            )?),
        };

        // prepend the receiver the member declares
        let has_this = this.is_some();
        if let Some(this) = this {
            parameters.insert(0, this);
        }

        // give constructors a void result
        if role == Some(dir::FunctionRole::Constructor) {
            result = tree.intern_type(mir::Type::Void);
        }

        self.declare_instance_function(
            tree,
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
        tree: &mut mir::Tree,
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
        // name every specialized memory argument explicitly
        let symbol = key.symbol;
        let name = self.symbol_path(symbol)?;
        let mut display = match specialization {
            Some((module, instance)) => {
                self.specialized_display_arguments(tree, key, module, instance)?
            }
            None => key.arguments.clone(),
        };

        // name the closed receiver ahead of the type arguments
        if let Some(receiver) = key.receiver {
            display.insert(0, receiver);
        }

        // declare the header under its instantiated symbol
        let base = mir::Symbol::declared(self.strings.intern(&name), Self::symbol_identity(symbol));
        let instance = base.instantiate(&display, tree);
        let header = mir::FunctionHeaderBuilder::new(self.strings, &name)
            .arguments(display)
            .symbol(instance);
        let header = lifetime_parameters.declare(header);
        let header = header.parameters(parameters).result(result);
        let function = tree.insert(header.declared());
        self.index_language_declaration(function, symbol)?;

        // record the declaration so later call sites resolve to it
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
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        module: ModuleId,
        instance: dir::LocalInstanceId,
    ) -> CompilerResult<Vec<mir::StaticId>> {
        // pair every selected parameter with its bound argument
        let pairs = {
            let interned = self.state(module)?.generics.get_instance(instance);

            interned
                .key
                .arguments
                .iter()
                .map(|binding| (binding.parameter, binding.argument))
                .collect::<Vec<_>>()
        };

        // read the memory kind each parameter declares
        let mut bindings = Vec::with_capacity(pairs.len());
        for (parameter, argument) in pairs {
            let kind = self
                .state(parameter.module_id)?
                .generics
                .get_parameter(parameter.local_id)
                .memory_parameter();
            bindings.push((kind, argument));
        }

        // consume the key arguments in binding order alongside the parameters
        let mut remaining = key.arguments.iter().cloned();
        let mut display = Vec::with_capacity(bindings.len());
        for (kind, argument) in bindings {
            match kind {
                Some(dir::MemoryParameter::Place | dir::MemoryParameter::Space) => {
                    // erase the lifetime half of a place like a region
                    if self.type_is_lifetime(argument)? {
                        continue;
                    }

                    let Some(space) = self.place_space(argument)? else {
                        return Err(CompilerError::Internal {
                            message: "an instance carries an unplaced space argument".to_string(),
                        });
                    };

                    // elide the ambient local space from the display name
                    if space != dir::Space::Local {
                        remaining.next();
                        let space = LowerState::mir_space(space);
                        display.push(tree.intern_static(mir::Static::Space(space)));
                    }
                }
                // erase every other memory kind
                Some(_) => {}
                // take the next type argument in binding order
                None => {
                    let Some(argument) = remaining.next() else {
                        return Err(CompilerError::Internal {
                            message: "a missing type argument in one instance key".to_string(),
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
        tree: &mut mir::Tree,
    ) -> CompilerResult<()> {
        // collect the classes this module declares without a constructor
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

            self.declare_default_constructor(tree, symbol, &[])?;
        }

        Ok(())
    }

    /// Declare one synthesized default constructor and queue its prologue body.
    pub(in crate::lower) fn declare_default_constructor(
        &mut self,
        tree: &mut mir::Tree,
        class: dir::GlobalSymbolId,
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<()> {
        // find the materialized instance closing the class's types
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let specialization = self.specialization_of(class, None, &arguments);

        // key the constructor by the class's runtime representation
        let pointer_bytes = self.pointer_bytes;
        let lifetime_parameters = LifetimeParameters::default();
        let key = self
            .type_lowerer(tree, pointer_bytes, &lifetime_parameters)
            .with_instance(specialization)
            .generic_instance_key(class, None, &arguments)?;

        // skip a constructor whose representation is already declared
        if self.functions.contains_key(&key) {
            return Ok(());
        }

        // receive an exclusive borrow of the constructed storage
        let source = self.symbol_type(class)?;
        let nominal = self
            .type_lowerer(tree, pointer_bytes, &lifetime_parameters)
            .with_instance(specialization)
            .lower_nominal(source)?;
        let receiver_storage = nominal_receiver_storage(tree, nominal.value);
        let this = constructor_receiver_type(tree, nominal.storage, receiver_storage);

        // name the constructor after its class
        let void = tree.intern_type(mir::Type::Void);
        let name = format!("{}.constructor", self.symbol_path(class)?);

        // import the constructor a foreign class defines beside itself
        if class.module_id != self.module && arguments.is_empty() {
            let header =
                lifetime_parameters.declare(mir::FunctionHeaderBuilder::new(self.strings, &name));
            let header = header.parameters(vec![this]).result(void);
            let function = tree.insert(header.imported());
            self.functions
                .insert(key, FunctionDeclaration::Declared(function));

            return Ok(());
        }

        // define generic instances and own-class constructors locally
        let base = mir::Symbol::declared(self.strings.intern(&name), Self::symbol_identity(class));
        let instance = base.instantiate(&key.arguments, tree);
        let header = mir::FunctionHeaderBuilder::new(self.strings, &name)
            .arguments(key.arguments.iter().cloned())
            .symbol(instance);
        let header = lifetime_parameters.declare(header);
        let header = header.parameters(vec![this]).result(void);
        let function = tree.insert(header.declared());

        // record the declaration and queue its prologue body
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
    receiver_storage: mir::Storage,
) -> mir::LocalNodeId<mir::Type> {
    let pointee = tree.intern_type(mir::Type::Uninit { value: storage });

    tree.intern_type(mir::Type::Reference {
        kind: mir::ReferenceKind::Borrowed,
        lifetime: mir::Lifetime::empty(),
        storage: receiver_storage,
        access: mir::Access::Exclusive,
        pointee,
        nullability: mir::Nullability::None,
    })
}

/// Return the storage one nominal's constructor receiver borrows.
///
/// Reference nominals construct into their own allocation, while owned
/// nominals construct in place inside a frame.
pub(in crate::lower) fn nominal_receiver_storage(
    tree: &mir::Tree,
    value: mir::LocalNodeId<mir::Type>,
) -> mir::Storage {
    tree.get(value)
        .reference_storage()
        .unwrap_or(mir::Storage::Frame)
}
