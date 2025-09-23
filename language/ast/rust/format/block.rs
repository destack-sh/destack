use dyst_language_fir::format::FormatResult;

use crate::{
    Block, Break, Continue, Defer, DystFormatContext, DystFormatter, FormatNode, Keyword, Node,
    NodeId, NodeTree, NodeTreeStore, NodeType, Return,
};
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

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
                    &f.context().infix_annotations(self.node_id)
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
        let container = f.context().get_container(node_id);
        let is_in_expression = matches!(container, Some((_, NodeType::Expression)));

        write!(f, [f.context().prefix_annotations(node_id)])?;
        // label
        if let Some(label) = &self.label {
            write!(f, [label, token(": ")])?;
        }
        // statements
        if self.statements.is_empty() {
            write!(
                f,
                [
                    empty_block_with_infix_annotations(node_id),
                    f.context().suffix_and_postfix_annotations(node_id)
                ]
            )?;
        }
        // single-statement block is inline if it's an expression and doesn't overflow
        else if self.statements.len() == 1 && is_in_expression {
            write!(
                f,
                [group(&format_args![
                    token("{"),
                    soft_line_break_or_space(),
                    soft_block_indent(&self.statements[0]),
                    soft_line_break_or_space(),
                    token("}"),
                    f.context().suffix_and_postfix_annotations(node_id),
                ])]
            )?;
        }
        // multi-statement block gets newlines always
        else {
            write!(
                f,
                [group(&format_args![
                    token("{"),
                    hard_line_break(),
                    soft_block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(&self.statements)
                        .finish())),
                    hard_line_break(),
                ])]
            )?;
            write!(f, [f.context().infix_annotations(node_id)])?;
            write!(f, [token("}")])?;
            write!(f, [f.context().suffix_and_postfix_annotations(node_id)])?;
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Break> for Break {
    fn format_node(
        &self,
        _node_id: NodeId<Break>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Break])?;
        // label
        if let Some(label) = &self.label {
            write!(f, [token(": "), label])?;
        }
        // value
        if let Some(value) = &self.value {
            write!(f, [token(" "), value])?;
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Continue> for Continue {
    fn format_node(
        &self,
        _node_id: NodeId<Continue>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Continue])?;
        // label
        if let Some(label) = &self.label {
            write!(f, [token(": "), label])?;
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Return> for Return {
    fn format_node(
        &self,
        _node_id: NodeId<Return>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Return])?;
        // value
        if let Some(value) = &self.value {
            write!(f, [token(" "), value])?;
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Defer> for Defer {
    fn format_node(
        &self,
        _node_id: NodeId<Defer>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Defer])?;
        match self {
            Defer::Expression(expression) => {
                write!(f, [token(" "), expression])?;
            }
            Defer::Block(block) => {
                write!(f, [token(" "), block])?;
            }
        }
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
    fn test_format_mixed_block_with_suffix_comment() {
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
}
