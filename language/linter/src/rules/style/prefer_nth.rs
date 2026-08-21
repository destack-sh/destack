use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer nth over drop followed by first.
    pub PREFER_NTH {
        id: "prefer-nth",
        summary: "Prefer nth over drop followed by first",
        explanation: r#"
Calling `first` immediately after `drop` creates an adapter only to consume its first value.
Instead, you SHOULD call `nth` to consume the preceding values and return the selected value directly.
"#,
        example: {
            reported: r#"
import { Iterator } from "destack:iter";

function select(values: Iterator<int32>, index: isize): int32 | undefined {
    return values.drop(index).first();
}
"#,
            accepted: r#"
import { Iterator } from "destack:iter";

function select(values: Iterator<int32>, index: isize): int32 | undefined {
    return values.nth(index);
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One drop adapter immediately consumed with first.
#[derive(Debug, Clone, Copy)]
struct DroppedFirst {
    /// The source iterator.
    receiver: dir::LocalNodeId<dir::Expression>,
    /// The number of values discarded before selection.
    count: dir::LocalNodeId<dir::Expression>,
}

/// Report Iterator.drop calls immediately consumed with first.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical Iterator.first calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(dropped) = dropped_first(module, expression)? else {
            continue;
        };

        // replace both operations with direct indexed consumption
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("drop adapter is consumed only for its first value", span);
        if let Some(fix) = fix(module, lint, expression, dropped)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one canonical Iterator.drop call immediately consumed with first.
fn dropped_first(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DroppedFirst>, ProviderError> {
    let Some(first) = module.member_call(expression) else {
        return Ok(None);
    };
    if first.is_optional()
        || !first.generic_arguments.is_empty()
        || !first.arguments.is_empty()
        || module.language_member(expression)? != Some(dir::LanguageItem::Iterator.member("first"))
    {
        return Ok(None);
    }

    // require a canonical drop receiver with one positional count
    let Some(drop) = module.member_call(first.receiver) else {
        return Ok(None);
    };
    if drop.is_optional()
        || !drop.generic_arguments.is_empty()
        || module.language_member(first.receiver)?
            != Some(dir::LanguageItem::Iterator.member("drop"))
    {
        return Ok(None);
    }
    let [argument] = drop.arguments else {
        return Ok(None);
    };
    let dir::Argument::Positional { value: count } = module.view().get(*argument) else {
        return Ok(None);
    };

    Ok(Some(DroppedFirst {
        receiver: drop.receiver,
        count: *count,
    }))
}

/// Replace one drop-first chain with Iterator.nth.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    dropped: DroppedFirst,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver = module.source_extent(dropped.receiver.into_any())?;
    let count = module.source_extent(dropped.count.into_any())?;
    if module.has_unretained_comment(extent, &[receiver, count])? {
        return Ok(None);
    }

    // retain the authored iterator and count
    let receiver = module.expression_source(dropped.receiver, dir::OperatorPrecedence::Postfix)?;
    let count = module.source(count)?;
    let patch = Patch::replace(extent, format!("{receiver}.nth({count})"));
    let fix = lint.fix("select the value directly", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace Iterator.drop followed by first.
    #[test]
    fn test_replaces_drop_first() {
        let session = TestSession::dir(
            &PREFER_NTH,
            r#"
import { Iterator } from "destack:iter";

function select(values: Iterator<int32>, index: isize): int32 | undefined {
    return values.drop(index).first();
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "destack:iter";

function select(values: Iterator<int32>, index: isize): int32 | undefined {
    return values.nth(index);
}
"#,
        );
    }

    /// Accept drop when its resulting iterator remains observable.
    #[test]
    fn test_accepts_retained_drop() {
        let session = TestSession::dir(
            &PREFER_NTH,
            r#"
import { Iterator } from "destack:iter";

function select(values: Iterator<int32>, index: isize): Iterator<int32> {
    return values.drop(index);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined drop and first methods.
    #[test]
    fn test_accepts_user_methods() {
        let session = TestSession::dir(
            &PREFER_NTH,
            r#"
class Values {
    drop(count: isize): this {
        return this;
    }

    first(): int32 {
        return 0;
    }
}

function select(values: Values): int32 {
    return values.drop(1).first();
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
