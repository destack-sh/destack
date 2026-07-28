use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::body::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one let statement's declarators.
    pub(in crate::lower) fn lower_let(
        &mut self,
        mutability: dir::Mutability,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) -> CompilerResult<()> {
        for declarator_id in declarators {
            let declarator = self.source().tree().get(*declarator_id);
            let (pattern, value) = (declarator.pattern, declarator.value);
            let dir::Pattern::Binding { pattern: None, .. } = self.source().tree().get(pattern)
            else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a destructuring let binding".to_string(),
                }
                .into());
            };
            let Some(value) = value else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "an uninitialized let binding".to_string(),
                }
                .into());
            };

            // resolve the binding symbol at the pattern node
            let node = pattern.into_global_any(self.source);
            let Some(symbol) = self.lowerer.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "checked DIR is missing a symbol for one let binding".to_string(),
                });
            };
            let value = self.lower_expression(value)?;

            // keep immutable bindings as pure values; give mutable ones a local
            let binding = match mutability {
                dir::Mutability::Immutable => Binding::Value(value),
                _ => {
                    let ty = self.lowerer.symbol_type(symbol)?;
                    let ty = self.lower_type(ty)?;
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
        // the checked pattern identifies a place assignment
        let dir::AssignPatternResolution::Place = self.assign_resolution(left)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a destructuring assignment".to_string(),
            }
            .into());
        };
        let dir::AssignPattern::Place { expression } = *self.source().tree().get(left) else {
            return Err(CompilerError::Internal {
                message: "checked DIR resolved a non-place pattern as a place".to_string(),
            });
        };
        let resolution = self.assignment_resolution(expression)?;
        let place = self.place(&resolution)?;

        // store the right value directly for plain assignment
        if operator == dir::AssignOperator::Assign {
            let value = self.lower_expression(right)?;
            self.write_place(&place, value)?;

            return Ok(());
        }

        // apply the checked builtin operation for compound assignment
        let resolution = self.operator_resolution(statement)?;
        let dir::OperationResolution::One(dir::OperatorApplication::Binary {
            operator,
            target: dir::OperatorTarget::Builtin(_),
            ..
        }) = resolution
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a protocol compound assignment".to_string(),
            }
            .into());
        };
        let current = self.read_place(&place)?;
        let operator = self.binary_value_operator(operator, current)?;
        let value = self.lower_expression(right)?;
        let value = self.builder.binary_op(operator, current, value);
        self.write_place(&place, value)?;

        Ok(())
    }
}
