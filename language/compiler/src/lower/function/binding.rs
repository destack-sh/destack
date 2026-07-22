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
            let declarator = self.lowerer.source().tree().get(*declarator_id);
            let (pattern, value) = (declarator.pattern, declarator.value);
            let dir::Pattern::Binding { pattern: None, .. } =
                self.lowerer.source().tree().get(pattern)
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
            let node = pattern.into_global_any(self.lowerer.source);
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
                    let ty = self.lowerer.lower_type_id(self.builder.tree_mut(), ty)?;
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
        // the checked resolution names the writable place
        let dir::AssignPatternResolution::Place(resolution) =
            self.lowerer.assign_resolution(left)?
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a destructuring assignment".to_string(),
            }
            .into());
        };
        let dir::AssignPattern::Place { expression: target } =
            *self.lowerer.source().tree().get(left)
        else {
            return Err(CompilerError::Internal {
                message: "checked DIR resolved a non-place pattern as a place".to_string(),
            });
        };
        let place = self.place(&resolution)?;

        // store the right value directly for plain assignment
        if operator == dir::AssignOperator::Assign {
            let value = self.lower_expression(right)?;
            self.write_place(&place, value)?;

            return Ok(());
        }

        // apply the checked builtin operation for compound assignment
        let dir::OperatorResolution::Builtin = self.lowerer.operator_resolution(statement)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a protocol compound assignment".to_string(),
            }
            .into());
        };
        let Some(operator) = operator.binary_operator() else {
            return Err(CompilerError::Internal {
                message: "compound assignment has no binary operator".to_string(),
            });
        };
        let carrier = self.lowerer.coerced_type(target)?;
        let operator = self.binary_operator(operator, &carrier)?;
        let current = self.read_place(&place)?;
        let value = self.lower_expression(right)?;
        let value = self.builder.binary_op(operator, current, value);
        self.write_place(&place, value)?;

        Ok(())
    }
}
