use destack_dir as dir;

use crate::resolve::state::{ModuleClause, ResolveState};

impl ResolveState<'_> {
    /// Walk one expression and collect dependency requirements.
    pub(in crate::resolve) fn walk_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        match expression {
            dir::Expression::Identifier { name } => {
                let source = id.into_any();
                let key = dir::StaticKey::Name(*name);

                self.require_global_reference(source, key, dir::SymbolSpace::Value);
            }
            dir::Expression::QualifiedReference { path, .. } if path.segments.len() == 1 => {
                let source = id.into_any();
                let key = dir::StaticKey::Name(path.segments[0]);

                self.require_global_reference(source, key, dir::SymbolSpace::Value);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::ForEach {
                operator: dir::ForEachOperator::Of,
                ..
            } => {
                self.require_syntax_language_item(dir::LanguageItem::Iterable);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Await { .. } => {
                self.require_syntax_language_item(dir::LanguageItem::Promise);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::AwaitMaybe { .. } | dir::Expression::AwaitMust { .. } => {
                self.require_syntax_language_item(dir::LanguageItem::Promise);
                self.require_try_language_items();
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Yield {
                cardinality: dir::YieldCardinality::Generator,
                ..
            } => {
                self.require_yield_star_language_item();
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::ImportMeta => {
                self.require_syntax_language_item(dir::LanguageItem::ImportMeta);
            }
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::RegexString { .. }) => {
                self.require_syntax_language_item(dir::LanguageItem::RegExp);
            }
            dir::Expression::Type { .. } => {
                self.require_syntax_language_item(dir::LanguageItem::Type);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::BorrowOf { .. } => {
                self.require_syntax_language_item(dir::LanguageItem::Lifetime);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::RangeExpression {
                start,
                end,
                end_kind,
            } => {
                self.require_range_language_item(start.is_some(), end.is_some(), *end_kind);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Unary { operator, .. } => {
                self.require_unary_operator_language_items(*operator);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Binary { operator, .. } => {
                self.require_binary_operator_language_items(*operator);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Maybe { .. } | dir::Expression::Must { .. } => {
                self.require_try_language_items();
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Import { items, .. } => {
                let clause = ModuleClause::Import {
                    expression_id: id,
                    items: items.clone(),
                };
                self.add_module_clause(clause);
            }
            dir::Expression::Export {
                target: Some(_),
                items,
                ..
            } => {
                let clause = ModuleClause::ReExport {
                    expression_id: id,
                    items: items.clone(),
                };
                self.add_module_clause(clause);
            }
            _ => dir::walk_expression(self, tree, id, expression),
        }
    }

    /// Walk one type expression and collect dependency requirements.
    pub(in crate::resolve) fn walk_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        ty: &dir::TypeExpression,
    ) {
        match ty {
            dir::TypeExpression::Reference { path, .. } if path.segments.len() == 1 => {
                let source = id.into_any();
                let key = dir::StaticKey::Name(path.segments[0]);

                self.require_global_reference(source, key, dir::SymbolSpace::Type);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::BorrowedOf { .. } => {
                self.require_syntax_language_item(dir::LanguageItem::Lifetime);
                dir::walk_type_expression(self, tree, id, ty);
            }
            _ => dir::walk_type_expression(self, tree, id, ty),
        }
    }
}
