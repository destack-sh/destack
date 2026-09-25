use smallvec::smallvec;
use tspp_dir as dir;

use crate::resolve::state::{PathReference, ResolveState};

impl ResolveState<'_> {
    /// Walk one expression and collect references and implied language items.
    ///
    /// Example:
    /// ```tspp
    /// import { value } from "./dep.tspp";
    /// dep.api.value + value;
    /// ```
    pub(in crate::resolve) fn walk_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        match expression {
            dir::Expression::Literal(value) => {
                if let Some(item) = value.representation_item() {
                    self.use_language_item(item);
                }
                if matches!(value, dir::Literal::RegexString { .. }) {
                    self.use_language_item(dir::LanguageItem::RegExp);
                }
            }
            dir::Expression::Identifier { name } => {
                self.collect_path_reference(PathReference {
                    source: id.into_global_any(self.module),
                    path: dir::Path {
                        segments: smallvec![*name],
                    },
                });
            }
            dir::Expression::Member { left, .. } => {
                // collect the complete contiguous name path
                if let Some(path) = tree.reference_path(id)
                    && path.segments.len() > 1
                {
                    self.collect_path_reference(PathReference {
                        source: id.into_global_any(self.module),
                        path,
                    });
                }

                self.use_apparent_member_language_items();

                // skip covered member prefixes, but resume normal visiting across expressions
                self.walk_member_receiver(tree, *left);
            }
            dir::Expression::ForEach { .. } => {
                self.use_language_item(dir::LanguageItem::Iterable);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Await { .. } => {
                self.use_language_item(dir::LanguageItem::Promise);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::AwaitMaybe { .. } | dir::Expression::AwaitMust { .. } => {
                self.use_language_item(dir::LanguageItem::Promise);
                self.use_try_language_items();
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Yield {
                cardinality: dir::YieldCardinality::Generator,
                ..
            } => {
                self.use_yield_star_language_item();
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::ImportMeta => {
                self.use_language_item(dir::LanguageItem::ImportMeta);
            }
            dir::Expression::Type { .. } => {
                self.use_language_item(dir::LanguageItem::Type);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::BorrowOf { .. } => {
                self.use_language_item(dir::LanguageItem::Region);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::RangeExpression {
                start,
                end,
                end_kind,
            } => {
                self.use_range_language_item(start.is_some(), end.is_some(), *end_kind);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::ArrayExpression { .. } => {
                self.use_language_item(dir::LanguageItem::Array);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::FixedArrayExpression { .. } => {
                self.use_language_item(dir::LanguageItem::FixedArray);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Index { .. } => {
                self.use_language_item(dir::LanguageItem::Index);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Unary { operator, .. } => {
                self.use_unary_operator_language_items(*operator);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Binary { operator, .. } => {
                self.use_binary_operator_language_items(*operator);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Maybe { .. } | dir::Expression::Must { .. } => {
                self.use_try_language_items();
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Import { .. } => {}
            dir::Expression::Export {
                target: Some(_), ..
            } => {}
            _ => dir::walk_expression(self, tree, id, expression),
        }
    }

    /// Walk the receiver beneath one collected member path.
    fn walk_member_receiver(
        &mut self,
        tree: &dir::Tree,
        mut receiver: dir::LocalNodeId<dir::Expression>,
    ) {
        loop {
            let expression = tree.get(receiver);
            let dir::Expression::Member { left, .. } = expression else {
                self.stats.expressions += 1;
                self.walk_expression(tree, receiver, expression);

                return;
            };

            // contiguous prefixes are already represented by the collected path
            self.stats.expressions += 1;
            self.use_apparent_member_language_items();
            receiver = *left;
        }
    }

    /// Walk one type expression and collect references and implied language items.
    ///
    /// Example:
    /// ```tspp
    /// let value: dep.Model[];
    /// ```
    pub(in crate::resolve) fn walk_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        ty: &dir::TypeExpression,
    ) {
        match ty {
            dir::TypeExpression::Literal { value } => {
                if let Some(item) = value.representation_item() {
                    self.use_language_item(item);
                }
            }
            dir::TypeExpression::Keyword { value } => {
                if let Some(item) = value.representation_item() {
                    self.use_language_item(item);
                }
            }
            dir::TypeExpression::Reference { path, .. } => {
                self.collect_path_reference(PathReference {
                    source: id.into_global_any(self.module),
                    path: path.clone(),
                });
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::Lifetime { name } => {
                // reserved tick names spell literals and resolve to nothing
                if !matches!(self.strings.get(*name), "'static" | "'frame") {
                    self.collect_path_reference(PathReference {
                        source: id.into_global_any(self.module),
                        path: dir::Path::from_segment(*name),
                    });
                }
            }
            dir::TypeExpression::BorrowedOf { .. } => {
                self.use_language_item(dir::LanguageItem::Region);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::Array { .. } => {
                self.use_language_item(dir::LanguageItem::Array);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::Slice { .. } => {
                self.use_language_item(dir::LanguageItem::Slice);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::FixedArray { .. } => {
                self.use_language_item(dir::LanguageItem::FixedArray);
                dir::walk_type_expression(self, tree, id, ty);
            }
            _ => dir::walk_type_expression(self, tree, id, ty),
        }
    }
}
