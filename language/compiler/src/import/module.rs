use dyst_ast as ast;
use dyst_dir::{Module, NodeTree, ScopeId, ScopeKind};

use crate::{Compiler, ResolveTask};

impl<'a> Compiler<'a> {
    /// Import a module from AST into DIR in a given parent/root scope.
    pub fn import_module(
        &self,
        mut module: Module,
        scope_id: Option<ScopeId>,
        expressions: &[ast::NodeId<ast::Expression>],
        tree: &mut NodeTree,
    ) {
        let module_id = module.id;

        // create scope
        let scope_id = tree.create_scope(ScopeKind::Module, scope_id, None);
        module.scope = Some(scope_id);

        // lower expressions
        let expressions: Vec<_> = expressions
            .iter()
            .map(|expression| self.lower_expression(&module, scope_id, *expression, tree))
            .collect();
        module.expressions.extend(expressions);
        self.session.modules.insert(module);

        // begin resolving
        self.enqueue(ResolveTask::ResolveModule { module: module_id }.into());
    }
}
