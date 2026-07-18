use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{AmbientCallable, Body, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// Visitor collecting call expressions in one body.
struct CallCollector {
    /// The collected expressions.
    expressions: Vec<dir::LocalNodeId<dir::Expression>>,
}

impl dir::NodeVisitor for CallCollector {
    fn options(&self) -> &dir::NodeVisitorOptions {
        const OPTIONS: dir::NodeVisitorOptions = dir::NodeVisitorOptions {};

        &OPTIONS
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if matches!(
            expression,
            dir::Expression::Call { .. } | dir::Expression::New { .. }
        ) {
            self.expressions.push(id);
        }
        destack_core::ensure_sufficient_stack(|| dir::walk_expression(self, tree, id, expression));
    }
}

/// Foreign and host callables demanded by the scanned bodies.
#[derive(Default)]
pub(in crate::lower) struct CallDemands {
    /// Foreign callables to declare as imports.
    pub(in crate::lower) imports: FxIndexSet<dir::GlobalSymbolId>,
    /// Sealed bindings to declare as dotted host externs.
    pub(in crate::lower) bindings: FxIndexSet<dir::GlobalSymbolId>,
}

impl ModuleLowerer<'_> {
    /// Materialize and declare every generic instantiation the bodies demand.
    ///
    /// Returns the instance bodies to lower and the foreign callables the
    /// scanned bodies call without instantiating.
    pub(in crate::lower) fn materialize_instances(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        bodies: &[Body],
    ) -> CompilerResult<(Vec<Body>, CallDemands)> {
        // seed the demands from the concrete bodies queued for lowering
        let mut pending = Vec::new();
        let mut demands = CallDemands::default();
        for body in bodies {
            self.collect_body_calls(
                body.source,
                body.expression,
                &FxIndexMap::default(),
                &mut pending,
                &mut demands,
            )?;
        }

        // each instance's body reveals further demands under its substitution
        let mut instances = Vec::new();
        let mut index = 0;
        while index < pending.len() {
            let (symbol, arguments) = pending[index].clone();
            index += 1;
            let Some(body) = self.materialize_instance(builder, symbol, &arguments)? else {
                continue;
            };
            self.collect_body_calls(
                symbol.module_id,
                body.expression,
                &body.substitution,
                &mut pending,
                &mut demands,
            )?;
            instances.push(body);
        }

        Ok((instances, demands))
    }

    /// Declare one demanded instance, unless its carrier key already declared.
    fn materialize_instance(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<Body>> {
        // instances sealing distinct types share one carrier key
        let key = self.instance_key(builder.tree_mut(), arguments)?;
        if self.functions.contains_key(&(symbol, key.clone())) {
            return Ok(None);
        }

        // declare under the instance's substitution and source module
        self.substitution = self.instance_substitution_for(symbol, arguments)?;
        self.source = symbol.module_id;
        let declared = self.declare_instance(builder, symbol, &key);
        self.source = self.module;
        self.substitution = FxIndexMap::default();

        declared.map(Some)
    }

    /// Collect the demands one body's sealed calls make under one substitution.
    ///
    /// Concrete generic calls demand instances; plain calls into other modules
    /// demand imports. A scanned body's calls instantiate concretely: a leftover
    /// parameter argument means the collector walked a body outside its instance.
    fn collect_body_calls(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        substitution: &FxIndexMap<dir::GlobalGenericParameterId, dir::GlobalTypeId>,
        pending: &mut Vec<(dir::GlobalSymbolId, Vec<dir::GlobalTypeId>)>,
        demands: &mut CallDemands,
    ) -> CompilerResult<()> {
        // collect the calls that can select generic instances or imports
        let state = self.state(module)?;
        let mut calls = CallCollector {
            expressions: Vec::new(),
        };
        let body = state.tree().get(expression);
        dir::NodeVisitor::visit_expression(&mut calls, state.tree(), expression, body);

        for id in calls.expressions {
            let node = id.into_global_any(module);

            // foreign declared constructors resolve through imports
            if let Some(resolution) = state.resolutions.construct_resolution(node)
                && let dir::ConstructTarget::Class(candidate) = &resolution.target
                && let dir::ClassConstructor::Declared { symbol } = &candidate.constructor
                && symbol.module_id != self.module
            {
                demands.imports.insert(*symbol);
            }

            let Some(resolution) = state.resolutions.call_resolution(node) else {
                continue;
            };
            let dir::CallTarget::Symbol(candidate) = &resolution.target else {
                continue;
            };

            // sealed intrinsic and binding callables never materialize instances
            match self.ambient_callable(candidate.symbol)? {
                // sealed bindings declare dotted host externs
                Some(AmbientCallable::Binding { .. }) => {
                    demands.bindings.insert(candidate.symbol);
                    continue;
                }
                // sealed intrinsics emit MIR without declarations
                Some(AmbientCallable::Intrinsic { .. }) => continue,
                None => {}
            }

            // only calls binding type arguments demand instances
            let arguments = self.instance_arguments(candidate, substitution)?;
            if arguments.is_empty() {
                // plain calls into other modules resolve through imports
                if candidate.symbol.module_id != self.module {
                    demands.imports.insert(candidate.symbol);
                }
                continue;
            }

            // a scanned body's calls instantiate concretely
            for argument in &arguments {
                if matches!(self.ty(*argument)?, dir::Type::Parameter(_)) {
                    return Err(CompilerError::Internal {
                        message: "instantiation collection left a generic argument unsubstituted"
                            .to_string(),
                    });
                }
            }
            let instance = (candidate.symbol, arguments);
            if !pending.contains(&instance) {
                pending.push(instance);
            }
        }

        Ok(())
    }

    /// Return the substituted type arguments one candidate binds beyond lifetimes.
    pub(in crate::lower) fn instance_arguments(
        &self,
        candidate: &dir::CallCandidate,
        substitution: &FxIndexMap<dir::GlobalGenericParameterId, dir::GlobalTypeId>,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut arguments = Vec::new();
        for binding in &candidate.generic_arguments {
            let parameter = binding.parameter;
            let generics = &self.state(parameter.module_id)?.generics;
            if generics
                .get_parameter(parameter.local_id)
                .memory_parameter()
                == Some(dir::MemoryParameter::Lifetime)
            {
                continue;
            }

            // substitute arguments that name outer parameters through the instance
            let argument = match self.ty(binding.argument)? {
                dir::Type::Parameter(outer) => substitution
                    .get(&outer)
                    .copied()
                    .unwrap_or(binding.argument),
                _ => binding.argument,
            };
            arguments.push(argument);
        }

        Ok(arguments)
    }

    /// Return the runtime carrier key identifying one instance's arguments.
    ///
    /// Sealed argument types lower to their storage carriers before keying, so
    /// two calls sealing distinct literal refinements of one carrier share one
    /// materialized instance.
    pub(in crate::lower) fn instance_key(
        &mut self,
        tree: &mut mir::Tree,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<mir::Type>> {
        let mut key = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let carrier = self.lower_type_id(tree, *argument)?;
            key.push(tree.get(carrier).clone());
        }

        Ok(key)
    }

    /// Return the parameter substitution selecting one instance's arguments.
    fn instance_substitution_for(
        &self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<FxIndexMap<dir::GlobalGenericParameterId, dir::GlobalTypeId>> {
        // the callable's template names the substituted parameters in order
        let declared = self.symbol_type(symbol)?;
        let (signature, owner) = self.signature_of(declared)?;
        let Some(template) = self.types(owner)?.signature(signature).template else {
            return Err(CompilerError::Internal {
                message: "checked DIR instantiated a non-generic callable".to_string(),
            });
        };

        let mut substitution = FxIndexMap::default();
        let generics = &self.state(template.module_id)?.generics;
        let template_module = template.module_id;
        let template = generics.get_template(template.local_id);
        let mut position = 0;
        for parameter in &template.parameters {
            let binding = generics.get_parameter(*parameter);
            if binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime) {
                continue;
            }
            let Some(argument) = arguments.get(position).copied() else {
                return Err(CompilerError::Internal {
                    message: "checked DIR instantiated a callable with missing arguments"
                        .to_string(),
                });
            };
            substitution.insert(parameter.into_global(template_module), argument);
            position += 1;
        }

        Ok(substitution)
    }

    /// Declare the MIR header of one generic instance and queue its body.
    fn declare_instance(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        key: &[mir::Type],
    ) -> CompilerResult<Body> {
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

        // declare the signature's lifetime generics before its parameter types
        let lifetimes = self.signature_lifetimes(self.symbol_type(symbol)?)?;
        self.lifetime_slots = lifetimes.clone();

        // resolve the substituted parameter and return types
        let (expression, parameter_nodes) = {
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

            (expression, function.signature.parameters.clone())
        };
        let mut parameters = Vec::with_capacity(parameter_nodes.len());
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        for parameter in &parameter_nodes {
            let node = parameter.into_global_any(symbol.module_id);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "checked DIR is missing a symbol for one parameter".to_string(),
                });
            };
            let ty = self.symbol_type(parameter_symbol)?;
            parameters.push(self.lower_type_id(builder.tree_mut(), ty)?);
            symbols.push(parameter_symbol.local_id);
        }
        let declared = self.symbol_type(symbol)?;
        let result = match self.signature_return(declared)? {
            Some(return_type) => self.lower_type_id(builder.tree_mut(), return_type)?,
            None => builder.tree_mut().insert(mir::Type::Void),
        };

        // declare the header under the instance's canonical name
        let Some(name) = self.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR instantiated a function without a name".to_string(),
            });
        };
        let path = &self.state(symbol.module_id)?.path;
        let mut name = format!("{path}.{}", self.strings.get(name));
        for carrier in key {
            name.push('#');
            name.push_str(&self.type_symbol_text(carrier)?);
        }
        let mut header = builder.function_header(&name);
        for slot in 0..lifetimes.len() {
            header = header.lifetime(&format!("L{slot}"));
        }
        let header = header.parameters(parameters).result(result);
        let function = builder.declare_function(header);
        self.functions.insert((symbol, key.to_vec()), function);

        Ok(Body {
            function,
            has_this: false,
            parameters: symbols,
            lifetimes,
            substitution: self.substitution.clone(),
            source: symbol.module_id,
            expression,
        })
    }

    /// Return the canonical symbol text of one instance carrier.
    fn type_symbol_text(&self, carrier: &mir::Type) -> CompilerResult<String> {
        // spell scalars by their carrier names
        match carrier {
            mir::Type::Boolean => Ok("boolean".to_string()),
            mir::Type::Isize => Ok("isize".to_string()),
            mir::Type::Usize => Ok("usize".to_string()),
            mir::Type::Int {
                width,
                is_signed: true,
            } => Ok(format!("int{width}")),
            mir::Type::Int {
                width,
                is_signed: false,
            } => Ok(format!("uint{width}")),
            mir::Type::Float(float) => Ok(float.label().to_string()),
            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("a monomorphization over the '{other:?}' carrier"),
            })?,
        }
    }
}
