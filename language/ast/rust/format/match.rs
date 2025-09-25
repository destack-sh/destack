use dyst_fir::format::FormatResult;

use crate::{
    DystFormatter, FormatNode, Keyword, Match, MatchCase, NodeId,
    empty_block_with_infix_annotations,
};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Match> for Match {
    fn format_node(
        &self,
        node_id: NodeId<Match>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // match <expression>
        write!(f, [Keyword::Match, space(), self.value])?;

        // empty match body
        if self.cases.is_empty() {
            write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
            return Ok(());
        }

        // match cases
        write!(f, [space(), token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&format_args![block_indent(&format_with(|f| f
                .join_with(hard_line_break())
                .entries(&self.cases)
                .finish())),])]
        )?;
        write!(f, [hard_line_break(), token("}")])
    }
}

impl<'ast> FormatNode<'ast, MatchCase> for MatchCase {
    fn format_node(
        &self,
        _node_id: NodeId<MatchCase>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                // pattern and optional guard before expression body
                write!(f, [*pattern])?;
                if let Some(guard) = guard {
                    write!(f, [space(), Keyword::If, space(), *guard])?;
                }
                write!(f, [token(" => "), *body])
            }
            MatchCase::Block {
                pattern,
                body,
                guard,
            } => {
                // format pattern, guard, and block body
                write!(f, [*pattern])?;
                if let Some(guard) = guard {
                    write!(f, [space(), Keyword::If, space(), *guard])?;
                }
                write!(f, [token(" => "), *body])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_match_expression_cases() {
        assert_format!(
            "match x { 1 => 2; 3 => 4 }",
            "match x {\n\t1 => 2\n\t3 => 4\n}",
            |p| p.eat_match(),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_match_with_block_case_and_guard() {
        assert_format!(
            "match value { Pattern if cond => { let X = 1 } }",
            "match value {\n\tPattern if cond => {\n\t\tlet X = 1\n\t}\n}",
            |p| p.eat_match(),
            DystFormatOptions::default_tab()
        );
    }
}
