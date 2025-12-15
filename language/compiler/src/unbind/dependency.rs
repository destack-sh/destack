use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Unbind a DIR dependency kind to an AST dependency kind.
    #[inline]
    pub(super) fn unbind_dependency_kind(&self, kind: dir::DependencyKind) -> ast::DependencyKind {
        match kind {
            dir::DependencyKind::Type => ast::DependencyKind::Type,
            dir::DependencyKind::Value => ast::DependencyKind::Value,
        }
    }

    /// Unbind a DIR dependency mode to an AST dependency mode.
    #[inline]
    pub(super) fn unbind_dependency_mode(&self, mode: dir::DependencyMode) -> ast::DependencyMode {
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
    ) -> ast::LocalNodeId<ast::DependencyItem> {
        let item = tree.get(item_id);
        let span = self.unbind_span(module, item_id.into());
        let ast_item = match item {
            dir::DependencyItem::Value { value } => {
                let value =
                    self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings);
                ast::DependencyItem {
                    kind: None,
                    mode: ast::DependencyMode::Default,
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
                let mode = self.unbind_dependency_mode(*mode);
                let kind = Some(self.unbind_dependency_kind(*kind));
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
                let mode = self.unbind_dependency_mode(*mode);
                let kind = Some(self.unbind_dependency_kind(*kind));
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
                let mode = self.unbind_dependency_mode(*mode);
                let kind = Some(self.unbind_dependency_kind(*kind));
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
        ast_tree.insert(ast_item, span)
    }
}
