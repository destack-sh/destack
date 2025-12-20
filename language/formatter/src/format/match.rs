use destack_fir::format::FormatResult;

use crate::{DestackFormatter, FormatNode};
use destack_ast::{Keyword, LocalNodeId, MatchCase, MatchSelector};
use destack_fir::prelude::*;
use destack_fir::write;

/// Format the selector (pattern + guard or default).
fn format_selector(f: &mut DestackFormatter<'_, '_>, selector: &MatchSelector) -> FormatResult<()> {
    match selector {
        MatchSelector::Pattern { pattern, guard } => {
            write!(f, [*pattern])?;
            if let Some(guard) = guard {
                write!(
                    f,
                    [
                        space(),
                        Keyword::If,
                        space(),
                        token("("),
                        *guard,
                        token(")")
                    ]
                )?;
            }
        }
        MatchSelector::Default => {
            // For match-style formatting, output underscore for default
            write!(f, [token("_")])?;
        }
    }
    Ok(())
}

impl<'ast> FormatNode<'ast, MatchCase> for MatchCase {
    fn format_node(
        &self,
        node_id: LocalNodeId<MatchCase>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            MatchCase::Expression { selector, body } => {
                format_selector(f, selector)?;
                write!(f, [space(), token("=>"), space(), *body])?;
            }
            MatchCase::Block { selector, body } => {
                format_selector(f, selector)?;
                write!(f, [space(), token("=>"), space(), *body])?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_match_expression_cases() {
        assert_format!(
            "match (x) { 1 => 2; 3 => 4 }",
            "match (x) {\n\t1 => 2\n\t3 => 4\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_match_with_block_case_and_guard() {
        assert_format!(
            "match (value) { Pattern if (cond) => { const X = 1; } }",
            "match (value) {\n\tPattern if (cond) => {\n\t\tconst X = 1;\n\t}\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_switch_expression_cases() {
        assert_format!(
            "switch (x) { case 1: 2; case 3: 4 }",
            "switch (x) {\n\tcase 1: 2\n\tcase 3: 4\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_switch_with_default_case() {
        assert_format!(
            "switch (x) { case 1: \"one\"; default: \"other\" }",
            "switch (x) {\n\tcase 1: \"one\"\n\tdefault: \"other\"\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_switch_with_block() {
        assert_format!(
            "switch (value) { case 1: { const x = 1; } }",
            "switch (value) {\n\tcase 1: {\n\t\tconst x = 1;\n\t}\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }
}
