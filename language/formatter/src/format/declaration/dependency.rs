use crate::{DestackFormatter, FormatNode};
use destack_ast::{
    DependencyItem, DependencyKind, DependencyMode, Expression, ImportSource, ImportTarget,
    Keyword, LocalNodeId, Name, NodeTree, ScalarLiteral,
};
use destack_base::ImmutableStringPool;
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;
use destack_query::common::{
    ImportDeclarationKey, categorize_import, sort_dependency_items as sort_items,
    sort_import_declaration_indices,
};
use destack_workspace::ImportSortOrder;

use crate::format::collection::literal::format_scalar_literal;
use destack_source::Span;

/// Format a dependency item name.
fn format_dependency_item_name<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    name: Name,
) -> FormatResult<()> {
    match name {
        Name::Identifier(name) | Name::Number(name) => {
            write!(f, [name])?;
        }
        Name::String(name) => {
            let literal = ScalarLiteral::String(name);
            let span = Span::empty(f.context().file.id);
            format_scalar_literal(&literal, span, f)?;
        }
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, DependencyItem> for DependencyItem {
    fn format_node(
        &self,
        node_id: LocalNodeId<DependencyItem>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // type
        if self.kind == Some(DependencyKind::Type) {
            write!(f, [Keyword::Type, space()])?;
        }

        let is_default_binding = self.mode == DependencyMode::Default
            || (self.mode == DependencyMode::Item && self.name.is_none());

        // default
        if is_default_binding {
            write!(f, [Keyword::Default])?;
            // alias
            if let Some(alias) = self.alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
        // namespace
        else if self.mode == DependencyMode::Namespace {
            write!(f, [token("*")])?;
            if let Some(alias) = self.alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
        // item
        else {
            // name
            if let Some(name) = self.name {
                format_dependency_item_name(f, name)?;
            }
            // alias
            if let Some(alias) = self.alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

/// Return the inner import expression, unwrapping statement wrappers when needed.
pub fn import_expression(expr_id: LocalNodeId<Expression>, tree: &NodeTree) -> Option<&Expression> {
    let expr = tree.get(expr_id);
    match expr {
        Expression::Import {
            source: ImportSource::ImportCall,
            ..
        } => None,
        Expression::Import { target, .. } => {
            if matches!(target, ImportTarget::String(_)) {
                Some(expr)
            } else {
                None
            }
        }
        Expression::Statement(inner_id) => {
            let inner = tree.get(*inner_id);
            if matches!(
                inner,
                Expression::Import {
                    source: ImportSource::ImportStatement | ImportSource::ImportEquals,
                    target: ImportTarget::String(_),
                    ..
                }
            ) {
                Some(inner)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if an expression is an import (unwrapping Statement if needed).
pub fn is_import(expr_id: LocalNodeId<Expression>, tree: &NodeTree) -> bool {
    import_expression(expr_id, tree).is_some()
}

/// Sort import expressions by group and then alphabetically within each group.
///
/// Side-effect imports (no items) preserve their relative order and stay at the top.
pub fn sort_imports(
    imports: &[LocalNodeId<Expression>],
    tree: &NodeTree,
    strings: &ImmutableStringPool,
) -> Vec<LocalNodeId<Expression>> {
    let mut expression_ids = Vec::new();
    let mut order_keys = Vec::new();

    // collect sortable declaration keys
    for &expr_id in imports {
        if let Some(Expression::Import { items, target, .. }) = import_expression(expr_id, tree) {
            let ImportTarget::String(target) = target else {
                continue;
            };

            let target_str = strings.get(*target);
            expression_ids.push(expr_id);
            order_keys.push(ImportDeclarationKey {
                target: target_str,
                is_side_effect: items.is_empty(),
            });
        }
    }

    // map declaration order back to expression ids
    let order = sort_import_declaration_indices(&order_keys);
    order
        .into_iter()
        .map(|index| expression_ids[index])
        .collect()
}

/// Sort dependency items by kind and configured key order.
pub fn sort_dependency_items(
    items: &[LocalNodeId<DependencyItem>],
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    sort_order: ImportSortOrder,
) -> Vec<LocalNodeId<DependencyItem>> {
    sort_items(items, tree, strings, sort_order)
}

/// Determine if a blank line should be inserted between two imports.
///
/// Returns true if:
/// - Transitioning from side-effect to regular imports
/// - Different import groups (for regular imports)
pub fn should_insert_blank_between(
    prev_expr_id: LocalNodeId<Expression>,
    curr_expr_id: LocalNodeId<Expression>,
    tree: &NodeTree,
    strings: &ImmutableStringPool,
) -> bool {
    let (prev_is_side_effect, prev_group) = match import_expression(prev_expr_id, tree) {
        Some(Expression::Import { items, target, .. }) => {
            let ImportTarget::String(target) = target else {
                return false;
            };
            let target_str = strings.get(*target);
            (items.is_empty(), categorize_import(target_str))
        }
        _ => return false,
    };

    let (curr_is_side_effect, curr_group) = match import_expression(curr_expr_id, tree) {
        Some(Expression::Import { items, target, .. }) => {
            let ImportTarget::String(target) = target else {
                return false;
            };
            let target_str = strings.get(*target);
            (items.is_empty(), categorize_import(target_str))
        }
        _ => return false,
    };

    // blank line between side effect and regular imports
    if prev_is_side_effect && !curr_is_side_effect {
        return true;
    }

    // blank line between different groups (for regular imports)
    if !prev_is_side_effect && !curr_is_side_effect && prev_group != curr_group {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use crate::{
        DestackFormatOptions, TestFormatter, assert_format, assert_format_roundtrip_with_file_type,
    };
    use destack_source::FileType;

    #[test]
    fn test_format_import() {
        assert_format!(
            "import \"foo\"",
            "import \"foo\"",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_alias() {
        assert_format!(
            "import * as foo from \"foo\"",
            "import * as foo from \"foo\"",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_items_from() {
        assert_format!(
            "import {bar, baz} from \"foo\"",
            "import { bar, baz } from \"foo\"",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default_with_line_width(60)
        );
    }

    #[test]
    fn test_format_import_with_overflow() {
        let source = r#"import {
    StructuredObject,
    StructuredObjectOptions,
    StructuredObjectOptions2,
    StructuredObjectOptions3,
} from "lib""#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default_with_line_width(60)
        );
    }

    #[test]
    fn test_format_export_glob() {
        assert_format!(
            r#"export * from "./foo""#,
            r#"export * from "./foo""#,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_default_and_block() {
        assert_format!(
            "import Default, { type Item } from \"foo\"",
            "import Default, { type Item } from \"foo\"",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_type_equals_require() {
        assert_format!(
            r#"import type React = require("react")"#,
            r#"import type React = require("react");"#,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_export_import_type_equals_require() {
        assert_format!(
            r#"export import type React = require("react")"#,
            r#"export import type React = require("react");"#,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_export_with_default_and_block() {
        assert_format!(
            "export { default, default as bar, foo } from \"foo\"",
            "export { default, default as bar, foo } from \"foo\"",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_export_with_attributes() {
        assert_format!(
            "export { foo } from \"bar\" with { mode: \"strict\" }",
            "export { foo } from \"bar\" with { mode: \"strict\" }",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_default_and_namespace_roundtrip() {
        assert_format_roundtrip_with_file_type(
            r#"import a, * as b from "a""#,
            r#"import a, * as b from "a""#,
            FileType::JavaScript,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default(),
        );
    }

    #[test]
    fn test_format_export_default_and_namespace_roundtrip() {
        assert_format_roundtrip_with_file_type(
            r#"export a, * as b from "mod""#,
            r#"export a, * as b from "mod""#,
            FileType::JavaScript,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default(),
        );
    }

    #[test]
    fn test_format_import_type_empty_items_roundtrip() {
        assert_format_roundtrip_with_file_type(
            r#"import type {} from "a""#,
            r#"import type {} from "a""#,
            FileType::TypeScript,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default(),
        );
    }

    #[test]
    fn test_format_export_empty_items_with_target_roundtrip() {
        assert_format_roundtrip_with_file_type(
            r#"export {} from "a""#,
            r#"export {} from "a""#,
            FileType::JavaScript,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default(),
        );
    }
}
