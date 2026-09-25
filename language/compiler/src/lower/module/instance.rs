use tspp_artifact::DiagnosticLike;
use tspp_dir as dir;
use tspp_mir as mir;
use tspp_mir::substitute_type;

use crate::lower::{Body, FunctionDefinition, GenericScope, LowerPhase, ModuleLowerer};
use crate::{CompilerError, CompilerResult};

/// The declaration outcome behind one callable instance key.
pub(in crate::lower) enum FunctionDeclaration {
    /// The declared MIR function.
    Declared(mir::FunctionId),
    /// The failed declaration behind a reported diagnostic.
    Failed,
}

/// One callable a selection names: a declared specialization or an applied template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::lower) enum Instance {
    /// A declared function: closed, or the template itself.
    Declared(mir::FunctionId),
    /// A template applied at open arguments.
    Applied {
        /// The template.
        template: mir::FunctionId,
        /// The generic arguments, at least one mentioning a parameter.
        arguments: Vec<mir::GenericArgument>,
    },
}

impl Instance {
    /// Return the declared function, an applied template having none.
    pub(in crate::lower) fn function(&self) -> CompilerResult<mir::FunctionId> {
        match self {
            Self::Declared(function) => Ok(*function),
            Self::Applied { .. } => Err(CompilerError::Internal {
                message: "a template applied at open arguments outside a call".to_string(),
            }),
        }
    }
}

/// One concrete declaration instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::lower) struct GenericInstanceKey {
    /// The instantiated declaration.
    pub(in crate::lower) symbol: dir::GlobalSymbolId,
    /// The concrete receiver closing an interface member's `this`.
    pub(in crate::lower) receiver: Option<mir::GenericArgument>,
    /// The concrete generic arguments.
    pub(in crate::lower) arguments: Vec<mir::GenericArgument>,
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

impl ModuleLowerer<'_> {
    /// Declare the polymorphic function of every template this module declares, and queue its body.
    pub(in crate::lower) fn declare_templates(
        &mut self,
        tree: &mut mir::Tree,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<Vec<FunctionDefinition>> {
        let mut symbols: Vec<_> = self
            .state(self.module)?
            .generics
            .iter_templates()
            .filter_map(|(_, template)| template.symbol)
            .collect();
        for (_, definition) in self.local().definitions.iter_definitions() {
            for member in definition.members() {
                if let dir::DefinitionMember::Method(method) = member {
                    symbols.push(method.symbol);
                }
            }
        }
        let mut definitions = Vec::new();
        for symbol in symbols {
            // skip nominals, closures, and derived members, declared where they are reached
            if symbol.module_id != self.module
                || self.definition(symbol)?.is_some()
                || !self.is_declared_callable(symbol)?
                || self.is_derived_member(symbol)?
            {
                continue;
            }
            match self.declare_template(tree, symbol) {
                Ok(Some(body)) => definitions.push(body),
                Ok(None) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.bank_failed_callable(Some(symbol), diagnostic, errors);
                }
                Err(error) => return Err(error),
            }
        }

        Ok(definitions)
    }

    /// Return whether one symbol declares a callable outside a body: a closure declares inside one.
    pub(in crate::lower) fn is_declared_callable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let state = self.state(symbol.module_id)?;
        let declared = state.bindings.get_symbol(symbol.local_id);
        let kind = state.bindings.get_scope_by_id(declared.scope.id).kind;

        Ok(declared.kind == dir::SymbolKind::Function
            && !matches!(kind, dir::ScopeKind::Function | dir::ScopeKind::Block))
    }

    /// Declare one template's polymorphic function, its body queued.
    fn declare_template(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<FunctionDefinition>> {
        let key = GenericInstanceKey::non_generic(symbol);
        if self.functions.contains_key(&key) {
            return Ok(None);
        }
        let scope = self.symbol_scope(symbol)?;
        if scope.count() == 0 {
            return Ok(None);
        }

        // read headers from the declared stage and bodies from the lowering
        if self.phase == LowerPhase::Lower && self.callable_header(symbol)?.body.is_none() {
            return Ok(None);
        }

        self.declare_callable(tree, &key, &GenericScope::default().erased())
    }

    /// Declare the instance one key applies: the template's header at the key's arguments.
    pub(in crate::lower) fn declare_instance(
        &mut self,
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        symbol: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        bindings: &[dir::GenericArgumentBinding],
        dependents: &[dir::GlobalTypeId],
        enclosing: &GenericScope,
    ) -> CompilerResult<Instance> {
        if let Some(FunctionDeclaration::Declared(function)) = self.functions.get(key) {
            return Ok(Instance::Declared(*function));
        }

        // read the template parameters the symbol declares
        let chain = self.symbol_scope(symbol)?;

        // return the declared function of a closed callable the walk or another module declared
        if chain.count() == 0 && key.arguments.is_empty() {
            let declared = GenericInstanceKey::non_generic(symbol);
            if self.functions.contains_key(&declared) || symbol.module_id != self.module {
                return self
                    .template_function(tree, symbol, receiver)
                    .map(Instance::Declared);
            }

            // declare a derived callable under its receiver instance name
            if let Some(definition) = self.declare_callable(tree, key, enclosing)? {
                self.pending.push(definition);
            }
            return match self.functions.get(key) {
                Some(FunctionDeclaration::Declared(function)) => Ok(Instance::Declared(*function)),
                _ => Err(CompilerError::Internal {
                    message: "a closed instance without a declared function".to_string(),
                }),
            };
        }

        // read the template and require the argument each of its places takes
        let template = self.template_function(tree, symbol, receiver)?;
        let placed = self.instance_arguments(
            tree, template, symbol, receiver, bindings, dependents, &chain,
        )?;
        let mut arguments = Vec::with_capacity(placed.len());
        for (index, argument) in placed.into_iter().enumerate() {
            let Some(argument) = argument else {
                return Err(self.unplaced_argument(symbol, &chain, index as u32)?);
            };
            arguments.push(argument);
        }

        // lower the arguments in the enclosing parameter space, noting open ones
        let mut lower = self.type_lowerer(tree, enclosing);
        let mut lowered = Vec::with_capacity(arguments.len());
        let mut is_open = false;
        for argument in arguments {
            is_open |= lower.lower.is_open_argument(argument)?;
            lowered.push(lower.lower_generic_argument(argument)?);
        }

        // apply the template in place when an argument stays open in the enclosing template
        if is_open {
            return Ok(Instance::Applied {
                template,
                arguments: lowered,
            });
        }

        self.declare_specialization(tree, key, symbol, template, lowered, &chain)
            .map(Instance::Declared)
    }

    /// Place the argument each template parameter and dependent takes, none where unbound.
    pub(in crate::lower) fn instance_arguments(
        &mut self,
        tree: &mir::Tree,
        template: mir::FunctionId,
        symbol: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        bindings: &[dir::GenericArgumentBinding],
        dependents: &[dir::GlobalTypeId],
        chain: &GenericScope,
    ) -> CompilerResult<Vec<Option<dir::GlobalTypeId>>> {
        // require one place per template parameter
        let slots = tree.get(template).generics.len();
        if chain.count() as usize != slots {
            return Err(CompilerError::Internal {
                message: format!(
                    "an instantiation of '{}' with {} places for {slots} slots",
                    self.symbol_path(symbol)?,
                    chain.count()
                ),
            });
        }

        // place each given dependent value
        let mut arguments = vec![None; chain.count() as usize];
        for (dependent, value) in chain.dependents.values().zip(dependents) {
            arguments[dependent.index as usize] = Some(*value);
        }

        // place each bound parameter, the receiver parameter defaulting to the receiver
        for (parameter, index) in &chain.parameters {
            let binding = self
                .state(parameter.module_id)?
                .generics
                .get_parameter(parameter.local_id);
            let is_receiver = binding.origin == dir::GenericParameterOrigin::Receiver;
            arguments[*index as usize] = bindings
                .iter()
                .find(|binding| binding.parameter == *parameter)
                .map(|binding| binding.argument)
                .or(if is_receiver { receiver } else { None });
        }

        Ok(arguments)
    }

    /// Return the error for one instantiation leaving a parameter or dependent unplaced.
    pub(in crate::lower) fn unplaced_argument(
        &mut self,
        symbol: dir::GlobalSymbolId,
        chain: &GenericScope,
        index: u32,
    ) -> CompilerResult<CompilerError> {
        let path = self.symbol_path(symbol)?;

        // name the parameter at the index
        let parameter = chain
            .parameters
            .iter()
            .find(|(_, parameter_index)| **parameter_index == index)
            .map(|(parameter, _)| *parameter);
        if let Some(parameter) = parameter {
            let name = self.format_parameter_name(parameter)?;

            return Ok(CompilerError::Internal {
                message: format!(
                    "an instantiation of '{path}' without a binding for its parameter '{name}'"
                ),
            });
        }

        // otherwise name the dependent at the index
        let Some(dependent) = chain
            .dependents
            .values()
            .find(|dependent| dependent.index == index)
        else {
            return Err(CompilerError::Internal {
                message: format!("an instantiation of '{path}' without a place at index {index}"),
            });
        };
        let dependent = self.ty(dependent.ty)?;

        Ok(CompilerError::Internal {
            message: format!(
                "an instantiation of '{path}' without a value for its dependent {dependent:?}"
            ),
        })
    }

    /// Return whether one generic argument names a parameter of its enclosing template.
    pub(in crate::lower) fn is_open_argument(
        &mut self,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let flags = self
            .types(argument.module_id)?
            .get_type_flags(argument.local_id);

        Ok(flags.has_type_parameter() || flags.has_this())
    }

    /// Declare the specialization one key names: the template's header at closed arguments.
    pub(in crate::lower) fn declare_specialization(
        &mut self,
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        symbol: dir::GlobalSymbolId,
        template: mir::FunctionId,
        lowered: Vec<mir::GenericArgument>,
        chain: &GenericScope,
    ) -> CompilerResult<mir::FunctionId> {
        if let Some(FunctionDeclaration::Declared(function)) = self.functions.get(key) {
            return Ok(*function);
        }

        // declare the specialization with the template's header at the arguments
        let declared = tree.get(template).clone();
        let parameters: Vec<_> = declared
            .parameters
            .iter()
            .map(|parameter| substitute_type(tree, parameter.ty, &lowered))
            .collect();
        let result = substitute_type(tree, declared.return_type, &lowered);
        let name = self.callable_path(symbol)?;

        // declare a binder for every position the arguments erase a region to
        let mut chain = chain.clone();
        for argument in &lowered {
            let mir::GenericArgument::Region(lifetime) = argument else {
                continue;
            };
            for index in lifetime.bound_indices() {
                while chain.names.len() <= index as usize {
                    chain.names.push(format!("'l{}", chain.names.len()));
                }
            }
        }

        // keep the template's lifetime parameters on its specialization, the template linked
        let function = self.insert_header(
            tree,
            key,
            &name,
            parameters,
            result,
            &chain,
            mir::Linkage::Shared,
            None,
            declared.kind,
        )?;
        tree.get_mut(function).template = Some(template);

        Ok(function)
    }

    /// Return whether one class declares instance fields with initializers.
    pub(in crate::lower) fn class_has_field_initializers(
        &mut self,
        class: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let Some(definition) = self.definition(class)?.cloned() else {
            return Ok(false);
        };

        Ok(self
            .instance_fields(definition.members())?
            .iter()
            .any(|field| field.initializer.is_some()))
    }

    /// Lower the receiver type one instantiated signature declares.
    pub(in crate::lower) fn declared_receiver_type(
        &mut self,
        tree: &mut mir::Tree,
        declared: dir::GlobalTypeId,
        scope: &GenericScope,
    ) -> CompilerResult<mir::TypeId> {
        let (signature, module) = self.signature(declared)?;
        let Some(this) = self.types(module)?.signature(signature).this_parameter else {
            return Err(CompilerError::Internal {
                message: "an instance method without a receiver".to_string(),
            });
        };

        self.type_lowerer(tree, scope).lower(this)
    }

    /// Split one coroutine definition into its entry and extracted body forms.
    pub(in crate::lower) fn split_coroutine_definition(
        &mut self,
        tree: &mut mir::Tree,
        key: Option<&GenericInstanceKey>,
        definition: FunctionDefinition,
    ) -> CompilerResult<FunctionDefinition> {
        // keep plain bodies whole
        let declared = self.symbol_type(definition.symbol)?;
        let (signature_id, owner) = self.signature(declared)?;
        let dir_signature = self.types(owner)?.signature(signature_id);
        let is_coroutine =
            dir_signature.asynchrony != dir::Asynchrony::Sync || dir_signature.is_generator;
        let Body::Plain(expression) = definition.body else {
            return Ok(definition);
        };
        if !is_coroutine {
            return Ok(definition);
        }

        // declare and queue the extracted body beside the entry
        let body = self.declare_coroutine_body(tree, definition.symbol, key, &definition.scope)?;
        self.pending.push(FunctionDefinition {
            function: body,
            symbol: definition.symbol,
            has_this: false,
            parameters: definition.parameters.clone(),
            scope: definition.scope.clone(),
            source: definition.source,
            constructs: None,
            defaults: vec![None; definition.parameters.len()],
            body: Body::Coroutine(expression),
        });

        Ok(FunctionDefinition {
            body: Body::CoroutineEntry { expression, body },
            ..definition
        })
    }

    /// Declare the synthesized constructors of this module's own classes.
    pub(in crate::lower) fn declare_default_constructors(
        &mut self,
        tree: &mut mir::Tree,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
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
            if !self.class_has_field_initializers(symbol)? {
                continue;
            }

            // collect a declaration diagnostic and keep declaring the rest
            let declared = self.declare_default_constructor(
                tree,
                symbol,
                &[],
                &GenericScope::default().erased(),
            );
            match declared {
                Ok(_) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    /// Declare one class's synthesized constructor, a generic class's at its template.
    pub(in crate::lower) fn declare_default_constructor(
        &mut self,
        tree: &mut mir::Tree,
        class: dir::GlobalSymbolId,
        bindings: &[dir::GenericArgumentBinding],
        enclosing: &GenericScope,
    ) -> CompilerResult<Instance> {
        // key the constructor by the class's runtime representation in the enclosing space
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let key = self
            .type_lowerer(tree, enclosing)
            .generic_instance_key(class, None, &arguments)?;
        if let Some(FunctionDeclaration::Declared(function)) = self.functions.get(&key) {
            return Ok(Instance::Declared(*function));
        }

        // read the class template, a generic class's constructor being a template of its own
        let template = self
            .definition(class)?
            .and_then(|definition| definition.template())
            .map(|template| template.into_global(class.module_id));
        let void = tree.intern_type(mir::Type::Void);
        let name = format!("{}.constructor", self.symbol_path(class)?);

        // apply a generic class's constructor template at the arguments
        if template.is_some() && !arguments.is_empty() {
            let constructor = self
                .declare_default_constructor(tree, class, &[], &GenericScope::default().erased())?
                .function()?;
            let mut lower = self.type_lowerer(tree, enclosing);
            let mut lowered = Vec::with_capacity(arguments.len());
            let mut is_open = false;
            for argument in &arguments {
                is_open |= lower.lower.is_open_argument(*argument)?;
                lowered.push(lower.lower_generic_argument(*argument)?);
            }
            // apply the template in place when an argument stays open in the enclosing template
            if is_open {
                return Ok(Instance::Applied {
                    template: constructor,
                    arguments: lowered,
                });
            }
            let parameters: Vec<_> = tree
                .get(constructor)
                .parameters
                .iter()
                .map(|parameter| parameter.ty)
                .collect();
            let parameters: Vec<_> = parameters
                .into_iter()
                .map(|parameter| substitute_type(tree, parameter, &lowered))
                .collect();
            let scope = self.class_constructor_scope(class)?;
            let function = self.insert_header(
                tree,
                &key,
                &name,
                parameters,
                void,
                &scope,
                mir::Linkage::Shared,
                None,
                mir::FunctionKind::Constructor,
            )?;

            return Ok(Instance::Declared(function));
        }

        // receive an exclusive borrow of the constructed storage at the receiver slot
        let scope = self.class_constructor_scope(class)?;
        let Some(slot) = scope.receiver_slot else {
            unreachable!("a constructor scope without its receiver slot");
        };
        let source = self.symbol_type(class)?;
        let nominal = self.type_lowerer(tree, &scope).lower_nominal(source)?;
        let this =
            constructor_receiver_type(tree, nominal.storage, mir::Lifetime::bound(slot.index));

        // import a foreign class's constructor, define an own class's and queue its prologue
        if class.module_id != self.module {
            return Ok(Instance::Declared(self.import_header(tree, &key, &name)?));
        }
        let function = self.insert_header(
            tree,
            &key,
            &name,
            vec![this],
            void,
            &scope,
            mir::Linkage::Local,
            None,
            mir::FunctionKind::Constructor,
        )?;
        self.pending.push(FunctionDefinition {
            function,
            symbol: class,
            has_this: true,
            parameters: Vec::new(),
            scope,
            source: class.module_id,
            constructs: None,
            defaults: Vec::new(),
            body: Body::DefaultConstructor,
        });

        Ok(Instance::Declared(function))
    }
}

/// Intern one constructor receiver: an exclusive borrow of the uninitialized constructed storage.
fn constructor_receiver_type(
    tree: &mut mir::Tree,
    storage: mir::TypeId,
    lifetime: mir::Lifetime,
) -> mir::TypeId {
    let pointee = tree.intern_type(mir::Type::Uninit { value: storage });

    tree.intern_type(mir::Type::Reference {
        kind: mir::Reference::Borrowed,
        lifetime,
        access: mir::Access::Exclusive,
        pointee,
    })
}
