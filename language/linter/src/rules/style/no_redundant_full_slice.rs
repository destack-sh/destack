use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow indexing a slice with the complete range.
    pub NO_REDUNDANT_FULL_SLICE {
        id: "no-redundant-full-slice",
        summary: "Disallow indexing a slice with the complete range",
        explanation: r#"
Indexing a slice with `..` borrows the complete slice without changing its bounds.
Instead, you SHOULD use the slice directly and let the required access be inferred.
"#,
        example: {
            reported: r#"
function visit(values: [int32]): void {
    consume(values[..]);
}

function consume(values: &readonly [int32]): void {
    // intentionally empty
}
"#,
            accepted: r#"
function visit(values: [int32]): void {
    consume(values);
}

function consume(values: &readonly [int32]): void {
    // intentionally empty
}
"#,
        },
        provenance: [Clippy("redundant_slicing")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report complete-range indexing of canonical slices.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect direct complete-range index expressions
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Index {
            left: receiver,
            index: Some(range),
            is_optional: false,
            ..
        } = node
        else {
            continue;
        };
        let dir::Expression::RangeExpression {
            start: None,
            end: None,
            end_kind: dir::RangeEnd::Open,
        } = view.get(*range)
        else {
            continue;
        };
        if module.representation_item(receiver.into_any())? != Some(dir::LanguageItem::Slice)
            || module.node_type_id(expression.into_any())?
                != module.node_type_id(receiver.into_any())?
        {
            continue;
        }

        // replace the redundant index while retaining its receiver
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("complete range repeats the same slice", span);
        if let Some(fix) = fix(module, lint, expression, *receiver)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Remove one complete-range index from a slice expression.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver = module.source_extent(receiver.into_any())?;
    if module.has_unretained_comment(extent, &[receiver])? {
        return Ok(None);
    }

    // retain the exact receiver expression
    let replacement = module.source(receiver)?;
    let patch = Patch::replace(extent, replacement);
    let fix = lint.fix("use the slice directly", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove complete-range indexing from a slice argument.
    #[test]
    fn test_removes_complete_range() {
        let session = TestSession::dir(
            &NO_REDUNDANT_FULL_SLICE,
            r#"
function visit(values: [int32]): void {
    consume(values[..]);
}

function consume(values: &readonly [int32]): void {
    // intentionally empty
}
"#,
        );

        session.assert_fixes(
            r#"
function visit(values: [int32]): void {
    consume(values);
}

function consume(values: &readonly [int32]): void {
    // intentionally empty
}
"#,
        );
    }

    /// Remove complete-range indexing from an inferred binding.
    #[test]
    fn test_removes_inferred_complete_range() {
        let session = TestSession::dir(
            &NO_REDUNDANT_FULL_SLICE,
            r#"
function length(values: [int32]): isize {
    const selected = values[..];

    return selected.length;
}
"#,
        );

        session.assert_fixes(
            r#"
function length(values: [int32]): isize {
    const selected = values;

    return selected.length;
}
"#,
        );
    }

    /// Accept a bounded slice.
    #[test]
    fn test_accepts_bounded_range() {
        let session = TestSession::dir(
            &NO_REDUNDANT_FULL_SLICE,
            r#"
function visit(values: [int32]): void {
    consume(values[1..]);
}

function consume(values: &readonly [int32]): void {
    // intentionally empty
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a complete range over an array because it changes the representation.
    #[test]
    fn test_accepts_array_range() {
        let session = TestSession::dir(
            &NO_REDUNDANT_FULL_SLICE,
            r#"
function visit(values: int32[]): void {
    consume(values[..]);
}

function consume(values: &readonly [int32]): void {
    // intentionally empty
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
