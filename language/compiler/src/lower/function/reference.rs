use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{Binding, FunctionLowerer, GenericInstanceKey, Instance};
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Materialize one callable declaration as a function value.
    pub(in crate::lower) fn lower_function_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
        environment: Option<mir::Value>,
    ) -> CompilerResult<Option<mir::Value>> {
        // emit the allocation recorded for a constructor function reference
        let node = expression.into_global_any(self.source);
        if let Some(dir::OperationResolution::One(dir::FunctionValue {
            target: dir::CallableTarget::Constructor(construction),
            callable_type,
        })) = self.source().decisions.function_decision(node).cloned()
        {
            return self
                .lower_constructor_value(callable_type, &construction)
                .map(Some);
        }

        // bind the selected instance at the callable form its context stores it beneath
        let declared = match self.coercion(expression) {
            Some(coercion)
                if matches!(
                    coercion.adjustments.as_slice(),
                    [dir::CoercionAdjustment::Representation { .. }]
                ) =>
            {
                coercion.target()
            }
            _ => self.representation_type_id(expression)?,
        };
        let instance = self.function_reference_instance(expression, symbol)?;
        let ty = self.lower_type(declared)?;

        self.bind_function_value(ty, instance, environment)
    }

    /// Materialize one callable reference at its coercion-selected instance.
    pub(in crate::lower) fn lower_instantiated_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalTypeId,
        arguments: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<mir::Value> {
        // select the instance by the coercion's arguments, a body-local closure by its declaration
        let instance = if self.is_body_local_closure(expression, symbol) {
            Instance::Declared(self.function(&GenericInstanceKey::non_generic(symbol))?)
        } else {
            let bindings = self.lower.instance_bindings(arguments)?;
            self.instance_of(symbol, &dir::InstanceKey::new(symbol, bindings))?
        };
        let ty = self.lower_type(target)?;

        // require a callable representation
        match self.bind_function_value(ty, instance, None)? {
            Some(value) => Ok(value),
            None => Err(CompilerError::Internal {
                message: "an instantiated reference outside a callable type".to_string(),
            }),
        }
    }

    /// Return the instance selected by one callable reference.
    fn function_reference_instance(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Instance> {
        // declare a body-local closure once, polymorphic over the enclosing template
        if self.is_body_local_closure(expression, symbol) {
            let key = GenericInstanceKey::non_generic(symbol);

            return self.function(&key).map(Instance::Declared);
        }

        // read the instance selected at this reference
        let node = expression.into_global_any(self.source);
        let selected = self
            .lower
            .state(self.source)?
            .decisions
            .function_decision(node)
            .and_then(|decision| match decision {
                dir::OperationResolution::One(value) => value.key().cloned(),
                dir::OperationResolution::Union { .. } => None,
            });

        // require a selection behind every declared function reference
        if selected.is_none() {
            let kind = self
                .lower
                .state(symbol.module_id)?
                .bindings
                .get_symbol(symbol.local_id)
                .kind;
            let is_reference = !matches!(
                self.source().tree().get(expression),
                dir::Expression::Declaration(_)
            );
            if is_reference && kind == dir::SymbolKind::Function {
                return Err(CompilerError::Internal {
                    message: format!("a function reference {node:?} without a function decision"),
                });
            }
        }

        // select explicitly applied references by their recorded arguments
        if let Some(selection) = selected
            && !selection.arguments.is_empty()
        {
            return self.instance_of(symbol, &selection);
        }

        // reject a generic reference whose instantiating coercion selected no instance
        let declared = self.lower.symbol_type(symbol)?;
        let is_callable = matches!(
            self.lower.ty(declared)?,
            dir::Type::Function(_) | dir::Type::FunctionSignature(_)
        );
        if is_callable && !self.lower.signature_type_parameters(declared)?.is_empty() {
            return Err(self.unsupported(
                "a generic function reference without an instantiating conversion".to_string(),
            ));
        }

        self.function(&GenericInstanceKey::non_generic(symbol))
            .map(Instance::Declared)
    }

    /// Emit one function value of one lowered callable type.
    pub(in crate::lower) fn bind_function_value(
        &mut self,
        ty: mir::TypeId,
        instance: Instance,
        environment: Option<mir::Value>,
    ) -> CompilerResult<Option<mir::Value>> {
        let (function, arguments) = match instance {
            Instance::Declared(function) => (function, Vec::new()),
            Instance::Applied {
                template,
                arguments,
            } => (template, arguments),
        };

        // emit by the callable representation
        match self.builder.tree().type_definition(ty) {
            // pair fat function values with their environment
            mir::Type::Function { .. } => {
                // pair environment-free values with the absent environment
                let environment = match environment {
                    Some(environment) => environment,
                    None => {
                        let tree = self.builder.tree_mut();
                        let void = tree.void_type();
                        let environment = tree.intern_type(mir::Type::Pointer {
                            pointee: void,
                            access: mir::Access::Readonly,
                        });

                        self.builder.constant(mir::Constant::Null, environment)
                    }
                };

                Ok(Some(self.builder.function_bind(
                    function,
                    arguments,
                    ty,
                    environment,
                )))
            }
            // emit thin function pointers directly
            mir::Type::FunctionPointer { .. } => {
                Ok(Some(self.builder.function_addr(function, arguments, ty)))
            }
            // leave every other representation without a value
            _ => Ok(None),
        }
    }

    /// Read the current value of one binding.
    pub(in crate::lower) fn read_binding(
        &mut self,
        binding: Binding,
    ) -> CompilerResult<mir::Value> {
        // read by the storage the binding holds
        Ok(match binding {
            Binding::Local(local) => {
                let ty = self.builder.tree().get(local).ty;

                self.load_place(mir::Place::local(local), ty)
            }
            // load captured bindings through their frame field
            Binding::Captured { frame, field, ty } => {
                let place = mir::Place::value(frame)
                    .with_projection(mir::Projection::Deref)
                    .with_projection(mir::Projection::Field { index: field });

                self.load_place(place, ty)
            }
        })
    }

    /// Lower one value expression that resolved to a symbol.
    pub(in crate::lower) fn read_symbol(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<mir::Value> {
        // read a local binding, else a module or callable declaration
        let binding = if symbol.module_id == self.source {
            self.values.get(&symbol.local_id).copied()
        } else {
            None
        };
        match binding {
            Some(binding) => {
                let value = self.read_binding(binding)?;

                // project a flow-narrowed read onto its recorded narrowing
                self.lower_narrowing(expression, value)
            }
            // load module constants through their globals
            None => {
                if let Some(global) = self.constant_global(symbol)? {
                    return Ok(self.builder.load_global(global));
                }

                // read a const parameter as the value its instantiation binds
                if let Some(value) = self.lower_parameter_value(expression, symbol)? {
                    return Ok(value);
                }

                // materialize callable declarations as function values
                if let Some(value) = self.lower_function_value(expression, symbol, None)? {
                    return Ok(value);
                }

                Err(self.unsupported("a module or captured binding"))
            }
        }
    }

    /// Lower one reference to a const parameter of the enclosing template.
    fn lower_parameter_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::Value>> {
        let Some(index) = self.const_parameter_index(symbol)? else {
            return Ok(None);
        };
        let ty = self.lower_type(self.node_type_id(expression)?)?;

        Ok(Some(
            self.builder.constant(mir::Constant::Parameter(index), ty),
        ))
    }

    /// Return the template index one const parameter symbol names.
    pub(in crate::lower) fn const_parameter_index(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<u32>> {
        let Some(parameter) = self
            .lower
            .state(symbol.module_id)?
            .generics
            .parameter_by_symbol(symbol)
        else {
            return Ok(None);
        };
        let parameter = parameter.into_global(symbol.module_id);
        match self.scope.parameter_index(parameter) {
            Some(index) => Ok(Some(index)),
            None => Err(CompilerError::Internal {
                message: "a const parameter read outside its template".to_string(),
            }),
        }
    }

    /// Return the const parameter index one expression reads, `None` for every other expression.
    pub(in crate::lower) fn read_parameter_index(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<u32>> {
        let node = expression.into_global_any(self.source);
        let Some(symbol) = self
            .source()
            .resolutions
            .name_resolution(node)
            .and_then(|resolution| resolution.single_symbol())
        else {
            return Ok(None);
        };

        self.const_parameter_index(symbol)
    }

    /// Return the declared global behind one constant binding, a foreign one imported.
    pub(in crate::lower) fn constant_global(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::LocalNodeId<mir::Global>>> {
        self.lower.constant_global(self.builder.tree_mut(), symbol)
    }

    /// Return whether one reference names a closure declared in this body.
    fn is_body_local_closure(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> bool {
        symbol.module_id == self.source
            && matches!(
                self.source().tree().get(expression),
                dir::Expression::Declaration(_)
            )
    }
}
