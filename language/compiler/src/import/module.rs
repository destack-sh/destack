use dyst_ast as ast;
use dyst_dir::{Module, ScopeId, ScopeKind};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Import a module from AST into DIR in a given parent/root scope.
    pub fn import_module(
        &mut self,
        mut module: Module,
        scope_id: Option<ScopeId>,
        expressions: &[ast::NodeId<ast::Expression>],
    ) {
        // create scope
        let scope_id = self
            .session
            .tree
            .create_scope(ScopeKind::Module, scope_id, None);

        // lower expressions
        let expressions: Vec<_> = expressions
            .iter()
            .map(|expression| self.lower_expression(&module, scope_id, *expression))
            .collect();
        module.expressions.extend(expressions);
        self.session.modules.insert(module);
    }
}
