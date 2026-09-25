use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow awaiting elements passed to Promise concurrency methods.
    pub NO_AWAIT_IN_PROMISE_METHODS {
        id: "no-await-in-promise-methods",
        summary: "Disallow awaiting elements passed to Promise concurrency methods",
        explanation: r#"
Awaiting a Promise inside the input array delays every following element and can serialize their operations.
Instead, you SHOULD pass each Promise directly so `Promise.all` or `Promise.race` can observe them together.
"#,
        example: {
            reported: r#"
import { Promise } from "tspp:async";

declare function first(): Promise<int32>;
declare function second(): Promise<int32>;

async function gather(): Promise<int32[]> {
    return await Promise.all([await first(), second()]);
}
"#,
            accepted: r#"
import { Promise } from "tspp:async";

declare function first(): Promise<int32>;
declare function second(): Promise<int32>;

async function gather(): Promise<int32[]> {
    return await Promise.all([first(), second()]);
}
"#,
        },
        provenance: [Unicorn("no-await-in-promise-methods")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report direct awaited array elements passed to Promise.all or Promise.race.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical concurrent Promise calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || !matches!(
                module.language_member(expression)?,
                Some(member)
                    if member == dir::LanguageItem::Promise.member("all")
                        || member == dir::LanguageItem::Promise.member("race")
            )
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

        // report direct awaited array elements
        for element in elements {
            let dir::Argument::Positional { value: awaited } = view.get(*element) else {
                continue;
            };
            let dir::Expression::Await { expression: value } = view.get(*awaited) else {
                continue;
            };
            if module.representation_item(value.into_any())? != Some(dir::LanguageItem::Promise) {
                continue;
            }

            let span = module.source_extent(awaited.into_any())?;
            let mut diagnostic = lint.diagnostic("await delays a Promise concurrency method", span);
            if let Some(suggestion) = suggestion(module, lint, *awaited, *value)? {
                diagnostic = diagnostic.suggestion(suggestion);
            }
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Build the direct Promise argument replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    awaited: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(awaited.into_any())?;
    let value_extent = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[value_extent])? {
        return Ok(None);
    }

    // remove the await prefix while retaining the complete Promise expression
    let prefix = Span::new(extent.file, extent.start, value_extent.start);
    let patch = Patch::replace(prefix, "");
    let suggestion = lint.suggestion("pass the Promise directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove await inside a Promise.all argument.
    #[test]
    fn test_replaces_await_inside_promise_all() {
        TestSession::assert_example(&NO_AWAIT_IN_PROMISE_METHODS);
    }

    /// Remove await inside a Promise.race argument.
    #[test]
    fn test_replaces_await_inside_promise_race() {
        let session = TestSession::dir(
            &NO_AWAIT_IN_PROMISE_METHODS,
            r#"
import { Promise } from "tspp:async";

async function first(left: Promise<int32>, right: Promise<int32>): Promise<int32> {
    return await Promise.race([left, await right]);
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Promise } from "tspp:async";

async function first(left: Promise<int32>, right: Promise<int32>): Promise<int32> {
    return await Promise.race([left, right]);
}
"#,
        );
    }

    /// Accept await inside a deferred callback argument.
    #[test]
    fn test_accepts_await_inside_callback() {
        let session = TestSession::dir(
            &NO_AWAIT_IN_PROMISE_METHODS,
            r#"
import { Promise } from "tspp:async";

async function deferred(value: Promise<int32>): Promise<(() => Promise<int32>)[]> {
    return await Promise.all([async () => await value]);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept await nested inside an array element operation.
    #[test]
    fn test_accepts_nested_await() {
        let session = TestSession::dir(
            &NO_AWAIT_IN_PROMISE_METHODS,
            r#"
import { Promise } from "tspp:async";

declare function wrap(value: int32): Promise<int32>;

async function gather(value: Promise<int32>): Promise<int32[]> {
    return await Promise.all([wrap(await value)]);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a directly awaited value represented by another awaitable type.
    #[test]
    fn test_accepts_other_awaitable() {
        let session = TestSession::dir(
            &NO_AWAIT_IN_PROMISE_METHODS,
            r#"
import { Promise, Task } from "tspp:async";

declare function work(): Task<int32>;

async function gather(): Promise<int32[]> {
    return await Promise.all([await work()]);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Remove each nested eager await once.
    #[test]
    fn test_replaces_nested_awaits_once() {
        let session = TestSession::dir(
            &NO_AWAIT_IN_PROMISE_METHODS,
            r#"
import { Promise } from "tspp:async";

async function gather(value: Promise<int32>): Promise<int32[]> {
    return await Promise.all([await Promise.race([await value])]);
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Promise } from "tspp:async";

async function gather(value: Promise<int32>): Promise<int32[]> {
    return await Promise.all([Promise.race([value])]);
}
"#,
        );
    }
}
