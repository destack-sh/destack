use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower one expression into a JS statement.
    pub fn lower_expression_as_statement(
        &mut self,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Statement>> {
        let lowered_id = self.lower_expression(source_expression_id)?;

        let statement_id = match lowered_id.ty {
            js::NodeType::Statement => lowered_id.try_into().unwrap(),
            js::NodeType::Expression => {
                let lowered_expression_id: js::LocalNodeId<js::Expression> =
                    lowered_id.try_into().unwrap();
                let statement = js::Statement::Expression {
                    expression: lowered_expression_id,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, source_expression_id)
            }
            js::NodeType::Block => {
                let block_id: js::LocalNodeId<js::Block> = lowered_id.try_into().unwrap();
                let statement = js::Statement::Block { block: block_id };
                self.tree
                    .insert_from_source(statement, self.module.id, source_expression_id)
            }
            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_expression_id.into_global_any(self.module.id),
                    message: Some(format!(
                        "statement lowering expected expression, statement, or block, got {}",
                        lowered_id.ty.name()
                    )),
                });
            }
        };

        Ok(statement_id)
    }

    /// Lower one expression into a JS block.
    pub fn lower_expression_as_block(
        &mut self,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Block>> {
        let lowered_id = self.lower_expression(source_expression_id)?;

        let block_id = match lowered_id.ty {
            js::NodeType::Block => lowered_id.try_into().unwrap(),
            js::NodeType::Statement => {
                let statement_id: js::LocalNodeId<js::Statement> = lowered_id.try_into().unwrap();
                let block = js::Block {
                    statements: vec![statement_id],
                };
                self.tree
                    .insert_from_source(block, self.module.id, source_expression_id)
            }
            js::NodeType::Expression => {
                let lowered_expression_id: js::LocalNodeId<js::Expression> =
                    lowered_id.try_into().unwrap();
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
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_expression_id.into_global_any(self.module.id),
                    message: Some(format!(
                        "block lowering expected expression, statement, or block, got {}",
                        lowered_id.ty.name()
                    )),
                });
            }
        };

        Ok(block_id)
    }

    /// Lower a block from DIR into JS AST.
    pub fn lower_block(
        &mut self,
        block_id: dir::LocalNodeId<dir::Block>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Block>> {
        let block = self.dir_tree.get(block_id);
        let statements = block
            .iter_expressions()
            .map(|statement| self.lower_expression_as_statement(statement))
            .collect::<Result<Vec<_>, CodegenJsError>>()?;
        let block = js::Block { statements };
        Ok(self
            .tree
            .insert_from_source(block, self.module.id, block_id))
    }
}
