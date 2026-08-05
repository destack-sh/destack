use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{FunctionLowerer, GenericInstanceKey};
use crate::{CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Materialize one callable declaration as a function value.
    pub(in crate::lower) fn lower_function_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::Value>> {
        let node = expression.into_global_any(self.source);
        let declared = self.lowerer.reduced_type(self.node_type_id(expression)?)?;

        // lower polymorphic references at their converted concrete expectation
        let is_polymorphic = matches!(
            self.lowerer.ty(declared)?,
            dir::Type::Function(_)
                | dir::Type::FunctionSignature(_)
                | dir::Type::FunctionPointer(_)
        ) && !self
            .lowerer
            .signature_template_parameters(declared)?
            .is_empty();
        let concrete = match is_polymorphic {
            false => declared,
            true => self
                .lowerer
                .types(self.source)?
                .get_expected_type_id(node)
                .ok_or_else(|| LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a generic function reference without a concrete expectation"
                        .to_string(),
                })?,
        };
        let ty = self.lower_type(concrete)?;
        let key = self.function_reference_key(expression, symbol)?;

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
                let function = self.function(&key)?;
                let pointee = self.builder.tree_mut().intern_type(mir::Type::Void);
                let environment = self.builder.tree_mut().intern_type(mir::Type::Reference {
                    kind,
                    lifetime,
                    storage,
                    access,
                    pointee,
                    nullability: mir::Nullability::Null,
                });
                let environment = self.builder.constant(mir::Constant::Null, environment);

                Ok(Some(self.builder.function_bind(function, ty, environment)))
            }
            // emit thin function pointers directly
            mir::Type::FunctionPointer { .. } => {
                let function = self.function(&key)?;

                Ok(Some(self.builder.function_addr(function, ty)))
            }
            _ => Ok(None),
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
            .resolutions
            .instantiation_resolution(node)
            .cloned();

        // key explicitly applied references by their recorded arguments
        if let Some(instantiation) = instantiation {
            let bindings = self
                .lowerer
                .instance_bindings(&instantiation.generic_arguments, &self.type_substitution)?;
            let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();

            return self.type_lowerer().generic_instance_key(symbol, &arguments);
        }

        // recover inferred instantiations from the converted concrete expectation
        let declared = self.lowerer.symbol_type(symbol)?;
        let parameters = self.lowerer.signature_template_parameters(declared)?;
        if !parameters.is_empty() {
            let expected = self
                .lowerer
                .types(self.source)?
                .get_expected_type_id(node)
                .ok_or_else(|| LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a generic function reference without a concrete expectation"
                        .to_string(),
                })?;
            let mut bindings = Vec::new();
            self.lowerer.match_template_arguments(
                &parameters,
                declared,
                expected,
                &mut bindings,
            )?;
            let bindings = self
                .lowerer
                .instance_bindings(&bindings, &self.type_substitution)?;
            let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();

            return self.type_lowerer().generic_instance_key(symbol, &arguments);
        }

        Ok(GenericInstanceKey::non_generic(symbol))
    }
}
