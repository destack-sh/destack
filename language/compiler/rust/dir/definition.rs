use crate::Compiler;
use {dyst_ast as ast, dyst_dir as dir};

impl<'a> Compiler<'a> {
    pub fn lower_definition(
        &mut self,
        definition: ast::NodeId<ast::Definition>,
    ) -> Option<dir::NodeId<dir::Definition>> {
        todo!()
    }
}
