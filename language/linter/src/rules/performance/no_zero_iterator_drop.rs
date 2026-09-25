use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow Iterator.drop with a constant zero count.
    pub NO_ZERO_ITERATOR_DROP {
        id: "no-zero-iterator-drop",
        summary: "Disallow Iterator.drop with a constant zero count",
        explanation: r#"
Dropping zero values wraps an Iterator without changing the sequence it yields.
Instead, you SHOULD use the original Iterator directly.
"#,
        example: {
            reported: r#"
import { Iterator } from "tspp:iter";

function retain(values: Iterator<int32>): Iterator<int32> {
    return values.drop(0);
}
"#,
            accepted: r#"
import { Iterator } from "tspp:iter";

function retain(values: Iterator<int32>): Iterator<int32> {
    return values;
}
"#,
        },
        provenance: [Clippy("iter_skip_zero")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report Iterator.drop calls with a constant zero count.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Iterator.drop calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Iterator.member("drop"))
        {
            continue;
        }
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value } = view.get(*argument) else {
            continue;
        };
        if module.integral_constant(*value)? != Some(0) {
            continue;
        }

        // remove the adapter while retaining its Iterator
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("Iterator.drop skips zero values", span);
        if let Some(suggestion) = suggestion(module, lint, expression, call.receiver)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the original Iterator replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver_extent = module.source_extent(receiver.into_any())?;
    if module.has_unretained_comment(extent, &[receiver_extent])? {
        return Ok(None);
    }

    // retain the original Iterator expression
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, receiver);
    let suggestion = lint.suggestion("use the original Iterator", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept Iterator.drop with a positive count.
    #[test]
    fn test_accepts_positive_drop() {
        let session = TestSession::dir(
            &NO_ZERO_ITERATOR_DROP,
            r#"
import { Iterator } from "tspp:iter";

function tail(values: Iterator<int32>): Iterator<int32> {
    return values.drop(1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a runtime drop count.
    #[test]
    fn test_accepts_runtime_count() {
        let session = TestSession::dir(
            &NO_ZERO_ITERATOR_DROP,
            r#"
import { Iterator } from "tspp:iter";

function tail(values: Iterator<int32>, count: isize): Iterator<int32> {
    return values.drop(count);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept Iterator.take with a zero limit because it yields an empty Iterator.
    #[test]
    fn test_accepts_zero_take() {
        let session = TestSession::dir(
            &NO_ZERO_ITERATOR_DROP,
            r#"
import { Iterator } from "tspp:iter";

function empty(values: Iterator<int32>): Iterator<int32> {
    return values.take(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
