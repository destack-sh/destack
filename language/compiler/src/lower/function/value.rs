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
    ) -> CompilerResult<Option<mir::Value>> {
        let ty = self.lower_type(self.node_type_id(expression)?)?;
        let key = GenericInstanceKey::non_generic(symbol);

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
                let function = self
                    .function(&key)
                    .map_err(|_| self.foreign_function_error())?;
                let pointee = self.builder.tree_mut().void_type();
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
                let function = self
                    .function(&key)
                    .map_err(|_| self.foreign_function_error())?;

                Ok(Some(self.builder.function_addr(function, ty)))
            }
            _ => Ok(None),
        }
    }

    /// Return the error for a function the module never declared.
    fn foreign_function_error(&self) -> CompilerError {
        LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "a foreign or generic function value reference".to_string(),
        }
        .into()
    }
}
