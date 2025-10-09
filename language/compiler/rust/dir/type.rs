use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{NodeId, Type};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower a an expression into a type (without evaluating it at all).
    pub fn lower_expression_to_type(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> NodeId<Type> {
        let expression = self.lower_expression(source_id, ast, expression_id);
        self.tree
            .allocate(Type::Expression(expression), source_id, expression_id)
    }
}
