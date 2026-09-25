use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{
    Body, BoundReceiver, CallableImplementation, FunctionDeclaration, FunctionDefinition,
    GenericInstanceKey, GenericScope, ModuleLowerer,
};
use crate::{CompilerError, CompilerResult};

/// The receiver one callable leads with.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::lower) enum Receiver {
    /// No receiver.
    None,
    /// The declared this parameter.
    This(dir::GlobalTypeId),
    /// The receiver parameter of an interface method with an implicit receiver.
    Erased,
    /// The declared receiver term of a constructor, borrowing its storage uninitialized.
    Constructs(dir::GlobalTypeId),
}

/// One callable as its declaration writes it.
pub(in crate::lower) struct CallableHeader {
    /// The nominal owning a member callable.
    pub(in crate::lower) owner: Option<dir::GlobalSymbolId>,
    /// The declared member role.
    pub(in crate::lower) role: Option<dir::FunctionRole>,
    /// The receiver the callable leads with.
    pub(in crate::lower) receiver: Receiver,
    /// The parameter symbols in header order.
    pub(in crate::lower) parameters: Vec<dir::LocalSymbolId>,
    /// The declared default expression of each parameter, in header order.
    pub(in crate::lower) defaults: Vec<Option<dir::LocalNodeId<dir::Expression>>>,
    /// The body, absent on an ambient signature or a requirement.
    pub(in crate::lower) body: Option<dir::LocalNodeId<dir::Expression>>,
}

impl ModuleLowerer<'_> {
    /// Read one callable off its declaration.
    pub(in crate::lower) fn callable_header(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<CallableHeader> {
        let member = self.imported_member(symbol)?;
        let state = self.state(symbol.module_id)?;
        let Some(node) = state.bindings.get_symbol(symbol.local_id).declaration else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a callable '{}' without a declaration",
                    self.symbol_path(symbol)?
                ),
            });
        };

        // read the signature nodes and the body by the declaration form
        let tree = state.tree();
        let (parameters, body, is_static, is_member) =
            if let Ok(declaration) = node.local_id.try_into_typed::<dir::Declaration>() {
                let dir::Declaration::Function(function) = tree.get(declaration) else {
                    return Err(CompilerError::Internal {
                        message: "a callable declared outside a function".to_string(),
                    });
                };

                (
                    function.signature.parameters.to_vec(),
                    function.body,
                    false,
                    false,
                )
            } else if let Ok(member) = node.local_id.try_into_typed::<dir::Member>() {
                let dir::Member::Method {
                    signature,
                    body,
                    is_static,
                    ..
                } = tree.get(member)
                else {
                    return Err(CompilerError::Internal {
                        message: "a callable declared outside a method".to_string(),
                    });
                };

                (signature.parameters.to_vec(), *body, *is_static, true)
            } else if let Ok(member) = node.local_id.try_into_typed::<dir::TypeMember>() {
                let dir::TypeMember::Method {
                    signature,
                    body,
                    is_static,
                    ..
                } = tree.get(member)
                else {
                    return Err(CompilerError::Internal {
                        message: "a callable declared outside an interface method".to_string(),
                    });
                };

                (signature.parameters.to_vec(), *body, *is_static, true)
            } else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "a callable '{}' declaring through {:?}",
                        self.symbol_path(symbol)?,
                        node.local_id
                    ),
                });
            };

        // collect each parameter's symbol and default
        let mut symbols = Vec::with_capacity(parameters.len());
        let defaults: Vec<_> = parameters
            .iter()
            .map(|parameter| tree.get(*parameter).default_value())
            .collect();
        for parameter in parameters {
            let node = parameter.into_global_any(symbol.module_id);
            let Some(parameter) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "a missing symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter.local_id);
        }

        // classify the receiver the callable leads with
        let owner = member.as_ref().map(|member| member.owner);
        let role = member.as_ref().and_then(|member| member.role);
        let is_static = is_static || member.as_ref().is_some_and(|member| member.is_static);
        let receiver = if is_member && !is_static {
            let declared = self.symbol_type(symbol)?;
            let (signature, module) = self.signature(declared)?;
            let this = self.types(module)?.signature(signature).this_parameter;
            let is_interface = match owner {
                Some(owner) => {
                    matches!(self.definition(owner)?, Some(dir::Definition::Interface(_)))
                }
                None => false,
            };

            match (role, this) {
                (Some(dir::FunctionRole::Constructor), Some(this)) => Receiver::Constructs(this),
                (_, Some(this)) => Receiver::This(this),
                (_, None) if is_interface => Receiver::Erased,
                (_, None) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "an instance method '{}' without a receiver",
                            self.symbol_path(symbol)?
                        ),
                    });
                }
            }
        } else {
            Receiver::None
        };

        Ok(CallableHeader {
            owner,
            role,
            receiver,
            parameters: symbols,
            defaults,
            body,
        })
    }

    /// Lower the receiver one callable leads with.
    pub(in crate::lower) fn lower_receiver(
        &mut self,
        tree: &mut mir::Tree,
        receiver: Receiver,
        scope: &GenericScope,
    ) -> CompilerResult<Option<mir::TypeId>> {
        match receiver {
            Receiver::None => Ok(None),
            Receiver::This(this) => Ok(Some(self.type_lowerer(tree, scope).lower(this)?)),
            // take the interface receiver parameter the scope indexes
            Receiver::Erased => {
                let Some(index) = scope.receiver else {
                    return Err(CompilerError::Internal {
                        message: "an erased receiver outside an interface scope".to_string(),
                    });
                };
                let receiver = mir::Type::Parameter {
                    index,
                    referent: false,
                };

                Ok(Some(tree.intern_type(receiver)))
            }
            // borrow the constructed storage uninitialized at the declared receiver term
            Receiver::Constructs(this) => {
                let reference = self.type_lowerer(tree, scope).lower(this)?;
                let mut uninit = tree.type_definition(reference).clone();
                let mir::Type::Reference { pointee, .. } = &mut uninit else {
                    return Err(CompilerError::Internal {
                        message: "a constructor receiver outside a reference".to_string(),
                    });
                };
                *pointee = tree.intern_type(mir::Type::Uninit { value: *pointee });

                Ok(Some(tree.intern_type(uninit)))
            }
        }
    }

    /// Declare one callable's header under its template parameters, queueing an own body.
    pub(in crate::lower) fn declare_callable(
        &mut self,
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        enclosing: &GenericScope,
    ) -> CompilerResult<Option<FunctionDefinition>> {
        let symbol = key.symbol;
        let header = self.callable_header(symbol)?;
        let declared = self.symbol_type(symbol)?;
        let scope = &if self.is_declared_callable(symbol)? {
            self.symbol_scope(symbol)?
        } else {
            self.nested_symbol_scope(symbol, Some(enclosing))?
                .with_parameters_of(enclosing, self, tree)?
        };

        // a binding declares a host extern at the host's calling convention
        let binding = match self.callable_implementation(symbol)? {
            Some(CallableImplementation::Binding { binding }) => Some(binding),
            Some(CallableImplementation::Intrinsic { .. }) => return Ok(None),
            None => None,
        };

        // name the header by the binding's extern name, else the callable's canonical path
        let name = match &binding {
            Some(binding) => self.strings.get(binding.name).to_string(),
            None => self.callable_path(symbol)?,
        };

        // import a foreign callable's header under its instance key
        if symbol.module_id != self.module {
            self.import_header(tree, key, &name)?;

            return Ok(None);
        }
        let (mut parameters, mut result) = self.lower_signature(tree, declared, scope)?;
        if parameters.len() != header.parameters.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "the parameters of '{}' disagree with its declared signature",
                    self.symbol_path(symbol)?
                ),
            });
        }

        // lead with the receiver, a constructor answering void
        if let Some(this) = self.lower_receiver(tree, header.receiver, scope)? {
            parameters.insert(0, this);
        }
        if header.role == Some(dir::FunctionRole::Constructor) {
            result = tree.intern_type(mir::Type::Void);
        }

        let is_defined = header.body.is_some() && symbol.module_id == self.module;
        let linkage = if is_defined {
            mir::Linkage::Local
        } else {
            mir::Linkage::Import
        };
        let kind = match header.role {
            Some(dir::FunctionRole::Constructor) => mir::FunctionKind::Constructor,
            _ => mir::FunctionKind::Function,
        };
        let function = self.insert_header(
            tree, key, &name, parameters, result, scope, linkage, binding, kind,
        )?;

        // queue the body this module defines
        let (Some(body), true) = (header.body, is_defined) else {
            return Ok(None);
        };
        let definition = FunctionDefinition {
            function,
            symbol,
            has_this: header.receiver != Receiver::None,
            parameters: header.parameters,
            scope: scope.clone(),
            source: symbol.module_id,
            constructs: match header.role {
                Some(dir::FunctionRole::Constructor) => header.owner,
                _ => None,
            },
            defaults: header.defaults,
            body: Body::Plain(body),
        };

        self.split_coroutine_definition(tree, Some(key), definition)
            .map(Some)
    }

    /// Import one foreign callable's header under the instance key its call sites name.
    pub(in crate::lower) fn import_header(
        &mut self,
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        name: &str,
    ) -> CompilerResult<mir::FunctionId> {
        let symbol = key.symbol;
        let base = self.declared_symbol(symbol, name);
        let mut function = self.import_function(tree, symbol.module_id, base, name)?;
        let display = Self::key_display(key);
        if !display.is_empty() {
            function.symbol = base.instantiate(&display, tree);
            function.arguments = display;
        }
        let function = tree.insert(function);
        self.declare_function(tree, key, function)?;

        Ok(function)
    }

    /// Insert one header under its instance key, recording it for the call sites.
    pub(in crate::lower) fn insert_header(
        &mut self,
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        name: &str,
        parameters: Vec<mir::TypeId>,
        result: mir::TypeId,
        scope: &GenericScope,
        linkage: mir::Linkage,
        binding: Option<mir::Binding>,
        kind: mir::FunctionKind,
    ) -> CompilerResult<mir::FunctionId> {
        let symbol = key.symbol;
        let display = Self::key_display(key);

        // keep a template polymorphic over its type parameters, a specialization closed
        let receiver = if self.definition(symbol)?.is_none() {
            BoundReceiver::OfCallable(symbol)
        } else {
            BoundReceiver::None
        };
        let generics = match linkage {
            mir::Linkage::Shared => Vec::new(),
            _ => self.generic_parameters(tree, scope, receiver)?,
        };

        // declare the header under its declared or instantiated symbol
        let base = self.declared_symbol(symbol, name);
        let instantiated = if display.is_empty() {
            base
        } else {
            base.instantiate(&display, tree)
        };
        let header = mir::FunctionHeaderBuilder::new(self.strings, self.module, name)
            .kind(kind)
            .generics(generics)
            .arguments(display)
            .symbol(instantiated);
        let header = scope.declare(header);
        let header = header.parameters(parameters).result(result);
        let function = match (linkage, binding) {
            (_, Some(binding)) => header.imported().with_binding(binding),
            (mir::Linkage::Import, None) => header.imported(),
            (mir::Linkage::Local | mir::Linkage::Export, None) => header.declared(),
            (mir::Linkage::Shared, None) => {
                let mut declared = header.declared();
                declared.linkage = mir::Linkage::Shared;

                declared
            }
        };
        let function = tree.insert(function);
        self.declare_function(tree, key, function)?;

        Ok(function)
    }

    /// Return the closed receiver and type arguments one key names, the receiver first.
    fn key_display(key: &GenericInstanceKey) -> Vec<mir::GenericArgument> {
        let mut display = key.arguments.clone();
        if let Some(receiver) = &key.receiver {
            display.insert(0, receiver.clone());
        }

        display
    }

    /// Return the declared symbol of one callable under a name.
    fn declared_symbol(&self, symbol: dir::GlobalSymbolId, name: &str) -> mir::Symbol {
        mir::Symbol::declared(
            symbol.module_id,
            self.strings.intern(name),
            Self::symbol_identity(symbol),
        )
    }

    /// Anchor, index, and record one declared function under its key.
    fn declare_function(
        &mut self,
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        function: mir::FunctionId,
    ) -> CompilerResult<()> {
        self.anchor_function(tree, function, key.symbol)?;
        self.index_language_declaration(tree, function, key.symbol);
        self.functions
            .insert(key.clone(), FunctionDeclaration::Declared(function));

        Ok(())
    }

    /// Declare the extracted body function one coroutine wraps in its creation call.
    pub(in crate::lower) fn declare_coroutine_body(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
        key: Option<&GenericInstanceKey>,
        scope: &GenericScope,
    ) -> CompilerResult<mir::FunctionId> {
        // read the recorded creation call's closure slot for the body signature
        let state = self.state(symbol.module_id)?;
        let node = state.bindings.get_symbol(symbol.local_id).declaration;
        let Some(create) = node.and_then(|node| state.decisions.call_decision(node).cloned())
        else {
            return Err(CompilerError::Internal {
                message: "a coroutine body without its recorded creation call".to_string(),
            });
        };
        let dir::OperationResolution::One(create) = create else {
            return Err(CompilerError::Internal {
                message: "a coroutine creation call on a union receiver".to_string(),
            });
        };
        let Some(binding) = create
            .arguments
            .iter()
            .find(|binding| matches!(binding.source, dir::ArgumentSource::Supplied(_)))
        else {
            return Err(CompilerError::Internal {
                message: "a creation call without its body closure slot".to_string(),
            });
        };
        let closure = binding.argument_type;

        // read the closure's lowered signature as the extracted body's own
        let representation = self.type_lowerer(tree, scope).lower(closure)?;
        let mut callable = representation;
        while let mir::Type::Newtype { value, .. } = tree.get(callable) {
            callable = *value;
        }
        let mir::Type::Function { signature, .. } = tree.get(callable) else {
            return Err(CompilerError::Internal {
                message: "a coroutine closure outside a callable representation".to_string(),
            });
        };
        let mir::Type::FunctionSignature {
            parameters, result, ..
        } = tree.get(*signature)
        else {
            return Err(CompilerError::Internal {
                message: "a coroutine closure without its signature".to_string(),
            });
        };
        let parameters: Vec<_> = parameters.iter().map(|parameter| parameter.ty).collect();
        let result = *result;

        // declare the body beside the entry under its own key
        let name = format!("{}.body", self.symbol_path(symbol)?);
        let body_key = GenericInstanceKey {
            symbol,
            receiver: None,
            arguments: key.map(|key| key.arguments.clone()).unwrap_or_default(),
        };
        let base = self.declared_symbol(symbol, &name);
        let mut header =
            mir::FunctionHeaderBuilder::new(self.strings, self.module, &name).symbol(base);
        if !body_key.arguments.is_empty() {
            let display = body_key.arguments.clone();
            let instantiated = base.instantiate(&display, tree);
            header = header.arguments(display).symbol(instantiated);
        }
        let generics = self.generic_parameters(tree, scope, BoundReceiver::OfCallable(symbol))?;
        let header = scope.declare(header);
        let header = header
            .generics(generics)
            .parameters(parameters)
            .result(result);
        let function = tree.insert(header.declared());
        self.anchor_function(tree, function, symbol)?;

        Ok(function)
    }

    /// Anchor one function at its symbol's declaration extent.
    pub(in crate::lower) fn anchor_function(
        &mut self,
        tree: &mut mir::Tree,
        function: mir::FunctionId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if let Some((node, span)) = self.declaration_anchor(symbol)? {
            tree.set_source(function.id, node);
            tree.set_span(function, span);
        }

        Ok(())
    }

    /// Return the non-lifetime template parameters behind one callable type.
    pub(in crate::lower) fn signature_type_parameters(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GlobalGenericParameterId>> {
        // peel the callable down to its signature template
        let (signature, owner) = self.signature(ty)?;
        let signature = *self.types(owner)?.signature(signature);
        let Some(template) = signature.template else {
            return Ok(Vec::new());
        };
        let arguments = self
            .types(owner)?
            .generic_arguments(signature.arguments)
            .to_vec();

        // keep the parameters an instance key selects
        let template_module = template.module_id;
        let generics = &self.state(template_module)?.generics;
        let template = generics.get_template(template.local_id);
        let mut parameters = Vec::new();
        for parameter in &template.parameters {
            let binding = generics.get_parameter(*parameter);
            let parameter = parameter.into_global(template_module);
            if binding.memory_parameter() == Some(dir::MemoryParameter::Region)
                || arguments
                    .iter()
                    .any(|binding| binding.parameter == parameter)
            {
                continue;
            }
            parameters.push(parameter);
        }

        Ok(parameters)
    }

    /// Return the bindings one written selection closes at, the allocated instance's over its own.
    pub(in crate::lower) fn selection_bindings(
        &mut self,
        selection: &dir::InstanceKey,
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let generics = &self.state(self.module)?.generics;
        let arguments = match generics.selection_instance(selection) {
            Some(instance) => generics.get_instance(instance).key.arguments.clone(),
            None => selection.arguments.clone(),
        };

        self.instance_bindings(&arguments)
    }

    /// Return the dependents one selection's instance evaluated, owner templates first.
    pub(in crate::lower) fn selection_dependents(
        &mut self,
        selection: &dir::InstanceKey,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let generics = &self.state(self.module)?.generics;

        Ok(match generics.selection_instance(selection) {
            Some(instance) => generics.get_instance(instance).key.dependents.clone(),
            None => selection.dependents.clone(),
        })
    }

    /// Return the type and memory bindings one selection holds, signature lifetimes dropped.
    pub(in crate::lower) fn instance_bindings(
        &mut self,
        generic_arguments: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let mut bindings = Vec::new();
        for binding in generic_arguments {
            let parameter = binding.parameter;
            let generics = &self.state(parameter.module_id)?.generics;
            let declared = generics.get_parameter(parameter.local_id);
            if declared.is_instance_parameter()
                || self.is_declaration_region_parameter(parameter)?
            {
                bindings.push(*binding);
            }
        }

        Ok(bindings)
    }

    /// Return the template parameters one callable lowers under, owner parameters first.
    pub(in crate::lower) fn callable_scope(
        &mut self,
        symbol: dir::GlobalSymbolId,
        owner: Option<dir::GlobalSymbolId>,
        is_static: bool,
        enclosing: Option<&GenericScope>,
    ) -> CompilerResult<GenericScope> {
        // peel the callable down to its signature template, else the declaration's own
        let ty = self.symbol_type(symbol)?;
        let (signature, module) = self.signature(ty)?;
        let signature = match self.types(module)?.signature(signature).template {
            Some(template) => Some(template),
            None => self
                .state(symbol.module_id)?
                .generics
                .template_by_symbol(symbol)
                .map(|template| template.into_global(symbol.module_id)),
        };
        let signature = self.signature_template(signature, enclosing)?;

        // read the owner's template
        let mut owner_template = None;
        if let Some(owner) = owner
            && let Some(definition) = self.definition(owner)?
            && !(is_static
                && !matches!(
                    definition,
                    dir::Definition::Extension(_) | dir::Definition::Interface(_)
                ))
        {
            owner_template = definition
                .template()
                .map(|template| template.into_global(owner.module_id));
        }

        // index the dependents a declared signature writes
        let mut parameters = GenericScope::from_templates(self, owner_template, signature)?;
        if signature.is_none() && self.is_declared_callable(symbol)? {
            parameters.collect_dependents(self, symbol)?;
        }

        // name the receiver an instance member leads with, an extension's this its target
        if !is_static {
            let (signature, module) = self.signature(ty)?;
            parameters.this_parameter = self.types(module)?.signature(signature).this_parameter;
        }
        parameters.extension_target = self.bound_this(symbol)?;

        Ok(parameters)
    }

    /// Return one signature template unless the enclosing scope binds its regions.
    pub(in crate::lower) fn signature_template(
        &mut self,
        template: Option<dir::GlobalGenericTemplateId>,
        enclosing: Option<&GenericScope>,
    ) -> CompilerResult<Option<dir::GlobalGenericTemplateId>> {
        let (Some(template), Some(enclosing)) = (template, enclosing) else {
            return Ok(template);
        };

        // drop a template whose regions the enclosing scope already binds
        let generics = &self.state(template.module_id)?.generics;
        let regions: Vec<_> = generics
            .get_template(template.local_id)
            .parameters
            .iter()
            .filter(|parameter| {
                generics.get_parameter(**parameter).memory_parameter()
                    == Some(dir::MemoryParameter::Region)
            })
            .map(|parameter| parameter.into_global(template.module_id))
            .collect();
        let is_bound = !regions.is_empty()
            && regions
                .iter()
                .all(|region| enclosing.slots.contains_key(region));

        if is_bound {
            Ok(None)
        } else {
            Ok(Some(template))
        }
    }

    /// Return the method symbol declared at one member node.
    pub(in crate::lower) fn method_symbol(
        &mut self,
        owner: dir::GlobalSymbolId,
        member: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let Some(definition) = self.definition(owner)?.cloned() else {
            return Ok(None);
        };

        Ok(definition.method_declared_at(member))
    }

    /// Return one member's owner-qualified extern name.
    pub(in crate::lower) fn member_extern_name(
        &mut self,
        symbol: dir::GlobalSymbolId,
        owner: dir::GlobalSymbolId,
        role: Option<dir::FunctionRole>,
    ) -> CompilerResult<String> {
        // qualify an anonymous extension by its target root or covered interface
        let (extension_root, implemented) = match self.definition(owner)?.cloned() {
            Some(dir::Definition::Extension(extension)) => {
                let root = extension
                    .target
                    .declaration()
                    .or(match extension.target.coverage() {
                        dir::BlanketCoverage::Interface(interface) => Some(interface),
                        _ => None,
                    });
                let mut implemented = None;
                for conformance in &extension.implements {
                    let selected = self
                        .state(owner.module_id)?
                        .members
                        .conformance_members(conformance.source);
                    let implements = selected.is_some_and(|selected| {
                        selected.iter().any(|selected| selected.member == symbol)
                    });
                    if implements {
                        implemented = self.ty(conformance.interface)?.symbol();
                        break;
                    }
                }

                (root, implemented)
            }
            _ => (None, None),
        };

        // resolve the owner name, falling back to the extension root
        let mut owner_name = self.symbol_name(owner)?;
        if owner_name.is_none()
            && let Some(root) = extension_root
        {
            owner_name = self.symbol_name(root)?;
        }
        // name uncovered blanket extension members by their lexical path
        let Some(owner_name) = owner_name else {
            return self.symbol_path(symbol);
        };
        let mut owner_name = self.strings.get(owner_name).to_string();
        if let Some(interface) = implemented
            && let Some(interface) = self.symbol_name(interface)?
        {
            owner_name = format!("{owner_name}.{}", self.strings.get(interface));
        }

        // resolve the member name, naming constructors after their role
        let member_name = match self.symbol_name(symbol)? {
            Some(name) => self.strings.get(name).to_string(),
            None if role == Some(dir::FunctionRole::Constructor) => "constructor".to_string(),
            None => {
                return Err(CompilerError::Internal {
                    message: "a method without a name".to_string(),
                });
            }
        };

        // split the accessor pair that shares one member name by role
        let member_name = match role {
            Some(dir::FunctionRole::Getter) => format!("{member_name}.get"),
            Some(dir::FunctionRole::Setter) => format!("{member_name}.set"),
            _ => member_name,
        };

        Ok(format!("{owner_name}.{member_name}"))
    }
}
