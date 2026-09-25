use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow Promise.race over one Promise.
    pub NO_SINGLE_PROMISE_RACE {
        id: "no-single-promise-race",
        summary: "Disallow Promise.race over one Promise",
        explanation: r#"
Racing one Promise cannot select among competing asynchronous work and often indicates a missing Promise.
Instead, you SHOULD use the existing Promise directly when no second input is intended.

`Promise.race` creates a distinct Promise and schedules its settlement through the combinator.
"#,
        example: {
            reported: r#"
import { Promise } from "tspp:async";

function first(value: Promise<int32>): Promise<int32> {
    return Promise.race([value]);
}
"#,
            accepted: r#"
import { Promise } from "tspp:async";

function first(value: Promise<int32>): Promise<int32> {
    return value;
}
"#,
        },
        provenance: [Unicorn("no-single-promise-in-promise-methods")],
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report Promise.race calls containing one existing Promise.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Promise.race calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Promise.member("race"))
        {
            continue;
        }
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value: values } = view.get(*argument) else {
            continue;
        };
        let dir::Expression::ArrayExpression { elements } = view.get(*values) else {
            continue;
        };
        let [element] = elements.as_slice() else {
            continue;
        };
        let dir::Argument::Positional { value } = view.get(*element) else {
            continue;
        };

        // require the sole value itself to be a Promise
        let value_type = module.node_type_id(value.into_any())?;
        if module.dir.representation_item(value_type)? != Some(dir::LanguageItem::Promise) {
            continue;
        }

        // replace the race with its sole Promise
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("Promise.race contains one Promise", span);
        if let Some(suggestion) = suggestion(module, lint, expression, *value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the direct Promise replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let value_extent = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[value_extent])? {
        return Ok(None);
    }

    // retain the sole Promise expression
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, value);
    let suggestion = lint.suggestion("use the existing Promise", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace Promise.race around one Promise.
    #[test]
    fn test_replaces_single_promise_race() {
        TestSession::assert_example(&NO_SINGLE_PROMISE_RACE);
    }

    /// Accept Promise.race over multiple Promises.
    #[test]
    fn test_accepts_multiple_promises() {
        let session = TestSession::dir(
            &NO_SINGLE_PROMISE_RACE,
            r#"
import { Promise } from "tspp:async";

function first(left: Promise<int32>, right: Promise<int32>): Promise<int32> {
    return Promise.race([left, right]);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept Promise.all because it preserves aggregate result shape.
    #[test]
    fn test_accepts_single_promise_all() {
        let session = TestSession::dir(
            &NO_SINGLE_PROMISE_RACE,
            r#"
import { Promise } from "tspp:async";

function gather(value: Promise<int32>): Promise<int32[]> {
    return Promise.all([value]);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept Promise.race over one plain value because it creates a Promise.
    #[test]
    fn test_accepts_single_plain_value() {
        let session = TestSession::dir(
            &NO_SINGLE_PROMISE_RACE,
            r#"
import { Promise } from "tspp:async";

function first(value: int32): Promise<int32> {
    return Promise.race([value]);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
