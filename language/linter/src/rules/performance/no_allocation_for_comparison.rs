use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow allocating conversions performed only to compare.
    pub NO_ALLOCATION_FOR_COMPARISON {
        id: "no-allocation-for-comparison",
        summary: "Disallow allocating conversions performed only to compare",
        explanation: r#"
Converting a borrowed string slice into owned storage solely for equality performs an allocation without changing the comparison.
Instead, you SHOULD compare the borrowed and owned string representations directly.
"#,
        example: {
            reported: r#"
import { StringSlice } from "destack:string";

function matches(value: &readonly StringSlice, expected: string): boolean {
    return value.toOwned() == expected;
}
"#,
            accepted: r#"
import { StringSlice } from "destack:string";

function matches(value: &readonly StringSlice, expected: string): boolean {
    return value == expected;
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report StringSlice ownership conversions used only by equality.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect builtin equality comparisons
    for (_, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Binary {
            left,
            operator: dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual,
            right,
        } = node
        else {
            continue;
        };
        // report either allocating operand independently
        for (allocated, other) in [(*left, *right), (*right, *left)] {
            let Some(receiver) = to_owned_receiver(module, allocated)? else {
                continue;
            };
            if module.representation_item(other.into_any())? != Some(dir::LanguageItem::String) {
                continue;
            }

            let span = module.source_extent(allocated.into_any())?;
            let mut diagnostic = lint
                .diagnostic("comparison allocates an owned string", span)
                .help("compare the string slice directly");
            if let Some(suggestion) = replacement(module, lint, allocated, receiver)? {
                diagnostic = diagnostic.suggestion(suggestion);
            }
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Return the StringSlice receiver copied by one canonical ToOwned call.
fn to_owned_receiver(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    if call.is_optional()
        || !call.generic_arguments.is_empty()
        || !call.arguments.is_empty()
        || module.implemented_language_member(expression)?
            != Some(dir::LanguageItem::ToOwned.member("toOwned"))
        || module.representation_item(call.receiver.into_any())?
            != Some(dir::LanguageItem::StringSlice)
    {
        return Ok(None);
    }

    Ok(Some(call.receiver))
}

/// Suggest replacing one allocation with its retained receiver.
fn replacement(
    module: &DirModule<'_>,
    lint: &Lint,
    allocated: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(allocated.into_any())?;
    let retained = module.source_extent(receiver.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    let source = module.expression_source(receiver, dir::OperatorPrecedence::Equality)?;
    let mut file = FilePatch::new(extent.file);
    file.replace(extent, source);
    let suggestion = lint.suggestion("compare the string slice directly", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an owned conversion on the left operand.
    #[test]
    fn test_replaces_left_allocation() {
        TestSession::assert_example(&NO_ALLOCATION_FOR_COMPARISON);
    }

    /// Replace an owned conversion on the right operand.
    #[test]
    fn test_replaces_right_allocation() {
        let session = TestSession::dir(
            &NO_ALLOCATION_FOR_COMPARISON,
            r#"
import { StringSlice } from "destack:string";

function differs(value: string, expected: &readonly StringSlice): boolean {
    return value != expected.toOwned();
}
"#,
        );

        session.assert_suggestions(
            r#"
import { StringSlice } from "destack:string";

function differs(value: string, expected: &readonly StringSlice): boolean {
    return value != expected;
}
"#,
        );
    }

    /// Accept ownership conversion when the owned value is returned.
    #[test]
    fn test_accepts_returned_allocation() {
        let session = TestSession::dir(
            &NO_ALLOCATION_FOR_COMPARISON,
            r#"
import { StringSlice } from "destack:string";

function copy(value: &readonly StringSlice): ^string {
    return value.toOwned();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined toOwned method.
    #[test]
    fn test_accepts_unrelated_method() {
        let session = TestSession::dir(
            &NO_ALLOCATION_FOR_COMPARISON,
            r#"
struct Value {
    value: int32;
}

export extension of Value {
    toOwned(&readonly this): Value {
        return this;
    }
}

function matches(value: &readonly Value, expected: Value): boolean {
    return value.toOwned() == expected;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
