use destack_ast::{
    Block, Expression, LocalNodeId, LocalNodeIdAny, Node, NodeTree, NodeTreeImpl, NodeType,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

use super::imports;
use crate::expression::format_expression;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};

/// Empty block with infix annotations.
#[derive(Debug, Clone, PartialEq)]
pub struct EmptyBlockWithInfixAnnotations<T: Node> {
    node_id: LocalNodeId<T>,
}

impl<'ast, T> Format<DestackFormatContext<'ast>> for EmptyBlockWithInfixAnnotations<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        write!(
            f,
            [group(&format_args![
                token("{"),
                soft_block_indent(&format_args![
                    if_group_fits_on_line(&space()),
                    &f.context().block_infix_annotations(self.node_id)
                ]),
                token("}")
            ])]
        )
    }
}

/// Format an empty block with infix annotations.
///
/// Example:
/// ```
/// {
///     // infix comment
/// }
/// ```
pub fn empty_block_with_infix_annotations<T: Node>(
    node_id: LocalNodeId<T>,
) -> EmptyBlockWithInfixAnnotations<T> {
    EmptyBlockWithInfixAnnotations { node_id }
}

/// Format a block inline with zero or one expression (including label and infix annotations).
#[inline]
pub(crate) fn format_block_body_narrow<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    debug_assert!(block.expressions.len() <= 1);

    // body
    if block.expressions.is_empty() {
        write!(f, [token("{"), space(), token("}")])?;
    } else {
        write!(
            f,
            [
                token("{"),
                soft_line_break_or_space(),
                soft_block_indent(&format_args![
                    &block.expressions[0],
                    f.context().block_infix_annotations(block_id)
                ]),
                soft_line_break_or_space(),
                token("}")
            ]
        )?;
    }
    Ok(())
}

/// Format a block multiline with multiple expressions (including label and infix annotations).
#[inline]
pub(crate) fn format_block_body_wide<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    // body
    write!(
        f,
        [
            token("{"),
            hard_line_break(),
            soft_block_indent(&format_with(|f| format_block_of_statements(
                f,
                block_id.into_any(),
                &block.expressions
            ))),
            hard_line_break(),
            block_indent(&f.context().block_infix_annotations(block_id)),
            token("}"),
        ]
    )
}

/// Format a block of expression statements (with appropriate empty annotations).
pub(crate) fn format_block_of_statements<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _scope_id: LocalNodeIdAny,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    let organize = f.context().options.organize_imports.is_enabled();
    let tree = f.context().tree;
    let strings = f.context().strings;

    // find contiguous import section at the start
    let import_count = expressions
        .iter()
        .take_while(|&&expr_id| imports::is_import(expr_id, tree))
        .count();

    // prepare the expression list (potentially with sorted imports)
    let sorted_imports: Vec<LocalNodeId<Expression>>;
    let effective_expressions: Vec<LocalNodeId<Expression>> = if organize && import_count > 1 {
        sorted_imports = imports::sort_imports(&expressions[..import_count], tree, strings);
        sorted_imports
            .iter()
            .copied()
            .chain(expressions[import_count..].iter().copied())
            .collect()
    } else {
        expressions.to_vec()
    };

    let mut prev_was_import = false;
    let mut prev_import_id: Option<LocalNodeId<Expression>> = None;

    for (i, &expression_id) in effective_expressions.iter().enumerate() {
        let expression = f.context().tree.get(expression_id);
        let is_import_expr = imports::is_import(expression_id, tree);

        // blank line between expressions
        if i > 0 {
            write!(f, [hard_line_break()])?;

            // determine if we need an extra blank line
            let needs_blank = if organize && prev_was_import && is_import_expr {
                // check if different import groups
                prev_import_id.is_some_and(|prev_id| {
                    imports::should_insert_blank_between(
                        prev_id,
                        expression_id,
                        f.context().tree,
                        f.context().strings,
                    )
                })
            } else if prev_was_import && !is_import_expr {
                // blank line after import section (if not already present)
                !f.context()
                    .has_blank_prefix_annotation_in_first_position(expression_id)
            } else if matches!(expression, Expression::Declaration(_)) {
                // extra blank line between declarations
                !f.context()
                    .has_blank_prefix_annotation_in_first_position(expression_id)
            } else {
                false
            };

            if needs_blank {
                write!(f, [empty_line()])?;
            }
        }

        // expression itself (with prefix annotations)
        write!(f, [f.context().any_prefix_annotations(expression_id)])?;
        format_expression(f, expression_id, expression)?;

        // add semicolon for bare Import if not last expression (when sorting moved it)
        let is_last = i == effective_expressions.len() - 1;
        let is_bare_import = matches!(expression, Expression::Import { .. });
        if !is_last && is_bare_import {
            write!(f, [token(";")])?;
        }

        // postfix annotations
        write!(
            f,
            [f.context().any_infix_or_postfix_annotations(expression_id)]
        )?;

        prev_was_import = is_import_expr;
        if is_import_expr {
            prev_import_id = Some(expression_id);
        }
    }
    Ok(())
}

#[inline]
pub(crate) fn should_inline_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = f.context().tree.get(block_id);
    let span = f.context().get_span(block_id);

    // can only inline if there is at most one expression
    //  (on the flipside, always inline if there is nothing in it)
    if block.expressions.len() > 1 || f.context().has_infix_annotation(block_id) {
        return false;
    } else if block.expressions.is_empty() {
        return true;
    }

    // check whether the block is inlinable based on its contents
    // if any expression is not inline, then the entire block shouldn't be
    let is_body_inlinable = block.expressions.is_empty()
        || block
            .expressions
            .iter()
            .all(|expr_id| f.context().get_node(*expr_id).is_narrow());

    // container (default to self, mostly for testing)
    let (mut container_node_id, mut container_node_type) = f
        .context()
        .get_parent_by_id(block_id.id)
        .unwrap_or((block_id.id, NodeType::Block));
    if container_node_type == NodeType::Expression {
        (container_node_id, container_node_type) = f
            .context()
            .get_parent_by_id(container_node_id)
            .unwrap_or((container_node_id, NodeType::Block));
    }

    is_body_inlinable
        && !f.context().is_at_line_start(block_id.id)
        && !f.context().is_at_line_start(container_node_id)
        && !f.context().has_newline(span)
        && container_node_type != NodeType::Declaration
}

/// Format a block (without a nested group!).
pub fn format_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    write!(f, [f.context().any_prefix_annotations(node_id)])?;
    if should_inline_block(f, node_id) {
        format_block_body_narrow(f, node_id)?;
    } else {
        format_block_body_wide(f, node_id)?;
    }
    write!(f, [f.context().any_postfix_annotations(node_id)])?;
    Ok(())
}

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        node_id: LocalNodeId<Block>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
        if should_inline_block(f, node_id) {
            write!(
                f,
                [group(&format_with(|f| format_block_body_narrow(
                    f, node_id
                )))]
            )?;
        } else {
            write!(
                f,
                [group(&format_with(|f| format_block_body_wide(f, node_id)))]
            )?;
        }
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    /// Semicolons should be automatically inserted for every value-ignored expression.
    /// Control flow forms like if and let only get semicolons if used as statements.
    #[test]
    fn test_format_block_insert_semicolon() {
        assert_format!(
            r#"{
    import "foo"
    import * as baz from "foo"

    let x = 1;
    let y = 2
    y

    if (x) {
        y
    } else {
        print("foo")
        z(x) 
    }

    loop {
       break;
    }

    let x = z()
    let x = if (let y = 1) {
        z()
    } else {
        w()
    };

    return 5;
}"#,
            r#"{
    import "foo";
    import * as baz from "foo";

    let x = 1;
    let y = 2;
    y;

    if (x) {
        y
    } else {
        print("foo");
        z(x)
    }

    loop {
        break;
    }

    let x = z();
    let x = if (let y = 1) {
        z()
    } else {
        w()
    };

    return 5;
}"#,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_empty_block_with_comment() {
        let source = "{
    // infix comment
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_mixed_block_with_prefix_postfix_comment() {
        let source = "{
    // prefix comment
    const X = 1; // suffix comment
    // postfix comment
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_mixed_block_with_postfix_comment() {
        let source = "{
    const X = 1; // this is my X
    const Y = 2; // this is my Y
    const Z = 3; // this is my Z
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_mixed_block_with_postfix_annotations_mixed() {
        let source = "{
    const X = 1; // this is my X
    const Y = 2; // this is my Y
    const Z = 3; // this is my Z
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Block shouldn't break if the expression is used inline.
    #[test]
    fn test_format_block_inline() {
        let source = "const x = if (y) { z } else { w }";
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_block_statement_like() {
        assert_format!(
            "if (y) { z } else { w; }",
            "if (y) {\n\tz\n} else {\n\tw;\n}",
            |p| p.eat_if(),
            DestackFormatOptions::default_tab()
        );
    }

    /// Block should retain the explicit newline.
    #[test]
    fn test_format_block_statement_retain_newline() {
        let source = "{\n\tconst X = 1;\n\tconst Y = 2;\n\tconst Z = 3;\n}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default_tab()
        );
    }

    /// Import statements should be sorted when organize_imports is enabled.
    #[test]
    fn test_format_block_import_sorting() {
        use destack_workspace::OrganizeImports;
        let options = DestackFormatOptions {
            organize_imports: OrganizeImports::On,
            ..DestackFormatOptions::default()
        };
        assert_format!(
            r#"{
    import lodash from "lodash"
    import fs from "node:fs"
    import path from "node:path"
}"#,
            r#"{
    import fs from "node:fs";
    import path from "node:path";

    import lodash from "lodash";
}"#,
            |p| p.eat_block(),
            options
        );
    }
}
