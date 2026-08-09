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
        let declared = self.lowerer.reduced_type(self.node_type_id(expression)?)?;
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
        let bindings = self
            .lowerer
            .instance_bindings(arguments, &self.type_substitution)?;
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let key = self.generic_instance_key(symbol, &arguments)?;
        let ty = self.lower_type(target)?;

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
        let node = expression.into_global_any(self.source);
        let instantiation = self
            .lowerer
            .state(self.source)?
            .decisions
            .instantiation_decision(node)
            .cloned();

        // key explicitly applied references by their recorded arguments
        if let Some(instantiation) = instantiation {
            let bindings = self
                .lowerer
                .instance_bindings(&instantiation.generic_arguments, &self.type_substitution)?;
            let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();

            return self.generic_instance_key(symbol, &arguments);
        }

        // an unconverted generic reference selects no instance: the selection
        //  arrives through the reference's instantiating coercion
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
            _ => Ok(None),
        }
    }
}
