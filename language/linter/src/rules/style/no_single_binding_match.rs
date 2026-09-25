use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer direct evaluation over a match with one irrefutable arm.
    pub NO_SINGLE_BINDING_MATCH {
        id: "no-single-binding-match",
        summary: "Prefer direct evaluation over a match with one irrefutable arm",
        explanation: r#"
A match with one unguarded binding arm accepts every value and performs no selection.
Instead, you SHOULD bind the value directly in a `do` expression.
"#,
        example: {
            reported: r#"
function double(value: int32): int32 {
    return match (value) {
        matched => matched * 2
    };
}
"#,
            accepted: r#"
function double(value: int32): int32 {
    return do {
        const matched = value;
        matched * 2
    };
}
"#,
        },
        provenance: [Clippy("match_single_binding")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report one arm matches that accept their scrutinee unconditionally.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect matches with exactly one unguarded arm
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Match { value, arms } = node else {
            continue;
        };
        let [arm] = arms.as_slice() else {
            continue;
        };
        let arm_node = view.get(*arm);
        if arm_node.guard().is_some() {
            continue;
        }
        let pattern = arm_node.pattern();
        let is_wildcard = matches!(
            module.pattern_decision(pattern)?,
            dir::PatternDecision::Ignore
        );
        let has_binding = module
            .symbols_declared_within(pattern.into_any())
            .next()
            .is_some();
        let is_irrefutable = module
            .match_coverage(expression)
            .is_some_and(|coverage| coverage.is_exhaustive);
        if (!is_wildcard && !has_binding) || !is_irrefutable {
            continue;
        }

        // replace expression arms while retaining explicit block arms for review
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("single match arm accepts every value", span);
        if let dir::MatchArm::Expression { body, .. } = arm_node
            && let Some(suggestion) = suggestion(module, lint, span, *value, pattern, *body)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one scoped binding expression.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    value: dir::LocalNodeId<dir::Expression>,
    pattern: dir::LocalNodeId<dir::Pattern>,
    body: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let value_extent = module.source_extent(value.into_any())?;
    let pattern_extent = module.source_extent(pattern.into_any())?;
    let body_extent = module.source_extent(body.into_any())?;
    if module.has_unretained_comment(extent, &[value_extent, pattern_extent, body_extent])? {
        return Ok(None);
    }

    // require a single line body for a canonical replacement
    let body = module.source(body_extent)?;
    if body.contains('\n') {
        return Ok(None);
    }

    // preserve scrutinee evaluation through a binding or expression statement
    let is_object = matches!(
        module.view().get(value),
        dir::Expression::ObjectExpression { .. }
    );
    let value = module.source(value_extent)?;
    let binding = match module.pattern_decision(pattern)? {
        dir::PatternDecision::Ignore if is_object => format!("({value});"),
        dir::PatternDecision::Ignore => format!("{value};"),
        _ => {
            let pattern = module.source(pattern_extent)?;

            format!("const {pattern} = {value};")
        }
    };

    // indent the block relative to its containing source line
    let closing_indentation = module.source_indentation(extent)?;
    let statement_indentation = format!("{closing_indentation}    ");
    let replacement = format!(
        "do {{\n{statement_indentation}{binding}\n{statement_indentation}{body}\n{closing_indentation}}}"
    );
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("bind the value directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Retain evaluation when replacing a wildcard arm.
    #[test]
    fn test_replaces_wildcard_arm() {
        let session = TestSession::dir(
            &NO_SINGLE_BINDING_MATCH,
            r#"
declare function observe(): int32;

function answer(): int32 {
    return match (observe()) {
        _ => 42
    };
}
"#,
        );

        session.assert_fixes(
            r#"
declare function observe(): int32;

function answer(): int32 {
    return do {
        observe();
        42
    };
}
"#,
        );
    }

    /// Parenthesize an object scrutinee retained as a statement.
    #[test]
    fn test_parenthesizes_wildcard_object() {
        let session = TestSession::dir(
            &NO_SINGLE_BINDING_MATCH,
            r#"
function answer(): int32 {
    return match ({ value: 1 }) {
        _ => 42
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function answer(): int32 {
    return do {
        ({ value: 1 });
        42
    };
}
"#,
        );
    }

    /// Replace an irrefutable tuple destructuring arm.
    #[test]
    fn test_replaces_tuple_binding() {
        let session = TestSession::dir(
            &NO_SINGLE_BINDING_MATCH,
            r#"
function sum(value: (int32, int32)): int32 {
    return match (value) {
        (left, right) => left + right
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function sum(value: (int32, int32)): int32 {
    return do {
        const (left, right) = value;
        left + right
    };
}
"#,
        );
    }

    /// Accept a single arm whose pattern can reject the scrutinee.
    #[test]
    fn test_accepts_refutable_pattern() {
        let session = TestSession::dir(
            &NO_SINGLE_BINDING_MATCH,
            r#"
function zero(value: 0): int32 {
    return match (value) {
        0 => 0
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
