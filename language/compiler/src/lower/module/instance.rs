use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    CallableImplementation, FunctionDefinition, LifetimeParameters, ModuleLowerer, TypeSubstitution,
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

        // collect calls recursively from each declared instance body
        let mut instances = Vec::new();
        let mut index = 0;
        while index < pending.len() {
            let (symbol, arguments) = pending[index].clone();
            index += 1;
            let Some(body) = self.declare_instance(builder, symbol, &arguments)? else {
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
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<FunctionDefinition>> {
        // source types with the same runtime representation share one instance
        let pointer_bytes = builder.pointer_bytes();
        let type_substitution = TypeSubstitution::default();
        let lifetime_parameters = LifetimeParameters::default();
        let key = self
            .type_lowerer(
                builder.tree_mut(),
                pointer_bytes,
                &type_substitution,
                &lifetime_parameters,
            )
            .generic_instance_key(symbol, arguments)?;
        if self.functions.contains_key(&key) {
            return Ok(None);
        }

        // declare under the instance's concrete types and polymorphic lifetimes
        let lifetime_parameters = self.lifetime_parameters(self.symbol_type(symbol)?)?;
        let type_substitution = self.instance_substitution(symbol, arguments)?;
        let declared =
            self.declare_instance_header(builder, &key, &type_substitution, &lifetime_parameters);

        declared.map(Some)
    }

    /// Collect one body's sealed calls under one substitution.
    fn collect_body_calls(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        substitution: &TypeSubstitution,
        pending: &mut Vec<(dir::GlobalSymbolId, Vec<dir::GlobalTypeId>)>,
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

            // foreign declared constructors resolve through imports
            if let Some(resolution) = state.resolutions.construct_resolution(node)
                && let dir::ConstructTarget::Class(candidate) = &resolution.target
                && let dir::ClassConstructor::Declared { symbol } = &candidate.constructor
                && symbol.module_id != self.module
            {
                references.imports.insert(*symbol);
            }

            let Some(resolution) = state.resolutions.call_resolution(node) else {
                continue;
            };
            self.collect_call_resolution(resolution, substitution, pending, references)?;
        }

        Ok(())
    }

    /// Collect every declaration selected by one call resolution.
    fn collect_call_resolution(
        &self,
        resolution: &dir::CallResolution,
        substitution: &TypeSubstitution,
        pending: &mut Vec<(dir::GlobalSymbolId, Vec<dir::GlobalTypeId>)>,
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
        pending: &mut Vec<(dir::GlobalSymbolId, Vec<dir::GlobalTypeId>)>,
        references: &mut ExternalCallables,
    ) -> CompilerResult<()> {
        let function = match &call.target {
            dir::CallTarget::Expression { .. } | dir::CallTarget::Dynamic { .. } => return Ok(()),
            dir::CallTarget::Symbol { function, .. } => function,
        };

        // sealed intrinsic and binding callables require no instance
        match self.callable_implementation(function.symbol)? {
            // sealed bindings declare dotted host externs
            Some(CallableImplementation::Binding { .. }) => {
                references.bindings.insert(function.symbol);

                return Ok(());
            }
            // sealed intrinsics emit MIR without declarations
            Some(CallableImplementation::Intrinsic { .. }) => return Ok(()),
            None => {}
        }

        // only calls binding type parameters select instances
        let arguments = self.instance_arguments(function, substitution)?;
        if arguments.is_empty() {
            // plain calls into other modules resolve through imports
            if function.symbol.module_id != self.module {
                references.imports.insert(function.symbol);
            }

            return Ok(());
        }

        // require every generic argument to be concrete under this body instance
        for argument in &arguments {
            if matches!(self.ty(*argument)?, dir::Type::Parameter(_)) {
                return Err(CompilerError::Internal {
                    message: "instantiation collection left a generic argument unsubstituted"
                        .to_string(),
                });
            }
        }
        let instance = (function.symbol, arguments);
        if !pending.contains(&instance) {
            pending.push(instance);
        }

        Ok(())
    }

    /// Return the substituted type arguments one candidate binds beyond lifetimes.
    pub(in crate::lower) fn instance_arguments(
        &self,
        function: &dir::FunctionTarget,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut arguments = Vec::new();
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
            arguments.push(argument);
        }

        Ok(arguments)
    }

    /// Return the parameter substitution selecting one instance's arguments.
    fn instance_substitution(
        &self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<TypeSubstitution> {
        // the callable's template names the substituted parameters in order
        let declared = self.symbol_type(symbol)?;
        let (signature, owner) = self.signature(declared)?;
        let Some(template) = self.types(owner)?.signature(signature).template else {
            return Err(CompilerError::Internal {
                message: "checked DIR instantiated a non-generic callable".to_string(),
            });
        };

        TypeSubstitution::bind(self, template, arguments, &TypeSubstitution::default())
    }

    /// Declare the MIR header of one generic instance and queue its body.
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
                message: "checked DIR instantiated a callable without a declaration".to_string(),
            });
        };
        let Ok(declaration) = declaration.local_id.try_into_typed::<dir::Declaration>() else {
            return Err(CompilerError::Internal {
                message: "checked DIR instantiated a non-declaration callable".to_string(),
            });
        };

        // resolve the source body and parameter symbols
        let dir::Declaration::Function(function) =
            self.state(symbol.module_id)?.tree().get(declaration)
        else {
            return Err(CompilerError::Internal {
                message: "checked DIR instantiated a non-function declaration".to_string(),
            });
        };
        let Some(expression) = function.body else {
            return Err(CompilerError::Internal {
                message: "checked DIR instantiated a bodiless function".to_string(),
            });
        };
        let parameter_nodes = function.signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            let node = parameter.into_global_any(symbol.module_id);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "checked DIR is missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }
        let declared = self.symbol_type(symbol)?;
        let signature =
            self.lower_signature(builder, declared, type_substitution, lifetime_parameters)?;
        if signature.parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "checked DIR instance parameters disagree with its sealed signature"
                    .to_string(),
            });
        }

        // declare the header under the instance's canonical name
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
            type_substitution: type_substitution.clone(),
            lifetime_parameters: lifetime_parameters.clone(),
            source: symbol.module_id,
            expression,
        })
    }
}
