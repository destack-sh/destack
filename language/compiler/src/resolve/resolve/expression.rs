use destack_dir as dir;
use smallvec::smallvec;

use crate::resolve::state::{PathReference, ResolveState};

impl ResolveState<'_> {
    /// Walk one expression and collect references and syntax language items.
    ///
    /// Example:
    /// ```ds
    /// import { value } from "./dep.ds";
    /// dep.api.value + value;
    /// ```
    pub(in crate::resolve) fn walk_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        match expression {
            dir::Expression::Identifier { name } => {
                self.collect_path_reference(PathReference {
                    source: id.into_global_any(self.module),
                    path: dir::Path {
                        segments: smallvec![*name],
                    },
                    space: dir::SymbolSpace::Declaration,
                });
            }
            dir::Expression::Member { .. } => {
                // collect the path once, at the outermost member of a chain
                if self.member_chain_depth == 0
                    && let Some(path) = tree.reference_path(id)
                    && path.segments.len() > 1
                {
                    self.collect_path_reference(PathReference {
                        source: id.into_global_any(self.module),
                        path,
                        space: dir::SymbolSpace::Declaration,
                    });
                }

                self.record_member_owner_language_items();

                // mark the nested members so only the outermost collects
                self.member_chain_depth += 1;
                dir::walk_expression(self, tree, id, expression);
                self.member_chain_depth -= 1;
            }
            dir::Expression::ForEach {
                operator: dir::ForEachOperator::Of,
                ..
            } => {
                self.record_language_item(dir::LanguageItem::Iterable);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Await { .. } => {
                self.record_language_item(dir::LanguageItem::Promise);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::AwaitMaybe { .. } | dir::Expression::AwaitMust { .. } => {
                self.record_language_item(dir::LanguageItem::Promise);
                self.record_try_language_items();
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Yield {
                cardinality: dir::YieldCardinality::Generator,
                ..
            } => {
                self.record_yield_star_language_item();
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::ImportMeta => {
                self.record_language_item(dir::LanguageItem::ImportMeta);
            }
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::RegexString { .. }) => {
                self.record_language_item(dir::LanguageItem::RegExp);
            }
            dir::Expression::Type { .. } => {
                self.record_language_item(dir::LanguageItem::Type);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::BorrowOf { .. } => {
                self.record_language_item(dir::LanguageItem::Lifetime);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::RangeExpression {
                start,
                end,
                end_kind,
            } => {
                self.record_range_language_item(start.is_some(), end.is_some(), *end_kind);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::ArrayExpression { .. } => {
                self.record_language_item(dir::LanguageItem::Array);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::FixedArrayExpression { .. } => {
                self.record_language_item(dir::LanguageItem::FixedArray);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Index { .. } => {
                self.record_language_item(dir::LanguageItem::Index);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Unary { operator, .. } => {
                self.record_unary_operator_language_items(*operator);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Binary { operator, .. } => {
                self.record_binary_operator_language_items(*operator);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Maybe { .. } | dir::Expression::Must { .. } => {
                self.record_try_language_items();
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Import { .. } => {}
            dir::Expression::Export {
                target: Some(_), ..
            } => {}
            _ => dir::walk_expression(self, tree, id, expression),
        }
    }

    /// Walk one type expression and collect references and syntax language items.
    ///
    /// Example:
    /// ```ds
    /// let value: dep.Model[];
    /// ```
    pub(in crate::resolve) fn walk_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        ty: &dir::TypeExpression,
    ) {
        match ty {
            dir::TypeExpression::ScalarLiteral { value } => {
                if let Some(item) = value.representation_item() {
                    self.record_language_item(item);
                }
            }
            dir::TypeExpression::Literal { value } => {
                if let Some(item) = value.representation_item() {
                    self.record_language_item(item);
                }
            }
            dir::TypeExpression::Reference { path, .. } if path.segments.len() == 1 => {
                self.collect_path_reference(PathReference {
                    source: id.into_global_any(self.module),
                    path: path.clone(),
                    space: dir::SymbolSpace::Declaration,
                });
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::Reference { path, .. } => {
                self.collect_path_reference(PathReference {
                    source: id.into_global_any(self.module),
                    path: path.clone(),
                    space: dir::SymbolSpace::Declaration,
                });
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::BorrowedOf { .. } => {
                self.record_language_item(dir::LanguageItem::Lifetime);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::Array { .. } => {
                self.record_language_item(dir::LanguageItem::Array);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::Slice { .. } => {
                self.record_language_item(dir::LanguageItem::Slice);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::FixedArray { .. } => {
                self.record_language_item(dir::LanguageItem::FixedArray);
                dir::walk_type_expression(self, tree, id, ty);
            }
            _ => dir::walk_type_expression(self, tree, id, ty),
        }
    }
}
