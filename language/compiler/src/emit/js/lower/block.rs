use crate::EmitError;
use tspp_dir as dir;
use tspp_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower one expression into a JS statement.
    pub(crate) fn lower_expression_as_statement(
        &mut self,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<js::LocalNodeId<js::Statement>>, EmitError> {
        if self.expression_is_erased(source_expression_id) {
            return Ok(None);
        }

        let lowered_id = self.lower_expression(source_expression_id)?;

        let statement_id = match lowered_id.ty {
            js::NodeType::Statement => js::LocalNodeId::new(lowered_id.id),
            js::NodeType::Expression => {
                let lowered_expression_id = js::LocalNodeId::new(lowered_id.id);
                let statement = js::Statement::Expression {
                    expression: lowered_expression_id,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, source_expression_id)
            }
            js::NodeType::Block => {
                let block_id = js::LocalNodeId::new(lowered_id.id);
                let statement = js::Statement::Block { block: block_id };
                self.tree
                    .insert_from_source(statement, self.module.id, source_expression_id)
            }
            _ => {
                return Err(self.unhandled(
                    source_expression_id.into_global_any(self.module.id),
                    Some(format!(
                        "statement lowering expected expression, statement, or block, got {}",
                        lowered_id.ty.name()
                    )),
                ));
            }
        };

        Ok(Some(statement_id))
    }

    /// Lower one expression into a JS block.
    pub(crate) fn lower_expression_as_block(
        &mut self,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::Block>, EmitError> {
        let lowered_id = self.lower_expression(source_expression_id)?;

        let block_id = match lowered_id.ty {
            js::NodeType::Block => js::LocalNodeId::new(lowered_id.id),
            js::NodeType::Statement => {
                let statement_id = js::LocalNodeId::new(lowered_id.id);
                let block = js::Block {
                    statements: vec![statement_id],
                };
                self.tree
                    .insert_from_source(block, self.module.id, source_expression_id)
            }
            js::NodeType::Expression => {
                let lowered_expression_id = js::LocalNodeId::new(lowered_id.id);
                let statement = js::Statement::Expression {
                    expression: lowered_expression_id,
                };
                let statement_id =
                    self.tree
                        .insert_from_source(statement, self.module.id, source_expression_id);
                let block = js::Block {
                    statements: vec![statement_id],
                };
                self.tree
                    .insert_from_source(block, self.module.id, source_expression_id)
            }
            _ => {
                return Err(self.unhandled(
                    source_expression_id.into_global_any(self.module.id),
                    Some(format!(
                        "block lowering expected expression, statement, or block, got {}",
                        lowered_id.ty.name()
                    )),
                ));
            }
        };

        Ok(block_id)
    }

    /// Lower a block from DIR into JavaScript.
    pub(crate) fn lower_block(
        &mut self,
        block_id: dir::LocalNodeId<dir::Block>,
    ) -> Result<js::LocalNodeId<js::Block>, EmitError> {
        let block = self.dir_tree.get(block_id);
        let mut statements = Vec::with_capacity(block.len());
        for expression in block.iter_expressions() {
            if let Some(statement) = self.lower_expression_as_statement(expression)? {
                statements.push(statement);
            }
        }

        let block = js::Block { statements };

        Ok(self
            .tree
            .insert_from_source(block, self.module.id, block_id))
    }
}
