use destack_dir::{Expression, LocalNodeId};

use crate::{LowerError, LowerResult};

use super::super::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a root expression.
    pub(crate) fn lower_root_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<()> {
        let expression = self.dir_tree.get(expression_id);
        match expression {
            Expression::Declaration { declaration } => {
                let declaration_id = *declaration;
                let declaration = self.dir_tree.get(declaration_id);
                self.lower_declaration(declaration_id, declaration)
            }
            Expression::Statement { statement } => {
                // unwrap statement wrapper and process the inner expression
                self.lower_root_expression(*statement)
            }
            Expression::Let {
                mutability,
                declarators,
                ..
            } => self.lower_module_let(expression_id, *mutability, declarators),
            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id.into_global_any(self.module_id),
                message: format!("unsupported root expression `{}`", expression.kind_name()),
            })?,
        }
    }
}
