use std::collections::HashMap;

use destack_dir::{Expression, GlobalSymbolId, LocalNodeId};
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::TypeLowerer;

/// Track whether a statement terminates control flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Terminates {
    /// The statement terminates control flow.
    Yes,
    /// The statement falls through to the next block.
    No,
}

impl Terminates {
    /// Return true if the statement terminates control flow.
    pub(crate) fn is_yes(self) -> bool {
        matches!(self, Self::Yes)
    }
}

/// Lower statement-level expressions into MIR blocks.
pub(crate) struct BlockLowerer<'a, 'b> {
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: &'a TypeLowerer,
    /// Resolve direct calls for known function symbols.
    pub(crate) functions_by_symbol: &'a HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Emit MIR into the current function builder.
    pub(crate) builder: &'b mut mir::FunctionBuilder<'a>,
    /// Track locals by symbol for variable resolution.
    pub(crate) locals_by_symbol: &'b mut HashMap<GlobalSymbolId, mir::Variable>,
}

impl<'a, 'b> BlockLowerer<'a, 'b> {
    /// Lower a statement expression.
    pub(crate) fn lower_statement_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<Terminates> {
        match self.dir_tree.get(expression_id) {
            Expression::Statement { statement } => self.lower_statement_expression(*statement),
            Expression::Block { block } => {
                let block = self.dir_tree.get(*block);
                for expr_id in &block.expressions {
                    let terminated = self.lower_statement_expression(*expr_id)?;
                    if terminated.is_yes() {
                        return Ok(Terminates::Yes);
                    }
                }
                Ok(Terminates::No)
            }
            Expression::Return { value } => {
                let return_value = if let Some(value) = value {
                    let (value, _) = self.lower_value_expression(*value)?;
                    Some(value)
                } else {
                    None
                };
                self.builder.return_(return_value);
                Ok(Terminates::Yes)
            }
            Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => self.lower_if_statement(*condition, *then_expression, *else_expression),

            _ => {
                let _ = self.lower_value_expression(expression_id)?;
                Ok(Terminates::No)
            }
        }
    }

    /// Lower an if statement into blocks and branches.
    fn lower_if_statement(
        &mut self,
        condition_id: LocalNodeId<Expression>,
        then_id: LocalNodeId<Expression>,
        else_id: Option<LocalNodeId<Expression>>,
    ) -> LowerResult<Terminates> {
        let (condition_value, condition_type) = self.lower_value_expression(condition_id)?;
        if condition_type != self.type_lowerer.ty_bool {
            return Err(LowerError::UnsupportedConstruct {
                node: condition_id.into_global_any(self.module_id),
                message: "condition type is not boolean".to_string(),
            })?;
        }

        let then_block = self.builder.create_block();
        let else_block = self.builder.create_block();
        let join_block = self.builder.create_block();

        // branch
        self.builder.branch(condition_value, then_block, else_block);

        // then branch
        self.builder.switch_to_block(then_block);
        let then_terminated = self.lower_statement_expression(then_id)?;
        if !then_terminated.is_yes() {
            self.builder.jump(join_block);
        }

        // else branch
        self.builder.switch_to_block(else_block);
        let else_terminated = if let Some(else_id) = else_id {
            self.lower_statement_expression(else_id)?
        } else {
            Terminates::No
        };
        if !else_terminated.is_yes() {
            self.builder.jump(join_block);
        }

        // join
        if then_terminated.is_yes() && else_terminated.is_yes() {
            return Ok(Terminates::Yes);
        }
        self.builder.switch_to_block(join_block);
        Ok(Terminates::No)
    }
}
