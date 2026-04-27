use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR decorator position to an AST decorator position.
    #[inline]
    pub(super) fn unbind_decorator_position(
        &self,
        _context: &mut UnbindContext,
        position: dir::DecoratorPosition,
    ) -> ast::DecoratorPosition {
        match position {
            dir::DecoratorPosition::BlockInfix => ast::DecoratorPosition::BlockInfix,
            dir::DecoratorPosition::BlockPrefix => ast::DecoratorPosition::BlockPrefix,
            dir::DecoratorPosition::BlockPostfix => ast::DecoratorPosition::BlockPostfix,
            dir::DecoratorPosition::LinePrefix => ast::DecoratorPosition::LinePrefix,
            dir::DecoratorPosition::LinePostfix => ast::DecoratorPosition::LinePostfix,
        }
    }

    /// Unbind a DIR decorator to an AST decorator.
    pub(super) fn unbind_decorator(
        &self,
        module: &Module,
        decorator_id: dir::LocalNodeId<dir::Decorator>,
        tree: &dir::Tree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Decorator> {
        let decorator = tree.get(decorator_id);
        let span = self.unbind_span(module, decorator_id.into());
        let position = self.unbind_decorator_position(context, decorator.position);
        let expression = self.unbind_expression(
            module,
            decorator.expression,
            tree,
            symbols,
            types,
            ast_tree,
            ast_strings,
            context,
        );
        let ast_decorator = ast::Decorator {
            expression,
            position,
        };
        let ast_decorator_id = ast_tree.insert(ast_decorator, span);
        context.map(decorator_id.into_any(), ast_decorator_id.into_any());
        ast_decorator_id
    }

    /// Attach all decorators for a DIR module to the given AST tree.
    pub(super) fn attach_unbind_decorators(
        &self,
        module: &Module,
        tree: &dir::Tree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) {
        let nodes: Vec<(dir::LocalNodeIdAny, ast::LocalNodeIdAny)> = context
            .node_map
            .iter()
            .map(|(dir_id, ast_id)| (*dir_id, *ast_id))
            .collect();
        for (dir_id, ast_id) in nodes {
            let decorators = tree.get_decorators(dir_id.id);
            for decorator_id in decorators {
                let ast_decorator_id = self.unbind_decorator(
                    module,
                    decorator_id,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast_tree.append_decorator(ast_id.id, ast_decorator_id);
            }
        }
    }
}
