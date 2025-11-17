use dyst_ast as ast;
use dyst_dir::Module;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a module into DIR.
    pub fn lower_module(
        &mut self,
        mut module: Module,
        expressions: &Vec<ast::NodeId<ast::Expression>>,
    ) {
        let expressions: Vec<_> = expressions
            .iter()
            .map(|expression| self.lower_expression(&module, *expression))
            .collect();
        module.expressions.extend(expressions);
        self.session.modules.insert(module);
    }
}
