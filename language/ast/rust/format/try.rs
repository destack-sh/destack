use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Keyword, NodeId, Try, empty_block_with_infix_annotations};
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Try> for Try {
    fn format_node(
        &self,
        node_id: NodeId<Try>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Try::Expression { try_expression } => {
                // try <expression>
                write!(f, [Keyword::Try, space(), *try_expression])
            }
            Try::Block { try_block } => {
                // try { <block> }
                write!(f, [Keyword::Try, space(), *try_block])
            }
            Try::BlockWithCatch {
                try_block,
                catch_match,
            } => {
                // try { <block> }
                write!(f, [Keyword::Try, space(), *try_block])?;

                // catch match
                let catch = f.context().get_node(*catch_match).clone();

                // catch <value>
                write!(f, [space(), Keyword::Catch, space(), catch.value])?;

                // empty catch body
                if catch.cases.is_empty() {
                    write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
                    return Ok(());
                }

                // catch cases
                write!(f, [space(), token("{"), hard_line_break()])?;
                write!(
                    f,
                    [group(&format_args![block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(&catch.cases)
                        .finish())),])]
                )?;
                write!(f, [hard_line_break(), token("}")])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_try_expression() {
        assert_format!(
            "try operation()",
            "try operation()",
            |p| p.eat_try_catch(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_try_block() {
        assert_format!(
            "try { let X = 1 }",
            "try {\n\tlet X = 1\n}",
            |p| p.eat_try_catch(),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_try_block_with_catch() {
        assert_format!(
            "try { let X = risky() } catch err { Error(err) => err }",
            "try {\n\tlet X = risky()\n} catch err {\n\tError(err) => err\n}",
            |p| p.eat_try_catch(),
            DystFormatOptions::default_tab()
        );
    }
}
