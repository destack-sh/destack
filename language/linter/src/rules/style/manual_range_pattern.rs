use std::cmp::Ordering;

use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer range patterns over enumerating consecutive values.
    pub MANUAL_RANGE_PATTERN {
        id: "manual-range-pattern",
        summary: "Prefer range patterns over enumerating consecutive values",
        explanation: r#"
A union that enumerates consecutive scalar values repeats the interval element by element.
Instead, you SHOULD write one inclusive range pattern.
"#,
        example: {
            reported: r#"
function digit(value: char): boolean {
    return match (value) {
        '0' | '1' | '2' | '3' => true
        _ => false
    };
}
"#,
            accepted: r#"
function digit(value: char): boolean {
    return match (value) {
        '0'..='3' => true
        _ => false
    };
}
"#,
        },
        provenance: [Clippy("manual_range_patterns")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report union patterns consisting of at least three consecutive scalar values.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect flat union patterns
    for (pattern, node) in view.iter_nodes::<dir::Pattern>() {
        let dir::Pattern::Union { patterns } = node else {
            continue;
        };
        let Some((first, last)) = consecutive_bounds(module, patterns)? else {
            continue;
        };

        // replace the complete enumeration with one range pattern
        let span = module.source_extent(pattern.into_any())?;
        let mut diagnostic = lint.diagnostic("union enumerates a consecutive range", span);
        if let Some(suggestion) = suggestion(module, lint, pattern, first, last)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the first and last expression in one consecutive scalar union.
fn consecutive_bounds(
    module: &DirModule<'_>,
    patterns: &[dir::LocalNodeId<dir::Pattern>],
) -> Result<
    Option<(
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    )>,
    ProviderError,
> {
    if patterns.len() < 3 {
        return Ok(None);
    }

    // collect direct scalar expression patterns
    let view = module.view();
    let mut alternatives: Vec<(dir::LocalNodeId<dir::Expression>, dir::Literal)> =
        Vec::with_capacity(patterns.len());
    for pattern in patterns {
        let dir::Pattern::Expression { value } = view.get(*pattern) else {
            return Ok(None);
        };
        let Some(literal) = module.scalar_constant(*value)? else {
            return Ok(None);
        };

        // insert each alternative into its interval order
        let mut index = alternatives.len();
        while index > 0 {
            match literal.interval_ordering(&alternatives[index - 1].1) {
                Some(Ordering::Less) => index -= 1,
                Some(_) => break,
                None => return Ok(None),
            }
        }
        alternatives.insert(index, (*value, literal));
    }

    // require every interval value exactly once
    if !alternatives
        .windows(2)
        .all(|pair| pair[0].1.successor() == Some(pair[1].1))
    {
        return Ok(None);
    }
    let first = alternatives[0].0;
    let last = alternatives[alternatives.len() - 1].0;

    Ok(Some((first, last)))
}

/// Build one inclusive range pattern from the retained endpoints.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    pattern: dir::LocalNodeId<dir::Pattern>,
    first: dir::LocalNodeId<dir::Expression>,
    last: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(pattern.into_any())?;
    let first_span = module.source_extent(first.into_any())?;
    let last_span = module.source_extent(last.into_any())?;
    if module.has_unretained_comment(span, &[first_span, last_span])? {
        return Ok(None);
    }

    // retain the first and last enumerated values
    let first = module.source(first_span)?;
    let last = module.source(last_span)?;
    let patch = Patch::replace(span, format!("{first}..={last}"));
    let suggestion = lint.suggestion("write an inclusive range pattern", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace consecutive integer alternatives.
    #[test]
    fn test_replaces_integer_union() {
        let session = TestSession::dir(
            &MANUAL_RANGE_PATTERN,
            r#"
function small(value: int32): boolean {
    return match (value) {
        1 | 2 | 3 => true
        _ => false
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
function small(value: int32): boolean {
    return match (value) {
        1..=3 => true
        _ => false
    };
}
"#,
        );
    }

    /// Replace unordered consecutive alternatives with their ordered range.
    #[test]
    fn test_replaces_unordered_union() {
        let session = TestSession::dir(
            &MANUAL_RANGE_PATTERN,
            r#"
function digit(value: char): boolean {
    return match (value) {
        '3' | '0' | '2' | '1' => true
        _ => false
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
function digit(value: char): boolean {
    return match (value) {
        '0'..='3' => true
        _ => false
    };
}
"#,
        );
    }

    /// Accept nonconsecutive alternatives.
    #[test]
    fn test_accepts_nonconsecutive_union() {
        let session = TestSession::dir(
            &MANUAL_RANGE_PATTERN,
            r#"
function sparse(value: int32): boolean {
    return match (value) {
        1 | 3 | 5 => true
        _ => false
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
