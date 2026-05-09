use crate::{Compiler, DeclareResult};
use destack_artifact::Ast;
use destack_dir::{
    BindingTable, DeclaredModule, Expression, LocalNodeId, LocalScopeId, Tree, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::ProviderContext;

impl Compiler {
    /// Bind a module's AST to DIR (create symbols, scopes, and base DIR).
    pub(crate) fn declare_module_bind(
        &self,
        module_id: ModuleId,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
        types: &mut TypeTable,
        roots: &mut Vec<LocalNodeId<Expression>>,
        context: &dyn ProviderContext,
    ) -> DeclareResult<()> {
        // syntax-only modules stop at AST
        if !self.is_code_module(context.revision(), module_id) {
            return Ok(());
        }

        let module = self.module(context.revision(), module_id);

        // bind module roots
        let bound_roots = {
            let module = module.as_ref();
            self.declare_module_roots(
                module,
                ast,
                namespace_scope,
                global_scope,
                declared_modules,
                tree,
                symbols,
                types,
            )
        };
        roots.extend(bound_roots);

        // attach annotations
        {
            let module = module.as_ref();
            let scope = (namespace_scope, symbols.get_scope_mark(namespace_scope));
            self.attach_annotations(
                module,
                ast,
                scope,
                namespace_scope,
                global_scope,
                declared_modules,
                tree,
                symbols,
                types,
                context,
            );
        }

        // mark global symbols
        {
            let module = module.as_ref();
            self.mark_global_symbols(module, tree, symbols, global_scope);
        }

        // copy final source spans into DIR
        {
            let module = module.as_ref();
            for node_id in tree.first_global_id()..tree.next_global_id() {
                let source_id = tree.get_source(node_id);
                let span = ast.tree.get_span_by_id(source_id).with_file(module.file_id);
                tree.set_span(node_id, span);
            }
        }

        Ok(())
    }
}
