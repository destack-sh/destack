use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_source::{FileId, ModuleId, Span};
use destack_workspace::{Module, ModuleDir, ProfileId};
use std::sync::Arc;

use super::UnbindContext;
use crate::Compiler;

/// Result of unbinding a module.
#[derive(Debug)]
pub struct UnboundModule {
    /// The AST tree.
    pub tree: ast::NodeTree,
    /// The string pool.
    pub strings: StringPool,
    /// The root expressions.
    pub roots: Vec<ast::LocalNodeId<ast::Expression>>,
}

impl Compiler {
    /// Return the most advanced DIR artifact available for unbinding.
    pub(super) fn best_unbind_dir(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<ModuleDir>> {
        for dir in [
            self.program.artifacts.dir_patched(module_id, profile),
            self.program.artifacts.dir_elaborated(module_id, profile),
            self.program.artifacts.dir_analyzed(module_id, profile),
            self.program.artifacts.dir_interface(module_id, profile),
            self.program.artifacts.dir_declared(module_id, profile),
            self.program.artifacts.dir_resolved(module_id, profile),
            self.program.artifacts.dir_prepared(module_id, profile),
            self.program.artifacts.dir_base(module_id),
        ] {
            if let Some(dir) = dir {
                return Some(dir);
            }
        }

        None
    }

    /// Get the span for a DIR node.
    #[inline]
    pub(super) fn unbind_span(&self, _module: &Module, _node_id: dir::LocalNodeIdAny) -> Span {
        // #Incomplete: resolve back to original AST span via source_node_id mapping?
        Span::empty(FileId::new(0))
    }

    /// Unbind a module's DIR tree to an AST tree.
    pub fn unbind_module(&self, module: &Module, profile: ProfileId) -> UnboundModule {
        // read the most advanced committed dir artifact
        let dir = self
            .best_unbind_dir(module.id, profile)
            .unwrap_or_else(|| panic!("missing committed dir artifact for module {:?}", module.id));
        let fallback_node = dir
            .roots
            .first()
            .copied()
            .map(dir::LocalNodeId::into_any)
            .unwrap_or(dir.anchor_node);
        self.unbind_module_from_parts(
            module,
            &dir.tree,
            &dir.symbols,
            &dir.roots,
            fallback_node,
            profile,
        )
    }

    /// Unbind module parts into an AST tree.
    pub(crate) fn unbind_module_from_parts(
        &self,
        module: &Module,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        roots: &[dir::LocalNodeId<dir::Expression>],
        fallback_node: dir::LocalNodeIdAny,
        profile: ProfileId,
    ) -> UnboundModule {
        // initialize the unbind context
        let mut context = UnbindContext::new(profile, fallback_node);

        // rebuild the AST tree
        let mut ast_tree = ast::NodeTree::new();
        let mut ast_strings = StringPool::new();
        let roots: Vec<ast::LocalNodeId<ast::Expression>> = roots
            .iter()
            .map(|expression_id| {
                self.unbind_expression(
                    module,
                    *expression_id,
                    tree,
                    symbols,
                    &mut ast_tree,
                    &mut ast_strings,
                    &mut context,
                )
            })
            .collect();

        // attach annotations after all nodes have been unbound
        self.attach_unbind_annotations(
            module,
            tree,
            symbols,
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
