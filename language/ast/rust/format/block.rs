use dyst_fir::format::FormatResult;

use crate::{
    Block, DystFormatContext, DystFormatter, FormatNode, Node, NodeId, NodeTree, NodeTreeStore,
    NodeType,
};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

/// Empty block with infix annotations.
#[derive(Debug, Clone, PartialEq)]
pub struct EmptyBlockWithInfixAnnotations<T: Node> {
    node_id: NodeId<T>,
}

impl<'ast, T> Format<DystFormatContext<'ast>> for EmptyBlockWithInfixAnnotations<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeStore<T>,
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
    node_id: NodeId<T>,
) -> EmptyBlockWithInfixAnnotations<T> {
    EmptyBlockWithInfixAnnotations { node_id }
}

/// Format a block inline with zero or one expression (including label and infix annotations).
#[inline]
pub(crate) fn format_block_body_narrow<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    block_id: NodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    debug_assert!(block.expressions.len() <= 1);

    // label
    if let Some(label) = &block.label {
        write!(f, [label, token(": ")])?;
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
    block_id: NodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);

    // label
    if let Some(label) = &block.label {
        write!(f, [label, token(": ")])?;
    }

    // body
    write!(
        f,
        [
            token("{"),
            hard_line_break(),
            soft_block_indent(&format_with(|f| f
                .join_with(hard_line_break())
                .entries(&block.expressions)
                .finish())),
            hard_line_break(),
            block_indent(&f.context().block_infix_annotations(block_id)),
            token("}"),
        ]
    )
}

#[inline]
pub(crate) fn should_inline_block<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    block_id: NodeId<Block>,
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
    let (container_node_id, container_node_type) = f
        .context()
        .get_parent_by_id(block_id.id)
        .unwrap_or((block_id.id, NodeType::Block));

    is_body_inlinable
        && !f.context().is_at_line_start(block_id.id)
        && !f.context().is_at_line_start(container_node_id)
        && !f.context().has_newline(span)
        && container_node_type != NodeType::Definition
}

/// Format a block (without a nested group!).
pub fn format_block<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    node_id: NodeId<Block>,
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
        node_id: NodeId<Block>,
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
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

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
    let X = 1 // suffix comment
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
    let X = 1 // this is my X
    let Y = 2 // this is my Y
    let Z = 3 // this is my Z
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
    let X = 1 #x // this is my X
    let Y = 2 #y // this is my Y
    let Z = 3 #z // this is my Z
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_block_with_nested_declaration() {
        let source = r"{
    // x comment
    let x =
        // y comment
        function y(v: float32) => float32 {
            // z comment
        }
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
        let source = "let x = if y { z } else { w }";
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
            "if y { z } else { w }",
            "if y {\n\tz\n} else {\n\tw\n}",
            |p| p.eat_if(None),
            DystFormatOptions::default_tab()
        );
    }

    /// Block should retain the explicit newline.
    #[test]
    fn test_format_block_statement_retain_newline() {
        let source = "{\n\tlet X = 1\n\tlet Y = 2\n\tlet Z = 3\n}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DystFormatOptions::default_tab()
        );
    }
}
