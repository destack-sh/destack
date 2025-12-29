use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

impl Compiler {
    /// Unbind a DIR dependency kind to an AST dependency kind.
    #[inline]
    pub(super) fn unbind_dependency_kind(
        &self,
        _context: &mut UnbindContext,
        kind: dir::DependencyKind,
    ) -> ast::DependencyKind {
        match kind {
            dir::DependencyKind::Type => ast::DependencyKind::Type,
            dir::DependencyKind::Value => ast::DependencyKind::Value,
        }
    }

    /// Unbind a DIR dependency mode to an AST dependency mode.
    #[inline]
    pub(super) fn unbind_dependency_mode(
        &self,
        _context: &mut UnbindContext,
        mode: dir::DependencyMode,
    ) -> ast::DependencyMode {
        match mode {
            dir::DependencyMode::Item => ast::DependencyMode::Item,
            dir::DependencyMode::Default => ast::DependencyMode::Default,
            dir::DependencyMode::Namespace => ast::DependencyMode::Namespace,
        }
    }

    /// Unbind a DIR dependency item to an AST dependency item.
    pub(super) fn unbind_dependency_item(
        &self,
        module: &Module,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::DependencyItem> {
        let item = tree.get(item_id);
        let span = self.unbind_span(module, item_id.into());
        let ast_item = match item {
            dir::DependencyItem::Value { mode, value } => {
                let mode = self.unbind_dependency_mode(context, *mode);
                let value =
                    self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings, context);
                ast::DependencyItem {
                    kind: None,
                    mode,
                    name: None,
                    alias: None,
                    value: Some(value),
                }
            }
            dir::DependencyItem::UnresolvedRemote {
                mode,
                kind,
                name,
                alias,
                ..
            } => {
                let mode = self.unbind_dependency_mode(context, *mode);
                let kind = Some(self.unbind_dependency_kind(context, *kind));
                let name = name.map(|n| ast_strings.intern_from(&self.program.strings, n));
                let alias = alias.map(|a| ast_strings.intern_from(&self.program.strings, a));
                ast::DependencyItem {
                    kind,
                    mode,
                    name,
                    alias,
                    value: None,
                }
            }
            dir::DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
                ..
            }
            | dir::DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                ..
            } => {
                let mode = self.unbind_dependency_mode(context, *mode);
                let kind = Some(self.unbind_dependency_kind(context, *kind));
                let name = name.map(|n| ast_strings.intern_from(&self.program.strings, n));
                let alias = alias.map(|a| ast_strings.intern_from(&self.program.strings, a));
                ast::DependencyItem {
                    kind,
                    mode,
                    name,
                    alias,
                    value: None,
                }
            }
            dir::DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                ..
            } => {
                let mode = self.unbind_dependency_mode(context, *mode);
                let kind = Some(self.unbind_dependency_kind(context, *kind));
                let name = name.map(|n| ast_strings.intern_from(&self.program.strings, n));
                let alias = alias.map(|a| ast_strings.intern_from(&self.program.strings, a));
                ast::DependencyItem {
                    kind,
                    mode,
                    name,
                    alias,
                    value: None,
                }
            }
        };
        let ast_item_id = ast_tree.insert(ast_item, span);
        context.map(item_id.into_any(), ast_item_id.into_any());
        ast_item_id
    }
}
