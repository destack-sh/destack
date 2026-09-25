use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer nth over drop followed by first.
    pub PREFER_NTH {
        id: "prefer-nth",
        summary: "Prefer nth over drop followed by first",
        explanation: r#"
Calling `first` after `drop` builds an adapter solely to consume its first value.
Instead, you SHOULD use `nth` to skip preceding values and return the selected value.
"#,
        example: {
            reported: r#"
import { Iterator } from "tspp:iter";

function select(values: Iterator<int32>, index: isize): int32 | undefined {
    return values.drop(index).first();
}
"#,
            accepted: r#"
import { Iterator } from "tspp:iter";

function select(values: Iterator<int32>, index: isize): int32 | undefined {
    return values.nth(index);
}
"#,
        },
        provenance: [Clippy("iter_skip_next")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One drop adapter immediately consumed with first.
#[derive(Debug, Clone, Copy)]
struct DroppedFirst {
    /// The complete drop adapter call.
    adapter: dir::LocalNodeId<dir::Expression>,
    /// The selected drop callee.
    callee: dir::LocalNodeId<dir::Expression>,
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
    if !first.generic_arguments.is_empty()
        || !first.arguments.is_empty()
        || module.language_member(expression)? != Some(dir::LanguageItem::Iterator.member("first"))
    {
        return Ok(None);
    }

    // require a canonical drop receiver with one positional count
    let Some(drop) = module.member_call(first.receiver) else {
        return Ok(None);
    };
    if !drop.generic_arguments.is_empty()
        || module.language_member(first.receiver)?
            != Some(dir::LanguageItem::Iterator.member("drop"))
    {
        return Ok(None);
    }
    let [argument] = drop.arguments else {
        return Ok(None);
    };
    let dir::Argument::Positional { .. } = module.view().get(*argument) else {
        return Ok(None);
    };

    Ok(Some(DroppedFirst {
        adapter: first.receiver,
        callee: drop.callee,
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
    let adapter = module.source_extent(dropped.adapter.into_any())?;
    if !extent.contains_span(adapter) {
        return Err(ProviderError::internal(
            "first call extent does not contain its drop receiver",
        ));
    }
    if module.has_unretained_comment(extent, &[adapter])? {
        return Ok(None);
    }

    // retain the complete adapter and replace its operation
    let suffix = Span::new(extent.file, adapter.end, extent.end);
    let mut file = FilePatch::new(extent.file);
    file.replace(module.main_span(dropped.callee.into_any())?, "nth");
    file.delete(suffix);
    file.sort();
    let fix = lint.fix("select the value directly", file)?;

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
import { Iterator } from "tspp:iter";

function select(values: Iterator<int32>, index: isize): int32 | undefined {
    return values.drop(index).first();
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "tspp:iter";

function select(values: Iterator<int32>, index: isize): int32 | undefined {
    return values.nth(index);
}
"#,
        );
    }

    /// Preserve an optional receiver when replacing the adapter chain.
    #[test]
    fn test_replaces_optional_drop_first() {
        let session = TestSession::dir(
            &PREFER_NTH,
            r#"
import { Iterator } from "tspp:iter";

function select(values: Iterator<int32> | undefined, index: isize): int32 | undefined {
    return values?.drop(index).first();
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "tspp:iter";

function select(values: Iterator<int32> | undefined, index: isize): int32 | undefined {
    return values?.nth(index);
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
import { Iterator } from "tspp:iter";

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

    /// Preserve comments between the adapter and terminal call by omitting the fix.
    #[test]
    fn test_reports_commented_chain_without_fix() {
        let session = TestSession::dir(
            &PREFER_NTH,
            r#"
import { Iterator } from "tspp:iter";

function select(values: Iterator<int32>, index: isize): int32 | undefined {
    return values.drop(index) /* retain */ .first();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-nth]: drop adapter is consumed only for its first value
 ──▶ main.tspp:4:12
  │
2 │
3 │ function select(values: Iterator<int32>, index: isize): int32 | undefined {
4 │     return values.drop(index) /* retain */ .first();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }
}
