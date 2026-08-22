use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow awaiting inside concurrent combinator arguments.
    pub NO_AWAIT_IN_CONCURRENT_COMBINATOR {
        id: "no-await-in-concurrent-combinator",
        summary: "Disallow awaiting inside concurrent combinator arguments",
        explanation: r#"
Awaiting a Promise before passing it to `Promise.all` or `Promise.race` delays construction of the concurrent operation.
Instead, you SHOULD pass every Promise directly to the combinator.
"#,
        example: {
            reported: r#"
import { Promise } from "destack:async";

async function gather(left: Promise<int32>, right: Promise<int32>): Promise<int32[]> {
    return await Promise.all([await left, right]);
}
"#,
            accepted: r#"
import { Promise } from "destack:async";

async function gather(left: Promise<int32>, right: Promise<int32>): Promise<int32[]> {
    return await Promise.all([left, right]);
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report await expressions evaluated before Promise.all or Promise.race begins.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();
    let mut reported = Vec::new();

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
        let callable = module.enclosing_callable_body(expression.into_any());

        // report awaits evaluated within a direct call argument
        for (awaited, node) in view.iter_nodes::<dir::Expression>() {
            let dir::Expression::Await { expression: value } = node else {
                continue;
            };
            let is_argument = call
                .arguments
                .iter()
                .any(|argument| view.is_inside(awaited.into_any(), argument.into_any()));
            if !is_argument
                || module.enclosing_callable_body(awaited.into_any()) != callable
                || reported.contains(&awaited)
            {
                continue;
            }
            reported.push(awaited);

            let span = module.source_extent(awaited.into_any())?;
            let mut diagnostic =
                lint.diagnostic("await delays construction of a concurrent Promise", span);
            if let Some(suggestion) = suggestion(module, lint, awaited, *value)? {
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
        TestSession::assert_example(&NO_AWAIT_IN_CONCURRENT_COMBINATOR);
    }

    /// Remove await inside a Promise.race argument.
    #[test]
    fn test_replaces_await_inside_promise_race() {
        let session = TestSession::dir(
            &NO_AWAIT_IN_CONCURRENT_COMBINATOR,
            r#"
import { Promise } from "destack:async";

async function first(left: Promise<int32>, right: Promise<int32>): Promise<int32> {
    return await Promise.race([left, await right]);
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Promise } from "destack:async";

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
            &NO_AWAIT_IN_CONCURRENT_COMBINATOR,
            r#"
import { Promise } from "destack:async";

async function deferred(value: Promise<int32>): Promise<(() => Promise<int32>)[]> {
    return await Promise.all([async () => await value]);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Remove each nested eager await once.
    #[test]
    fn test_replaces_nested_awaits_once() {
        let session = TestSession::dir(
            &NO_AWAIT_IN_CONCURRENT_COMBINATOR,
            r#"
import { Promise } from "destack:async";

async function gather(value: Promise<int32>): Promise<int32[]> {
    return await Promise.all([await Promise.race([await value])]);
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Promise } from "destack:async";

async function gather(value: Promise<int32>): Promise<int32[]> {
    return await Promise.all([Promise.race([value])]);
}
"#,
        );
    }
}
