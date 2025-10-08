use crate::{AstNodeId, Compiler};
use dyst_ast as ast;
use dyst_dir::{Definition, NodeId};

impl<'a> Compiler<'a> {
    /// Lower a definition to a DIR definition.
    pub fn lower_definition(
        &mut self,
        ast: &ast::NodeTree,
        definition_id: ast::NodeId<ast::Definition>,
    ) -> Option<NodeId<Definition>> {
        let definition = ast.get(definition_id);
        match definition {
            ast::Definition::Module { .. } => todo!(),
            _ => todo!(
                "Compiler.lower_definition not implemented for {:?}",
                definition
            ),
        }
    }
}
