use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Iterator.inspect when mapping observes and returns each value.
    pub MANUAL_ITERATOR_INSPECT {
        id: "manual-iterator-inspect",
        summary: "Prefer Iterator.inspect when mapping observes and returns each value",
        explanation: r#"
Mapping an iterator value only to observe it before returning it unchanged obscures the side effect.
Instead, you SHOULD call `inspect` to observe each value without changing the sequence.
"#,
        example: {
            reported: r#"
import { Iterator } from "tspp:iter";

declare function record(value: &readonly int32): void;

function observe(values: Iterator<int32>): Iterator<int32> {
    return values.map((value) => {
        record(value);
        value
    });
}
"#,
            accepted: r#"
import { Iterator } from "tspp:iter";

declare function record(value: &readonly int32): void;

function observe(values: Iterator<int32>): Iterator<int32> {
    return values.inspect((value) => {
        record(value);
    });
}
"#,
        },
        provenance: [Clippy("manual_inspect")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report Iterator.map callbacks that only observe and return each value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Iterator.map callbacks
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Iterator.member("map"))
        {
            continue;
        }
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value: callback } = view.get(*argument) else {
            continue;
        };

        // require an iterator callback arity accepted by inspect
        let Some(lambda) = module.lambda(*callback) else {
            continue;
        };
        if lambda.signature.parameters.is_empty() || lambda.signature.parameters.len() > 2 {
            continue;
        }
        let Some(removed) = module.removable_parameter_return(*callback)? else {
            continue;
        };

        // require the identity mapping to preserve the iterator type
        if module.adjusted_type_id(expression.into_any())?
            != module.adjusted_type_id(call.receiver.into_any())?
        {
            continue;
        }

        // replace map and remove the identity tail
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("iterator map only observes each value", span);
        if call.generic_arguments.is_empty()
            && let Some(fix) = fix(module, lint, call.callee, removed)?
        {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one observing map callback with Iterator.inspect.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    callee: dir::LocalNodeId<dir::Expression>,
    removed: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let removed = module.statement_removal_span(removed)?;
    if module.has_unretained_comment(removed, &[])? {
        return Ok(None);
    }

    // replace the method and remove the returned value
    let member = module.main_span(callee.into_any())?;
    let mut patch = FilePatch::new(member.file);
    patch.replace(member, "inspect");
    patch.delete(removed);
    patch.sort();
    let suggestion = lint.suggestion("observe values with Iterator.inspect", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an observing identity map with inspect.
    #[test]
    fn test_replaces_observing_map() {
        let session = TestSession::dir(
            &MANUAL_ITERATOR_INSPECT,
            r#"
import { Iterator } from "tspp:iter";

declare function record(value: &readonly int32): void;

function observe(values: Iterator<int32>): Iterator<int32> {
    return values.map((value) => {
        record(value);
        value
    });
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Iterator } from "tspp:iter";

declare function record(value: &readonly int32): void;

function observe(values: Iterator<int32>): Iterator<int32> {
    return values.inspect((value) => {
        record(value);
    });
}
"#,
        );
    }

    /// Accept mapping that changes each value.
    #[test]
    fn test_accepts_changed_value() {
        let session = TestSession::dir(
            &MANUAL_ITERATOR_INSPECT,
            r#"
import { Iterator } from "tspp:iter";

function increment(values: Iterator<int32>): Iterator<int32> {
    return values.map((value) => value + 1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept observing work that consumes each value.
    #[test]
    fn test_accepts_consumed_value() {
        let session = TestSession::dir(
            &MANUAL_ITERATOR_INSPECT,
            r#"
import { Iterator } from "tspp:iter";

declare function consume(value: ^string): void;

function observe(values: Iterator<^string>): Iterator<^string> {
    return values.map((value) => {
        consume(value);
        value
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an identity callback whose result is coerced to another element type.
    #[test]
    fn test_accepts_coerced_value() {
        let session = TestSession::dir(
            &MANUAL_ITERATOR_INSPECT,
            r#"
import { Iterator } from "tspp:iter";

declare function record(value: &readonly int32): void;

function observe(values: Iterator<int32>): Iterator<unknown> {
    return values.map<unknown>((value) => {
        record(value);
        value
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
