use crate::{AstNodeId, Compiler};
use dyst_dir::{Definition, NodeId};
use {dyst_ast as ast, dyst_dir as dir};

impl<'a> Compiler<'a> {
    pub fn lower_definition(
        &mut self,
        definition: AstNodeId<ast::Definition>,
    ) -> Option<NodeId<Definition>> {
        todo!("Compiler.lower_definition")
    }
}
