use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_source::{FileId, Span};
use destack_workspace::{Module, ProfileId};

use super::UnbindContext;
use crate::Compiler;

/// Result of unbinding a module.
#[derive(Debug)]
pub struct UnboundModule {
    /// The AST tree.
    pub tree: ast::Tree,
    /// The string pool.
    pub strings: StringPool,
    /// The root expressions.
    pub roots: Vec<ast::LocalNodeId<ast::Expression>>,
}

impl Compiler {
    /// Get the span for a DIR node.
    #[inline]
    pub(super) fn unbind_span(&self, _module: &Module, _node_id: dir::LocalNodeIdAny) -> Span {
        // #Incomplete: resolve back to original AST span via source_node_id mapping?
        Span::empty(FileId::new(0))
    }

    /// Unbind a module's DIR tree to an AST tree.
    pub fn unbind_module(&self, module: &Module, profile: ProfileId) -> UnboundModule {
        if let Some(dir) = self.dir_patched(module.id, profile) {
            return self.unbind_module_from_parts(
                module,
                &dir.tree,
                &dir.symbols,
                &dir.types,
                &dir.roots,
            );
        }

        if let Some(dir) = self.dir_elaborated(module.id, profile) {
            return self.unbind_module_from_parts(
                module,
                &dir.tree,
                &dir.symbols,
                &dir.types,
                &dir.roots,
            );
        }

        if let Some(dir) = self.dir_analyzed(module.id, profile) {
            return self.unbind_module_from_parts(
                module,
                &dir.tree,
                &dir.symbols,
                &dir.types,
                &dir.roots,
            );
        }

        if let Some(dir) = self.dir_interface(module.id, profile) {
            return self.unbind_module_from_parts(
                module,
                &dir.tree,
                &dir.symbols,
                &dir.types,
                &dir.roots,
            );
        }

        if let Some(dir) = self.dir_declared(module.id, profile) {
            return self.unbind_module_from_parts(
                module,
                &dir.tree,
                &dir.symbols,
                &dir.types,
                &dir.roots,
            );
        }

        if let Some(dir) = self.dir_resolved(module.id, profile) {
            return self.unbind_module_from_parts(
                module,
                &dir.tree,
                &dir.symbols,
                &dir.types,
                &dir.roots,
            );
        }

        if let Some(dir) = self.dir_prepared(module.id, profile) {
            return self.unbind_module_from_parts(
                module,
                &dir.tree,
                &dir.symbols,
                &dir.types,
                &dir.roots,
            );
        }

        if let Some(dir) = self.dir_base(module.id) {
            return self.unbind_module_from_parts(
                module,
                &dir.tree,
                &dir.symbols,
                &dir.types,
                &dir.roots,
            );
        }

        panic!("missing committed dir artifact for module {:?}", module.id);
    }

    /// Unbind module parts into an AST tree.
    pub(crate) fn unbind_module_from_parts(
        &self,
        module: &Module,
        tree: &dir::Tree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        roots: &[dir::LocalNodeId<dir::Expression>],
    ) -> UnboundModule {
        // initialize the unbind context
        let mut context = UnbindContext::new();

        // rebuild the AST tree
        let mut ast_tree = ast::Tree::new();
        let mut ast_strings = StringPool::new();
        let roots: Vec<ast::LocalNodeId<ast::Expression>> = roots
            .iter()
            .map(|expression_id| {
                self.unbind_expression(
                    module,
                    *expression_id,
                    tree,
                    symbols,
                    types,
                    &mut ast_tree,
                    &mut ast_strings,
                    &mut context,
                )
            })
            .collect();

        // attach decorators after all nodes have been unbound
        self.attach_unbind_decorators(
            module,
            tree,
            symbols,
            types,
            &mut ast_tree,
            &mut ast_strings,
            &mut context,
        );

        // return the unbound module
        UnboundModule {
            tree: ast_tree,
            strings: ast_strings,
            roots,
        }
    }
}
