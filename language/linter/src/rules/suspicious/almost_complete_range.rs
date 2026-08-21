use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow alphabetic ranges stopping before their conventional endpoint.
    pub ALMOST_COMPLETE_RANGE {
        id: "almost-complete-range",
        summary: "Disallow alphabetic ranges stopping before their conventional endpoint",
        explanation: r#"
An exclusive alphabetic range ending at `z` or `Z` omits that final letter.
Instead, you SHOULD make the endpoint inclusive when the full alphabet is intended.
"#,
        example: {
            reported: r#"
const lowercase = 'a'..'z';
"#,
            accepted: r#"
const lowercase = 'a'..='z';
"#,
        },
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report conventional alphabet ranges that omit their final letter.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect bounded exclusive range expressions
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::RangeExpression {
            start: Some(start),
            end: Some(end),
            end_kind: dir::RangeEnd::Open,
        } = node
        else {
            continue;
        };
        let bounds = (
            module.scalar_constant(*start)?,
            module.scalar_constant(*end)?,
        );
        if !matches!(
            bounds,
            (
                Some(dir::Literal::Character('a')),
                Some(dir::Literal::Character('z'))
            ) | (
                Some(dir::Literal::Character('A')),
                Some(dir::Literal::Character('Z'))
            )
        ) {
            continue;
        }

        // make the range endpoint inclusive
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("alphabetic range omits its final letter", span);
        if let Some(suggestion) = suggestion(module, lint, expression, *start, *end)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the corresponding inclusive range.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    start: dir::LocalNodeId<dir::Expression>,
    end: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let start_span = module.source_extent(start.into_any())?;
    let end_span = module.source_extent(end.into_any())?;
    if module.has_unretained_comment(span, &[start_span, end_span])? {
        return Ok(None);
    }

    // retain both authored bounds
    let start = module.source(start_span)?;
    let end = module.source(end_span)?;
    let patch = Patch::replace(span, format!("{start}..={end}"));
    let suggestion = lint.suggestion("include the final letter", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Include the final letter in an uppercase alphabet range.
    #[test]
    fn test_includes_uppercase_z() {
        let session = TestSession::dir(
            &ALMOST_COMPLETE_RANGE,
            r#"
const uppercase = 'A'..'Z';
"#,
        );

        session.assert_suggestions(
            r#"
const uppercase = 'A'..='Z';
"#,
        );
    }

    /// Accept other exclusive character ranges.
    #[test]
    fn test_accepts_other_character_range() {
        let session = TestSession::dir(
            &ALMOST_COMPLETE_RANGE,
            r#"
const digits = '0'..'9';
"#,
        );

        session.assert_no_diagnostics();
    }
}
