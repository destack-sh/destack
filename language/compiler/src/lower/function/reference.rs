use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{Binding, FunctionLowerer, GenericInstanceKey};
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Materialize one callable declaration as a function value.
    pub(in crate::lower) fn lower_function_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
        environment: Option<mir::Value>,
    ) -> CompilerResult<Option<mir::Value>> {
        // bind the selected instance at the reference's lowered type
        let declared = self.representation_type_id(expression)?;
        let key = self.function_reference_key(expression, symbol)?;
        let ty = self.lower_type(declared)?;

        self.bind_function_value(ty, &key, environment)
    }

    /// Materialize one callable reference at its coercion-selected instance.
    pub(in crate::lower) fn lower_instantiated_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalTypeId,
        arguments: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<mir::Value> {
        // key the instance by the coercion's selected arguments
        let bindings = self.lower.instance_bindings(arguments, self.instance)?;
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let key = self.generic_instance_key(symbol, None, &arguments)?;
        let ty = self.lower_type(target)?;

        // require a callable representation
        match self.bind_function_value(ty, &key, None)? {
            Some(value) => Ok(value),
            None => Err(CompilerError::Internal {
                message: "an instantiated reference outside a callable type".to_string(),
            }),
        }
    }

    /// Return the instance key selected by one callable reference.
    fn function_reference_key(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<GenericInstanceKey> {
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

        // key explicitly applied references by their recorded arguments
        if let Some(selection) = selected
            && !selection.arguments.is_empty()
        {
            let bindings = self
                .lower
                .instance_bindings(&selection.arguments, self.instance)?;
            let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();

            // resolve the receiver through the enclosing instance's types
            let receiver = match selection.receiver {
                Some(receiver) => Some(self.lower.instance_type(self.instance, receiver)?),
                None => None,
            };

            return self.generic_instance_key(symbol, receiver, &arguments);
        }

        // reject a generic reference whose instantiating coercion selected no instance
        let declared = self.lower.symbol_type(symbol)?;
        if !self
            .lower
            .signature_template_parameters(declared)?
            .is_empty()
        {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a generic function reference without an instantiating conversion"
                    .to_string(),
            }
            .into());
        }

        Ok(GenericInstanceKey::non_generic(symbol))
    }

    /// Emit one function value of one lowered callable type.
    fn bind_function_value(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        key: &GenericInstanceKey,
        environment: Option<mir::Value>,
    ) -> CompilerResult<Option<mir::Value>> {
        // emit by the callable representation
        match self.builder.tree().get(ty) {
            // pair fat function values with their environment
            mir::Type::Function {
                kind,
                lifetime,
                storage,
                access,
                ..
            } => {
                let kind = *kind;
                let lifetime = lifetime.clone();
                let storage = *storage;
                let access = *access;
                let function = self.function(key)?;

                // pair environment-free values with a null environment
                let environment = match environment {
                    Some(environment) => environment,
                    None => {
                        let pointee = self.builder.tree_mut().intern_type(mir::Type::Void);
                        let environment =
                            self.builder.tree_mut().intern_type(mir::Type::Reference {
                                kind,
                                lifetime,
                                storage,
                                access,
                                pointee,
                                nullability: mir::Nullability::Null,
                            });

                        self.builder.constant(mir::Constant::Null, environment)
                    }
                };

                Ok(Some(self.builder.function_bind(function, ty, environment)))
            }
            // emit thin function pointers directly
            mir::Type::FunctionPointer { .. } => {
                let function = self.function(key)?;

                Ok(Some(self.builder.function_addr(function, ty)))
            }
            // leave every other representation without a value
            _ => Ok(None),
        }
    }

    /// Read the current value of one binding.
    pub(in crate::lower) fn read_binding(&mut self, binding: Binding) -> mir::Value {
        // read by the storage the binding holds
        match binding {
            Binding::Value(value) => value,
            Binding::Local(local) => self.builder.local_get(local),
            // load captured bindings through their frame field
            Binding::Captured { frame, field, ty } => {
                let address = self.emit_field_address(frame, field, ty, mir::Access::Mutable);

                self.builder.load(address, ty)
            }
        }
    }

    /// Lower one value expression that resolved to a symbol.
    pub(in crate::lower) fn lower_resolved_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<mir::Value> {
        // read a local binding, else a module or callable declaration
        match self.values.get(&symbol.local_id).copied() {
            Some(binding) => Ok(self.read_binding(binding)),
            // load module constants through their globals
            None => {
                if let Some(global) = self.module_constant_global(symbol)? {
                    return Ok(self.builder.load_global(global));
                }

                // materialize callable declarations as function values
                if let Some(value) = self.lower_function_value(expression, symbol, None)? {
                    return Ok(value);
                }

                Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "a module or captured binding".to_string(),
                }
                .into())
            }
        }
    }

    /// Return the declared global behind one module constant.
    fn module_constant_global(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::LocalNodeId<mir::Global>>> {
        // declare the imported constant on its first read
        if !self.lower.globals.contains_key(&symbol) {
            // skip local constants and non-binding symbols
            if symbol.module_id == self.lower.module || !self.lower.is_module_binding(symbol)? {
                return Ok(None);
            }

            self.lower
                .declare_imported_constant(self.builder.tree_mut(), symbol)?;
        }

        // read the global this module declared or imported
        match self.lower.globals.get(&symbol) {
            Some(Ok(global)) => Ok(Some(*global)),
            // cascade the recorded declaration failure
            Some(Err(diagnostic)) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
            None => {
                let path = self.lower.symbol_path(symbol)?;

                Err(CompilerError::Internal {
                    message: format!("a missing global behind the constant '{path}'"),
                })
            }
        }
    }
}
