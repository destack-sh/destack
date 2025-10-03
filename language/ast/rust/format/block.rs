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

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        node_id: NodeId<Block>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let span = f.context().get_span(node_id);
        #[cfg(debug_assertions)]
        let _span_str = f.context().get_span_str(span);

        // check whether the block is inlinable based on its contents
        // if any expression is not inline, then the entire block shouldn't be
        let is_inlinable = self
            .expressions
            .iter()
            .all(|expr_id| f.context().get_node(*expr_id).is_narrow());

        // container (default to self, mostly for testing)
        let (container_node_id, container_node_type) = f
            .context()
            .get_parent_by_id(node_id.id)
            .unwrap_or((node_id.id, NodeType::Block));

        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // label
        if let Some(label) = &self.label {
            write!(f, [label, token(": ")])?;
        }

        // empty block
        if self.expressions.is_empty() {
            write!(f, [empty_block_with_infix_annotations(node_id),])?;
        }
        // single-statement block may be inline
        // (retain existing newline if it exists)
        else if self.expressions.len() == 1
            && !f.context().is_at_line_start(node_id.id)
            && !f.context().is_at_line_start(container_node_id)
            && !f.context().has_newline(span)
            && container_node_type != NodeType::Definition
            && is_inlinable
        {
            write!(
                f,
                [group(&format_args![
                    token("{"),
                    soft_line_break_or_space(),
                    soft_block_indent(&self.expressions[0]),
                    soft_line_break_or_space(),
                    f.context().block_infix_annotations(node_id),
                    token("}"),
                ]),]
            )?;
        }
        // multi-statement block always gets newlines
        else {
            write!(
                f,
                [group(&format_args![
                    token("{"),
                    hard_line_break(),
                    soft_block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(&self.expressions)
                        .finish())),
                    hard_line_break(),
                    f.context().block_infix_annotations(node_id),
                    token("}"),
                ])]
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
