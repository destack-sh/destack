use destack_dir as dir;
use smallvec::SmallVec;

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
                let source = id.into_any();
                let key = dir::StaticKey::Name(*name);

                self.collect_global_reference(source, key, dir::SymbolSpace::Value);
            }
            dir::Expression::QualifiedReference { path, .. } if path.segments.len() == 1 => {
                let source = id.into_any();
                let key = dir::StaticKey::Name(path.segments[0]);

                self.collect_global_reference(source, key, dir::SymbolSpace::Value);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::QualifiedReference { path, .. } => {
                self.collect_path_reference(PathReference {
                    source: id.into_global_any(self.module),
                    path: path.clone(),
                });
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Member { .. } => {
                if self.member_path_collection_depth == 0
                    && let Some(path) = self.member_path_reference(tree, id)
                {
                    self.collect_path_reference(PathReference {
                        source: id.into_global_any(self.module),
                        path,
                    });
                }

                self.member_path_collection_depth += 1;
                dir::walk_expression(self, tree, id, expression);
                self.member_path_collection_depth -= 1;
            }
            dir::Expression::ForEach {
                operator: dir::ForEachOperator::Of,
                ..
            } => {
                self.require_language_item(dir::LanguageItem::Iterable);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::Await { .. } => {
                self.require_language_item(dir::LanguageItem::Promise);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::AwaitMaybe { .. } | dir::Expression::AwaitMust { .. } => {
                self.require_language_item(dir::LanguageItem::Promise);
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
                self.require_language_item(dir::LanguageItem::ImportMeta);
            }
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::RegexString { .. }) => {
                self.require_language_item(dir::LanguageItem::RegExp);
            }
            dir::Expression::Type { .. } => {
                self.require_language_item(dir::LanguageItem::Type);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::BorrowOf { .. } => {
                self.require_language_item(dir::LanguageItem::Lifetime);
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
            dir::Expression::ArrayExpression { .. } => {
                self.require_language_item(dir::LanguageItem::Array);
                dir::walk_expression(self, tree, id, expression);
            }
            dir::Expression::FixedArrayExpression { .. } => {
                self.require_language_item(dir::LanguageItem::FixedArray);
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
            dir::TypeExpression::Reference { path, .. } if path.segments.len() == 1 => {
                let source = id.into_any();
                let key = dir::StaticKey::Name(path.segments[0]);

                self.collect_global_reference(source, key, dir::SymbolSpace::Type);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::Reference { path, .. } => {
                self.collect_path_reference(PathReference {
                    source: id.into_global_any(self.module),
                    path: path.clone(),
                });
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::BorrowedOf { .. } => {
                self.require_language_item(dir::LanguageItem::Lifetime);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::Array { .. } => {
                self.require_language_item(dir::LanguageItem::Array);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::Slice { .. } => {
                self.require_language_item(dir::LanguageItem::Slice);
                dir::walk_type_expression(self, tree, id, ty);
            }
            dir::TypeExpression::FixedArray { .. } => {
                self.require_language_item(dir::LanguageItem::FixedArray);
                dir::walk_type_expression(self, tree, id, ty);
            }
            _ => dir::walk_type_expression(self, tree, id, ty),
        }
    }

    /// Return one member chain as a namespace path candidate.
    ///
    /// Example:
    /// ```ds
    /// dep.api.value
    /// ```
    fn member_path_reference(
        &self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::Path> {
        let mut segments = SmallVec::new();
        collect_member_path_segments(tree, id, &mut segments)?;

        (segments.len() > 1).then_some(dir::Path { segments })
    }
}

/// Collect source path segments from one expression member chain.
///
/// Example:
/// ```ds
/// dep.api.value
/// ```
fn collect_member_path_segments(
    tree: &dir::Tree,
    id: dir::LocalNodeId<dir::Expression>,
    segments: &mut SmallVec<[dir::StringId; 1]>,
) -> Option<()> {
    match tree.get(id) {
        // collect the path root
        dir::Expression::Identifier { name } => {
            segments.push(*name);
        }

        // collect an already path-shaped root
        dir::Expression::QualifiedReference { path, .. } => {
            segments.extend(path.segments.iter().copied());
        }

        // extend through one member segment
        dir::Expression::Member {
            left,
            name: Some(name),
        } => {
            collect_member_path_segments(tree, *left, segments)?;
            segments.push(*name);
        }

        // reject dynamic member syntax
        _ => return None,
    }

    Some(())
}
