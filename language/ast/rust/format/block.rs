use dyst_fir::format::FormatResult;

use crate::{
    Block, Break, Continue, Defer, DystFormatContext, DystFormatter, FormatNode, Keyword, Node,
    NodeId, NodeTree, NodeTreeStore, NodeType, Return,
};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

pub(crate) const CONTAINER_NODE_TYPES: [NodeType; 8] = [
    NodeType::Module,
    NodeType::Struct,
    NodeType::Enum,
    NodeType::Union,
    NodeType::Trait,
    NodeType::Implement,
    NodeType::Function,
    NodeType::Block,
];

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
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
        // label
        if let Some(label) = &self.label {
            write!(f, [label, token(": ")])?;
        }
        // statements
        if self.expressions.is_empty() {
            write!(f, [empty_block_with_infix_annotations(node_id),])?;
        }
        // single-statement block can be inline if not at start of line
        else if self.expressions.len() == 1 && !f.context().is_at_line_start(node_id) {
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

impl<'ast> FormatNode<'ast, Break> for Break {
    fn format_node(
        &self,
        node_id: NodeId<Break>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
        write!(f, [Keyword::Break])?;
        // label
        if let Some(label) = &self.label {
            write!(f, [token(": "), label])?;
        }
        // value
        if let Some(value) = &self.value {
            write!(f, [space(), value])?;
        }
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Continue> for Continue {
    fn format_node(
        &self,
        node_id: NodeId<Continue>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
        write!(f, [Keyword::Continue])?;
        // label
        if let Some(label) = &self.label {
            write!(f, [token(": "), label])?;
        }
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Return> for Return {
    fn format_node(
        &self,
        node_id: NodeId<Return>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
        write!(f, [Keyword::Return])?;
        // value
        if let Some(value) = &self.value {
            write!(f, [space(), value])?;
        }
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Defer> for Defer {
    fn format_node(
        &self,
        node_id: NodeId<Defer>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
        write!(f, [Keyword::Defer])?;
        match self {
            Defer::Expression(expression) => {
                write!(f, [space(), expression])?;
            }
            Defer::Block(block) => {
                write!(f, [space(), block])?;
            }
            Defer::Catch(match_) => {
                write!(f, [space(), Keyword::Catch, space(), match_])?;
            }
        }
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, ExpressionParserOptions, assert_format};

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
            |p| p.eat_expression(ExpressionParserOptions::default()),
            DystFormatOptions::default_tab()
        );
    }

    /// Block should break if the expression is used as a "statement".
    #[test]
    fn test_format_block_statement_like() {
        assert_format!(
            "if y { z } else { w }",
            "if y {\n\tz\n} else {\n\tw\n}",
            |p| p.eat_if(None),
            DystFormatOptions::default_tab()
        );
    }

    /// Block should break if the expression is used as a "statement" (even inside another block)
    #[test]
    fn test_format_block_statement_like_nested() {
        assert_format!(
            "{ if y { z } else { w } }",
            "{\n\tif y {\n\t\tz\n\t} else {\n\t\tw\n\t}\n}",
            |p| p.eat_block(),
            DystFormatOptions::default_tab()
        );
    }
}
