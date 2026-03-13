use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_source::{FileId, ModuleId, Span};
use destack_workspace::{Module, ModuleDirData, ProfileId};

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
    /// Return the most advanced published DIR snapshot for one module and profile.
    pub(super) fn unbind_dir_snapshot(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Option<ModuleDirData> {
        self.program
            .artifacts
            .dir_snapshot(module_id, profile)
            .map(|dir| dir.as_ref().clone())
    }

    /// Get the span for a DIR node.
    #[inline]
    pub(super) fn unbind_span(&self, _module: &Module, _node_id: dir::LocalNodeIdAny) -> Span {
        // #Incomplete: resolve back to original AST span via source_node_id mapping?
        Span::empty(FileId::new(0))
    }

    /// Unbind a module's DIR tree to an AST tree.
    pub fn unbind_module(&self, module: &Module, profile: ProfileId) -> UnboundModule {
        // read the most advanced committed artifact snapshot
        let dir = self
            .unbind_dir_snapshot(module.id, profile)
            .map(std::sync::Arc::new)
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
