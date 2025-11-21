use dyst_fir::format::FormatResult;

use crate::expression::format_expression;
use crate::{DystFormatContext, DystFormatter, FormatNode};
use dyst_ast::{
    Block, Expression, LocalNodeId, LocalNodeIdAny, Node, NodeTree, NodeTreeImpl, NodeType,
};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

/// Empty block with infix annotations.
#[derive(Debug, Clone, PartialEq)]
pub struct EmptyBlockWithInfixAnnotations<T: Node> {
    node_id: LocalNodeId<T>,
}

impl<'ast, T> Format<DystFormatContext<'ast>> for EmptyBlockWithInfixAnnotations<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
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
    f: &mut DystFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    debug_assert!(block.expressions.len() <= 1);

    // label
    if let Some(label) = &block.label {
        write!(f, [label, token(":"), space()])?;
    }

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
    f: &mut DystFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    // label
    if let Some(label) = &block.label {
        write!(f, [label, token(":"), space()])?;
    }
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
/// Automatically inserts semicolons for value-ignored non-statement expressions.
pub(crate) fn format_block_of_statements<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    scope_id: LocalNodeIdAny,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    let is_root_like = f
        .context()
        .get_parent_by_id(scope_id.id)
        .is_none_or(|(_, parent_type)| parent_type == NodeType::Declaration);
    for (i, &expression_id) in expressions.iter().enumerate() {
        let expression = f.context().tree.get(expression_id);

        // blank line between expressions
        if i > 0 {
            write!(f, [hard_line_break()])?;
            // extra blank line between declarations
            if matches!(expression, Expression::Declaration(_))
                && !f
                    .context()
                    .has_blank_prefix_annotation_in_first_position(expression_id)
            {
                write!(f, [empty_line()])?;
            }
        }

        // expression itself (with prefix annotations)
        write!(f, [f.context().any_prefix_annotations(expression_id)])?;
        format_expression(f, expression_id, expression)?;

        // insert semicolon for value-ignored non-statement expressions
        if !expression.is_statement_like() && (i < expressions.len() - 1 || is_root_like)
            || expression.is_statement_like_at_root()
        {
            write!(f, [token(";")])?;
        }

        // postfix annotations
        write!(
            f,
            [f.context().any_infix_or_postfix_annotations(expression_id)]
        )?;
    }
    Ok(())
}

#[inline]
pub(crate) fn should_inline_block<'ast>(
    f: &mut DystFormatter<'ast, '_>,
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
    f: &mut DystFormatter<'ast, '_>,
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
        f: &mut DystFormatter<'ast, '_>,
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
    use crate::{DystFormatOptions, TestFormatter, assert_format};

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

    if x {
        y
    } else {
        print("foo")
        z(x) 
    }

    loop {
       break;
    }

    let x = z()
    let x = if let y = 1 {
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

    if x {
        y
    } else {
        print("foo");
        z(x)
    }

    loop {
        break;
    }

    let x = z();
    let x = if let y = 1 {
        z()
    } else {
        w()
    };

    return 5;
}"#,
            |p| p.eat_block(),
            DystFormatOptions::default()
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
            DystFormatOptions::default()
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
            DystFormatOptions::default()
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
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_mixed_block_with_postfix_annotations_mixed() {
        let source = "{
    const X = 1; #x // this is my X
    const Y = 2; #y // this is my Y
    const Z = 3; #z // this is my Z
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DystFormatOptions::default()
        );
    }

    /// Block shouldn't break if the expression is used inline.
    #[test]
    fn test_format_block_inline() {
        let source = "const x = if y { z } else { w }";
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_block_statement_like() {
        assert_format!(
            "if y { z } else { w; }",
            "if y {\n\tz\n} else {\n\tw;\n}",
            |p| p.eat_if(),
            DystFormatOptions::default_tab()
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
            DystFormatOptions::default_tab()
        );
    }
}
