use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer `RegExp.test` when only match existence is observed.
    pub PREFER_REGEXP_TEST {
        id: "prefer-regexp-test",
        summary: "Prefer `RegExp.test` when only match existence is observed",
        explanation: r#"
Comparing a regular-expression match or search result only with its absence value discards the returned match details or position.
Instead, you SHOULD use `RegExp.test` when the condition only needs a boolean result.
"#,
        example: {
            reported: r#"
function containsWord(text: string): boolean {
    return text.match(/word/) !== null;
}
"#,
            accepted: r#"
function containsWord(text: string): boolean {
    return /word/.test(text);
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// The operands and absence value selected by one matching call.
#[derive(Debug, Clone, Copy)]
struct MatchOperands {
    /// The regular expression pattern.
    pattern: dir::LocalNodeId<dir::Expression>,
    /// The searched string.
    input: dir::LocalNodeId<dir::Expression>,
    /// The value produced when no match exists.
    absence: dir::Literal,
    /// Whether conversion to `RegExp.test` reverses evaluation order.
    is_reordered: bool,
}

/// Report regular-expression match calls compared only with their absence value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect checked builtin strict equality comparisons
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };
        if !operator.is_strict_equality() {
            continue;
        }

        // select one match call and its exact absence value
        let mut selected = None;
        for (call, absence) in [
            (left.source.local_id, right.source.local_id),
            (right.source.local_id, left.source.local_id),
        ] {
            let Some(operands) = match_operands(module, call)? else {
                continue;
            };
            if view.get(absence).as_scalar() != Some(operands.absence) {
                continue;
            }

            selected = Some(operands);
            break;
        }
        let Some(operands) = selected else {
            continue;
        };

        // replace the complete comparison with the boolean query
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("match details are discarded after testing", span);
        let is_negated = !operator.is_negative_equality();
        if let Some(suggestion) = suggestion(module, lint, expression, operands, is_negated)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the operands selected by one canonical matching call.
fn match_operands(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<MatchOperands>, ProviderError> {
    let view = module.view();
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    if call.is_optional() {
        return Ok(None);
    }
    let [argument] = call.arguments else {
        return Ok(None);
    };
    let Some(argument) = view.get(*argument).value() else {
        return Ok(None);
    };

    // preserve RegExp.exec evaluation order
    let member = module.language_member(expression)?;
    if member == Some(dir::LanguageItem::RegExp.member("exec")) {
        return Ok(Some(MatchOperands {
            pattern: call.receiver,
            input: argument,
            absence: dir::Literal::Null,
            is_reordered: false,
        }));
    }

    // require a fresh literal because reusable regexes carry matching state
    let absence = if member == Some(dir::LanguageItem::String.member("match")) {
        dir::Literal::Null
    } else if member == Some(dir::LanguageItem::String.member("search")) {
        dir::Literal::Undefined
    } else {
        return Ok(None);
    };
    if !matches!(
        view.get(argument),
        dir::Expression::Literal(dir::Literal::RegexString { .. })
    ) {
        return Ok(None);
    }

    Ok(Some(MatchOperands {
        pattern: argument,
        input: call.receiver,
        absence,
        is_reordered: true,
    }))
}

/// Build one direct regular-expression test.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    operands: MatchOperands,
    is_negated: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let pattern_span = module.source_extent(operands.pattern.into_any())?;
    let input_span = module.source_extent(operands.input.into_any())?;
    if module.has_unretained_comment(extent, &[pattern_span, input_span])? {
        return Ok(None);
    }

    // preserve evaluation order when moving String.match operands
    if operands.is_reordered
        && (!module.is_speculatable_expression(operands.pattern)?
            || !module.is_speculatable_expression(operands.input)?)
    {
        return Ok(None);
    }

    // retain exact sources and group the postfix pattern when required
    let pattern = module.expression_source(operands.pattern, dir::OperatorPrecedence::Postfix)?;
    let input = module.source(input_span)?;
    let negation = if is_negated { "!" } else { "" };
    let replacement = format!("{negation}{pattern}.test({input})");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("test the regular expression directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a null comparison around RegExp.exec.
    #[test]
    fn test_replaces_exec_comparison() {
        let session = TestSession::dir(
            &PREFER_REGEXP_TEST,
            r#"
function misses(text: string): boolean {
    return /word/.exec(text) === null;
}
"#,
        );

        session.assert_fixes(
            r#"
function misses(text: string): boolean {
    return !/word/.test(text);
}
"#,
        );
    }

    /// Replace an undefined comparison around String.search.
    #[test]
    fn test_replaces_search_comparison() {
        let session = TestSession::dir(
            &PREFER_REGEXP_TEST,
            r#"
function containsWord(text: string): boolean {
    return text.search(/word/) !== undefined;
}
"#,
        );

        session.assert_fixes(
            r#"
function containsWord(text: string): boolean {
    return /word/.test(text);
}
"#,
        );
    }

    /// Report a test around an effectful string receiver.
    #[test]
    fn test_reports_effectful_input() {
        let session = TestSession::dir(
            &PREFER_REGEXP_TEST,
            r#"
declare function read(): string;

function containsWord(): boolean {
    return read().match(/word/) !== null;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-regexp-test]: match details are discarded after testing
 ──▶ main.ds:4:12
  │
2 │
3 │ function containsWord(): boolean {
4 │     return read().match(/word/) !== null;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Accept a match result whose details are returned.
    #[test]
    fn test_accepts_consumed_match() {
        let session = TestSession::dir(
            &PREFER_REGEXP_TEST,
            r#"
function match(text: string): unknown {
    return /word/.exec(text);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep a reusable regular expression whose matching state may be observed.
    #[test]
    fn test_accepts_reusable_regexp() {
        let session = TestSession::dir(
            &PREFER_REGEXP_TEST,
            r#"
function matches(text: string, pattern: RegExp): boolean {
    return text.match(pattern) !== null;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
