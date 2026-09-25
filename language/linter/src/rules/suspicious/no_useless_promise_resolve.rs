use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow Promise.resolve calls that preserve the same Promise.
    pub NO_USELESS_PROMISE_RESOLVE {
        id: "no-useless-promise-resolve",
        summary: "Disallow Promise.resolve calls that preserve the same Promise",
        explanation: r#"
`Promise.resolve` returns its argument unchanged when that argument is already a Promise.
Instead, you SHOULD use the existing Promise directly.
"#,
        example: {
            reported: r#"
import { Promise } from "tspp:async";

function retain(value: Promise<int32>): Promise<int32> {
    return Promise.resolve(value);
}
"#,
            accepted: r#"
import { Promise } from "tspp:async";

function retain(value: Promise<int32>): Promise<int32> {
    return value;
}
"#,
        },
        provenance: [Unicorn("no-useless-promise-resolve-reject")],
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report Promise.resolve calls receiving an existing Promise.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Promise.resolve calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Promise.member("resolve"))
        {
            continue;
        }
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value } = view.get(*argument) else {
            continue;
        };

        // require the resolved value itself to be a Promise
        let value_type = module.adjusted_type_id(value.into_any())?;
        if module.dir.representation_item(value_type)? != Some(dir::LanguageItem::Promise) {
            continue;
        }

        // replace the wrapper while retaining the argument
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("Promise.resolve receives a Promise", span);
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

    // retain the existing Promise expression
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, value);
    let suggestion = lint.fix("use the existing Promise", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace Promise.resolve around an existing Promise.
    #[test]
    fn test_replaces_existing_promise() {
        TestSession::assert_example(&NO_USELESS_PROMISE_RESOLVE);
    }

    /// Replace Promise.resolve around an aliased Promise type.
    #[test]
    fn test_replaces_aliased_promise() {
        let session = TestSession::dir(
            &NO_USELESS_PROMISE_RESOLVE,
            r#"
import { Promise } from "tspp:async";

type Pending = Promise<int32>;

function retain(value: Pending): Promise<int32> {
    return Promise.resolve(value);
}
"#,
        );

        session.assert_fixes(
            r#"
import { Promise } from "tspp:async";

type Pending = Promise<int32>;

function retain(value: Pending): Promise<int32> {
    return value;
}
"#,
        );
    }

    /// Accept Promise.resolve around a plain value.
    #[test]
    fn test_accepts_plain_value() {
        let session = TestSession::dir(
            &NO_USELESS_PROMISE_RESOLVE,
            r#"
import { Promise } from "tspp:async";

function resolve(value: int32): Promise<int32> {
    return Promise.resolve(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
