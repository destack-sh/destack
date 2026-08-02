use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::r#type::LoweredSignature;
use crate::lower::{
    CallableImplementation, FunctionDefinition, LifetimeParameters, ModuleLowerer, ReceiverBinding,
    TypeSubstitution,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// One runtime representation of a declaration instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::lower) struct GenericInstanceKey {
    /// The declaration being represented.
    pub(in crate::lower) symbol: dir::GlobalSymbolId,
    /// The runtime type of each representation-relevant argument.
    pub(in crate::lower) representations: Vec<mir::TypeId>,
}

impl GenericInstanceKey {
    /// Create the representation key of one non-generic declaration.
    pub(in crate::lower) fn non_generic(symbol: dir::GlobalSymbolId) -> Self {
        Self {
            symbol,
            representations: Vec::new(),
        }
    }
}

/// Foreign and host callables referenced by function bodies.
#[derive(Default)]
pub(in crate::lower) struct ExternalCallables {
    /// Foreign callables to declare as imports.
    pub(in crate::lower) imports: FxIndexSet<dir::GlobalSymbolId>,
    /// Sealed bindings to declare as dotted host externs.
    pub(in crate::lower) bindings: FxIndexSet<dir::GlobalSymbolId>,
}

/// Visitor collecting every expression node in one body subtree.
struct ExpressionCollector {
    /// The visitor options.
    options: dir::NodeVisitorOptions,
    /// The collected expression nodes.
    expressions: Vec<dir::LocalNodeId<dir::Expression>>,
}

impl dir::NodeVisitor for ExpressionCollector {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        self.expressions.push(id);
        destack_core::ensure_sufficient_stack(|| dir::walk_expression(self, tree, id, expression));
    }
}

impl ModuleLowerer<'_> {
    /// Declare every concrete generic instance reachable from the bodies.
    pub(in crate::lower) fn declare_reachable_instances(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        bodies: &[FunctionDefinition],
    ) -> CompilerResult<(Vec<FunctionDefinition>, ExternalCallables)> {
        // collect calls from the concrete bodies queued for lowering
        let mut pending = Vec::new();
        let mut references = ExternalCallables::default();
        for body in bodies {
            self.collect_body_calls(
                body.source,
                body.expression,
                &body.type_substitution,
                &mut pending,
                &mut references,
            )?;
        }

        // collect calls from the module initializer expressions
        let substitution = TypeSubstitution::default();
        let expressions: Vec<_> = self
            .initializers
            .iter()
            .map(|(_, expression)| *expression)
            .collect();
        for expression in expressions {
            self.collect_body_calls(
                self.module,
                expression,
                &substitution,
                &mut pending,
                &mut references,
            )?;
        }

        // collect calls recursively from each declared instance body
        let mut instances = Vec::new();
        let mut index = 0;
        while index < pending.len() {
            let (symbol, bindings) = pending[index].clone();
            index += 1;
            let Some(body) = self.declare_instance(builder, symbol, &bindings)? else {
                continue;
            };
            self.collect_body_calls(
                symbol.module_id,
                body.expression,
                &body.type_substitution,
                &mut pending,
                &mut references,
            )?;
            instances.push(body);
        }

        Ok((instances, references))
    }

    /// Declare one concrete instance unless its representation is already declared.
    fn declare_instance(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Option<FunctionDefinition>> {
        // key the instance by its runtime representation
        let pointer_bytes = builder.pointer_bytes();
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let type_substitution = TypeSubstitution::default();
        let lifetime_parameters = LifetimeParameters::default();
        let key = self
            .type_lowerer(
                builder.tree_mut(),
                pointer_bytes,
                &type_substitution,
                &lifetime_parameters,
            )
            .generic_instance_key(symbol, &arguments)?;
        if self.functions.contains_key(&key) {
            return Ok(None);
        }

        // declare under the instance's concrete types and polymorphic lifetimes
        let lifetime_parameters = self.lifetime_parameters(self.symbol_type(symbol)?)?;
        let type_substitution = TypeSubstitution::from_bindings(bindings);
        let declared =
            self.declare_instance_header(builder, &key, &type_substitution, &lifetime_parameters);

        declared.map(Some)
    }

    /// Collect one body's calls under one substitution.
    fn collect_body_calls(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        substitution: &TypeSubstitution,
        pending: &mut Vec<(dir::GlobalSymbolId, Vec<dir::GenericArgumentBinding>)>,
        references: &mut ExternalCallables,
    ) -> CompilerResult<()> {
        // walk the body subtree collecting its expression nodes
        let state = self.state(module)?;
        let mut collector = ExpressionCollector {
            options: dir::NodeVisitorOptions::default(),
            expressions: Vec::new(),
        };
        dir::NodeVisitor::visit_expression(
            &mut collector,
            state.tree(),
            expression,
            state.tree().get(expression),
        );

        for id in collector.expressions {
            let node = id.into_global_any(module);

            // import foreign declared constructors
            if let Some(resolution) = state.resolutions.construct_resolution(node)
                && let dir::ConstructTarget::Class(candidate) = &resolution.target
                && let dir::ClassConstructor::Declared { symbol } = &candidate.constructor
                && symbol.module_id != self.module
            {
                references.imports.insert(*symbol);
            }

            // collect the calls selected inside a tree resolution
            if let Some(resolution) = state.resolutions.tree_resolution(node) {
                match &resolution.target {
                    dir::TreeTarget::Element { call, .. } | dir::TreeTarget::Fragment { call } => {
                        self.collect_call_resolution(call, substitution, pending, references)?;
                    }
                    dir::TreeTarget::Component { invocation, .. } => match invocation {
                        dir::TreeInvocation::Call(call) => {
                            self.collect_call_resolution(call, substitution, pending, references)?;
                        }
                        dir::TreeInvocation::Construct(construct) => {
                            if let dir::ConstructTarget::Class(candidate) = &construct.target
                                && let dir::ClassConstructor::Declared { symbol } =
                                    &candidate.constructor
                                && symbol.module_id != self.module
                            {
                                references.imports.insert(*symbol);
                            }
                        }
                        dir::TreeInvocation::Struct { .. } => {}
                    },
                }
            }

            // collect the implementing methods required by erasing coercions
            if let Some(coercion) = state.coercions.coercion(node) {
                self.collect_coercion(coercion, substitution, pending)?;
            }

            let Some(resolution) = state.resolutions.call_resolution(node) else {
                continue;
            };
            self.collect_call_resolution(resolution, substitution, pending, references)?;
        }

        Ok(())
    }

    /// Collect the implementing methods one erasing coercion requires.
    fn collect_coercion(
        &self,
        coercion: &dir::Coercion,
        substitution: &TypeSubstitution,
        pending: &mut Vec<(dir::GlobalSymbolId, Vec<dir::GenericArgumentBinding>)>,
    ) -> CompilerResult<()> {
        let mut source = coercion.source;
        for adjustment in &coercion.adjustments {
            if let dir::CoercionAdjustment::Existential { target } = adjustment {
                self.collect_existential(source, *target, substitution, pending)?;
            }
            source = adjustment.target();
        }

        Ok(())
    }

    /// Collect the methods one concrete source supplies for a constraint.
    fn collect_existential(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
        pending: &mut Vec<(dir::GlobalSymbolId, Vec<dir::GenericArgumentBinding>)>,
    ) -> CompilerResult<()> {
        let source = substitution.resolve(self, source)?;
        let source = self.reduced_type(source)?;
        let dir::Type::Application(class) = self.ty(source)? else {
            return Ok(());
        };

        // read the constraint interface from the erased target
        let target = self.reduced_type(target)?;
        let target = match self.ty(target)? {
            dir::Type::Dynamic(dynamic) => self.reduced_type(dynamic.constraint)?,
            _ => target,
        };
        let dir::Type::Application(interface) = self.ty(target)? else {
            return Ok(());
        };
        let Some(definition) = self.definition(interface.symbol)? else {
            return Ok(());
        };

        // require one declared instance per dispatched method
        for member in definition.members() {
            let dir::DefinitionMember::Method(method) = member else {
                continue;
            };
            let Some(name) = self.symbol_name(method.symbol)? else {
                continue;
            };
            let implementing = self.implementing_method(class.symbol, name)?;
            let instance = (implementing, Vec::new());
            if !pending.contains(&instance) {
                pending.push(instance);
            }
        }

        Ok(())
    }

    /// Collect every declaration selected by one call resolution.
    fn collect_call_resolution(
        &self,
        resolution: &dir::CallResolution,
        substitution: &TypeSubstitution,
        pending: &mut Vec<(dir::GlobalSymbolId, Vec<dir::GenericArgumentBinding>)>,
        references: &mut ExternalCallables,
    ) -> CompilerResult<()> {
        match resolution {
            dir::OperationResolution::One(call) => {
                self.collect_call(call, substitution, pending, references)
            }
            dir::OperationResolution::Union { arms, .. } => {
                for call in arms {
                    self.collect_call(call, substitution, pending, references)?;
                }

                Ok(())
            }
        }
    }

    /// Collect one singular call target.
    fn collect_call(
        &self,
        call: &dir::Call,
        substitution: &TypeSubstitution,
        pending: &mut Vec<(dir::GlobalSymbolId, Vec<dir::GenericArgumentBinding>)>,
        references: &mut ExternalCallables,
    ) -> CompilerResult<()> {
        let function = match &call.target {
            dir::CallTarget::Expression { .. } | dir::CallTarget::Dynamic { .. } => return Ok(()),
            dir::CallTarget::Symbol { function, .. } => function,
        };

        // route intrinsic and binding callables without an instance
        match self.callable_implementation(function.symbol)? {
            // declare dotted host externs for bindings
            Some(CallableImplementation::Binding { .. }) => {
                references.bindings.insert(function.symbol);

                return Ok(());
            }
            // emit intrinsics without declarations
            Some(CallableImplementation::Intrinsic { .. }) => return Ok(()),
            None => {}
        }

        // select an instance only when the call binds type parameters
        let bindings = self.instance_bindings(function, substitution)?;
        if bindings.is_empty() {
            // import plain calls into other modules
            if function.symbol.module_id != self.module {
                references.imports.insert(function.symbol);
            }

            return Ok(());
        }

        // require every generic argument to be concrete under this body instance
        for binding in &bindings {
            if matches!(self.ty(binding.argument)?, dir::Type::Parameter(_)) {
                return Err(CompilerError::Internal {
                    message: "instantiation collection left a generic argument unsubstituted"
                        .to_string(),
                });
            }
        }
        let instance = (function.symbol, bindings);
        if !pending.contains(&instance) {
            pending.push(instance);
        }

        Ok(())
    }

    /// Return the substituted type bindings one candidate selects beyond lifetimes.
    pub(in crate::lower) fn instance_bindings(
        &self,
        function: &dir::FunctionTarget,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let mut bindings = Vec::new();
        for binding in &function.generic_arguments {
            let parameter = binding.parameter;
            let generics = &self.state(parameter.module_id)?.generics;
            let parameter = generics.get_parameter(parameter.local_id);
            match parameter.kind {
                dir::GenericParameterKind::Memory(dir::MemoryParameter::Lifetime) => continue,
                dir::GenericParameterKind::Type => {}
                dir::GenericParameterKind::Value | dir::GenericParameterKind::Memory(_) => {
                    return Err(LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: "a value-parameterized callable instance".to_string(),
                    }
                    .into());
                }
            }

            // substitute arguments through the enclosing instance
            let argument = substitution.resolve(self, binding.argument)?;
            bindings.push(dir::GenericArgumentBinding {
                parameter: binding.parameter,
                argument,
            });
        }

        Ok(bindings)
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
            type_substitution.clone(),
            lifetime_parameters,
            expression,
        )
    }

    /// Declare the header of one static member instance and queue its body.
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

        // require the member to be static
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
        if !is_static {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a generic instance method instance".to_string(),
            }
            .into());
        }

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
        let signature =
            self.lower_signature(builder, declared, &type_substitution, lifetime_parameters)?;
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
        type_substitution: TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<FunctionDefinition> {
        let symbol = key.symbol;
        let name = self.symbol_path(symbol)?;
        let base = mir::Symbol::named(builder.intern(&name));
        let instance = base.instantiate(&key.representations, builder.tree());
        let header = builder.function_header(&name).symbol(instance);
        let header = lifetime_parameters.declare(header);
        let header = header
            .parameters(signature.parameters)
            .result(signature.result);
        let function = builder.declare_function(header);
        self.functions.insert(key.clone(), function);

        Ok(FunctionDefinition {
            function,
            has_this: false,
            parameters: symbols,
            type_substitution,
            lifetime_parameters: lifetime_parameters.clone(),
            source: symbol.module_id,
            expression,
        })
    }
}
