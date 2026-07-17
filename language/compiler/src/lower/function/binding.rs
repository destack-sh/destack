use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::function::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_> {
    /// Lower one let statement's declarators.
    pub(in crate::lower) fn lower_let(
        &mut self,
        mutability: dir::Mutability,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) -> CompilerResult<()> {
        for declarator_id in declarators {
            let declarator = self.lowerer.tree.get(*declarator_id).clone();
            let dir::Pattern::Binding { pattern: None, .. } =
                self.lowerer.tree.get(declarator.pattern)
            else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a destructuring let binding".to_string(),
                }
                .into());
            };
            let Some(value) = declarator.value else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "an uninitialized let binding".to_string(),
                }
                .into());
            };

            // resolve the binding symbol at the pattern node
            let node = declarator.pattern.into_global_any(self.lowerer.module);
            let Some(symbol) = self.lowerer.symbol_declared_at(node) else {
                return Err(CompilerError::Internal {
                    message: "checked DIR is missing a symbol for one let binding".to_string(),
                });
            };
            let value = self.lower_expression(value)?;

            // keep immutable bindings as pure values; give mutable ones a local
            let binding = match mutability {
                dir::Mutability::Immutable => Binding::Value(value),
                _ => {
                    let ty = self.lowerer.ty(self.lowerer.symbol_type(symbol)?)?;
                    let ty = self.lowerer.lower_type(&ty)?;
                    let ty = self.builder.tree_mut().insert_type(ty);
                    let local = self.builder.local(ty, mir::Mutability::Mutable);
                    self.builder.local_set(local, value);

                    Binding::Local(local)
                }
            };
            self.values.insert(symbol.local_id, binding);
        }

        Ok(())
    }

    /// Lower one assignment statement.
    pub(in crate::lower) fn lower_assign(
        &mut self,
        statement: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // write through the checked place behind the target
        let dir::AssignPatternResolution::Place(place) = self.lowerer.assign_resolution(left)?
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a destructuring assignment".to_string(),
            }
            .into());
        };
        let dir::Storage::Binding { symbol } = place.storage else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an assignment through a projection".to_string(),
            }
            .into());
        };
        let Some(Binding::Local(local)) = self.values.get(&symbol.local_id).copied() else {
            return Err(CompilerError::Internal {
                message: "checked DIR assigned a binding without a mutable local".to_string(),
            });
        };

        // store the right value directly for plain assignment
        if operator == dir::AssignOperator::Assign {
            let value = self.lower_expression(right)?;
            self.builder.local_set(local, value);

            return Ok(());
        }

        // apply the checked builtin operation for compound assignment
        let resolution = self.lowerer.call_resolution(statement)?;
        let dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator }) =
            resolution.target
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a protocol compound assignment".to_string(),
            }
            .into());
        };
        let carrier = self.lowerer.ty(place.ty)?;
        let operator = self.binary_operator(operator, &carrier)?;
        let current = self.builder.local_get(local);
        let value = self.lower_expression(right)?;
        let value = self.builder.binary_op(operator, current, value);
        self.builder.local_set(local, value);

        Ok(())
    }
}
