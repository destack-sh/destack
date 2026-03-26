use crate::{
    Block, CodegenJsError, CodegenJsResult, Expression, LocalNodeId, ModuleLowerer, NodeType,
    Statement, Type,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower one expression into a JS statement.
    pub fn lower_expression_as_statement(
        &mut self,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<LocalNodeId<Statement>> {
        let lowered_id = self.lower_expression(source_expression_id)?;

        let statement_id = match lowered_id.ty {
            NodeType::Statement => lowered_id.try_into().unwrap(),
            NodeType::Expression => {
                let lowered_expression_id: LocalNodeId<Expression> = lowered_id.try_into().unwrap();
                let statement = Statement::Expression {
                    expression: lowered_expression_id,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, source_expression_id)
            }
            NodeType::Block => {
                let block_id: LocalNodeId<Block> = lowered_id.try_into().unwrap();
                let statement = Statement::Block { block: block_id };
                self.tree
                    .insert_from_source(statement, self.module.id, source_expression_id)
            }
            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_expression_id.into_global_any(self.module.id),
                    message: None,
                });
            }
        };

        Ok(statement_id)
    }

    /// Lower one expression into a JS block.
    pub fn lower_expression_as_block(
        &mut self,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<LocalNodeId<Block>> {
        let lowered_id = self.lower_expression(source_expression_id)?;

        let block_id = match lowered_id.ty {
            NodeType::Block => lowered_id.try_into().unwrap(),
            NodeType::Statement => {
                let statement_id: LocalNodeId<Statement> = lowered_id.try_into().unwrap();
                let block = Block {
                    statements: vec![statement_id],
                };
                self.tree
                    .insert_from_source(block, self.module.id, source_expression_id)
            }
            NodeType::Expression => {
                let lowered_expression_id: LocalNodeId<Expression> = lowered_id.try_into().unwrap();
                let statement = Statement::Expression {
                    expression: lowered_expression_id,
                };
                let statement_id =
                    self.tree
                        .insert_from_source(statement, self.module.id, source_expression_id);
                let block = Block {
                    statements: vec![statement_id],
                };
                self.tree
                    .insert_from_source(block, self.module.id, source_expression_id)
            }
            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_expression_id.into_global_any(self.module.id),
                    message: None,
                });
            }
        };

        Ok(block_id)
    }

    /// Lower one expression into a JS type.
    pub fn lower_expression_as_type(
        &mut self,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<LocalNodeId<Type>> {
        let lowered_id = self.lower_expression(source_expression_id)?;

        let type_id = match lowered_id.ty {
            NodeType::Type => lowered_id.try_into().unwrap(),
            NodeType::Expression => {
                let lowered_expression_id: LocalNodeId<Expression> = lowered_id.try_into().unwrap();
                let ty = Type::Expression(lowered_expression_id);
                self.tree.insert_from_source_any(
                    ty,
                    self.module.id,
                    source_expression_id.into_any(),
                )
            }
            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_expression_id.into_global_any(self.module.id),
                    message: None,
                });
            }
        };

        Ok(type_id)
    }

    /// Lower a block from DIR into JS AST.
    pub fn lower_block(
        &mut self,
        block_id: dir::LocalNodeId<dir::Block>,
    ) -> CodegenJsResult<LocalNodeId<Block>> {
        let block = self.dir_tree.get(block_id);
        let statements = block
            .expressions
            .iter()
            .map(|statement| self.lower_expression_as_statement(*statement))
            .collect::<Result<Vec<_>, CodegenJsError>>()?;
        let block = Block { statements };
        Ok(self
            .tree
            .insert_from_source(block, self.module.id, block_id))
    }
}
