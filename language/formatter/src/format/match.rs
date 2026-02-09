use destack_fir::format::FormatResult;

use crate::{DestackFormatter, FormatNode};
use destack_ast::{Keyword, LocalNodeId, MatchCase, MatchSelector};
use destack_fir::prelude::*;
use destack_fir::write;

/// Match case rendering style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchCaseStyle {
    /// Emit `pattern => body`.
    Match,
    /// Emit `case pattern: body` and `default: body`.
    Switch,
}

/// Format a match selector according to the selected case style.
fn format_selector_with_style(
    f: &mut DestackFormatter<'_, '_>,
    selector: &MatchSelector,
    style: MatchCaseStyle,
) -> FormatResult<()> {
    match style {
        MatchCaseStyle::Match => match selector {
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
                write!(f, [token("_")])?;
            }
        },
        MatchCaseStyle::Switch => match selector {
            MatchSelector::Pattern { pattern, guard } => {
                write!(f, [Keyword::Case, space(), *pattern, token(":")])?;
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
                write!(f, [Keyword::Default, token(":")])?;
            }
        },
    }

    Ok(())
}

/// Format one match case with the selected style.
pub(crate) fn format_match_case_with_style<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    case_id: LocalNodeId<MatchCase>,
    style: MatchCaseStyle,
) -> FormatResult<()> {
    let case = f.context().tree.get(case_id);

    // case prefix
    write!(f, [f.context().any_prefix_annotations(case_id)])?;

    // selector, separator, and body
    match case {
        MatchCase::Expression { selector, body } => {
            format_selector_with_style(f, selector, style)?;
            match style {
                MatchCaseStyle::Match => {
                    write!(f, [space(), token("=>"), space(), *body])?;
                }
                MatchCaseStyle::Switch => {
                    write!(f, [space(), *body])?;
                }
            }
        }
        MatchCase::Block { selector, body } => {
            format_selector_with_style(f, selector, style)?;
            match style {
                MatchCaseStyle::Match => {
                    write!(f, [space(), token("=>"), space(), *body])?;
                }
                MatchCaseStyle::Switch => {
                    write!(f, [space(), *body])?;
                }
            }
        }
    }

    // case postfix
    write!(f, [f.context().any_infix_or_postfix_annotations(case_id)])?;

    Ok(())
}

impl<'ast> FormatNode<'ast, MatchCase> for MatchCase {
    fn format_node(
        &self,
        node_id: LocalNodeId<MatchCase>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_match_case_with_style(f, node_id, MatchCaseStyle::Match)
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
