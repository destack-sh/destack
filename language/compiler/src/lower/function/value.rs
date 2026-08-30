use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{FunctionLowerer, GenericInstanceKey};
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
        let bindings = self.lowerer.instance_bindings(arguments, self.instance)?;
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
        // read the instance the checker selected at this reference
        let node = expression.into_global_any(self.source);
        let selected = self
            .lowerer
            .state(self.source)?
            .decisions
            .function_decision(node)
            .and_then(|decision| match decision {
                dir::OperationResolution::One(value) => value.key().cloned(),
                dir::OperationResolution::Union { .. } => None,
            });

        // require the checker's selection behind every declared function reference
        if selected.is_none() {
            let kind = self
                .lowerer
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
                    message: format!("function reference {node:?} has no function decision"),
                });
            }
        }

        // key explicitly applied references by their recorded arguments
        if let Some(selection) = selected
            && !selection.arguments.is_empty()
        {
            let bindings = self
                .lowerer
                .instance_bindings(&selection.arguments, self.instance)?;
            let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();

            // resolve the receiver through the enclosing instance's types
            let receiver = match selection.receiver {
                Some(receiver) => Some(self.lowerer.instance_type(self.instance, receiver)?),
                None => None,
            };

            return self.generic_instance_key(symbol, receiver, &arguments);
        }

        // reject a generic reference whose instantiating coercion selected no instance
        let declared = self.lowerer.symbol_type(symbol)?;
        if !self
            .lowerer
            .signature_template_parameters(declared)?
            .is_empty()
        {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
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
            // pair fat function values with an empty environment
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
}
