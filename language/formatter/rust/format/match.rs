use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode};
use dyst_ast::{Keyword, MatchCase, NodeId};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, MatchCase> for MatchCase {
    fn format_node(
        &self,
        node_id: NodeId<MatchCase>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

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
                write!(f, [space(), token("=>"), space(), *body])?;
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
                write!(f, [space(), token("=>"), space(), *body])?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
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
            "match value { Pattern if cond => { const X = 1 } }",
            "match value {\n\tPattern if cond => {\n\t\tconst X = 1\n\t}\n}",
            |p| p.eat_match(),
            DystFormatOptions::default_tab()
        );
    }
}
