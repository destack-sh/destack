use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow allocating conversions performed only to compare.
    pub NO_ALLOCATION_FOR_COMPARISON {
        id: "no-allocation-for-comparison",
        summary: "Disallow allocating conversions performed only to compare",
        explanation: r#"
Converting a borrowed string or path into owned storage solely for equality performs an allocation without changing the comparison.
Instead, you SHOULD compare the borrowed and owned representations directly.
"#,
        example: {
            reported: r#"
import { StringSlice } from "tspp:string";

function matches(value: &immutable StringSlice, expected: string): boolean {
    return value.toOwned() == expected;
}
"#,
            accepted: r#"
import { StringSlice } from "tspp:string";

function matches(value: &immutable StringSlice, expected: string): boolean {
    return value == expected;
}
"#,
        },
        provenance: [Clippy("cmp_owned")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report ownership conversions used only by equality.
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
            let Some(receiver) = to_owned_receiver(module, allocated, other)? else {
                continue;
            };

            let span = module.source_extent(allocated.into_any())?;
            let mut diagnostic = lint
                .diagnostic("comparison allocates an owned representation", span)
                .help("compare the borrowed representation directly");
            if let Some(suggestion) = replacement(module, lint, allocated, receiver)? {
                diagnostic = diagnostic.suggestion(suggestion);
            }
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Return the borrowed receiver copied by one canonical ToOwned call.
fn to_owned_receiver(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    other: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    if call.is_optional()
        || !call.generic_arguments.is_empty()
        || !call.arguments.is_empty()
        || module.implemented_language_member(expression)?
            != Some(dir::LanguageItem::ToOwned.member("toOwned"))
    {
        return Ok(None);
    }

    // require one canonical borrowed and owned representation pair
    let borrowed = module.representation_item(call.receiver.into_any())?;
    let owned = module.representation_item(other.into_any())?;
    let is_comparable = matches!(
        (borrowed, owned),
        (
            Some(dir::LanguageItem::StringSlice),
            Some(dir::LanguageItem::String)
        ) | (
            Some(dir::LanguageItem::CStringSlice),
            Some(dir::LanguageItem::CString)
        ) | (
            Some(dir::LanguageItem::OsStringSlice),
            Some(dir::LanguageItem::OsString)
        ) | (
            Some(dir::LanguageItem::PathSlice),
            Some(dir::LanguageItem::Path)
        )
    );
    if !is_comparable {
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
    let suggestion = lint.suggestion("compare the borrowed representation directly", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an owned conversion on the right operand.
    #[test]
    fn test_replaces_right_allocation() {
        let session = TestSession::dir(
            &NO_ALLOCATION_FOR_COMPARISON,
            r#"
import { StringSlice } from "tspp:string";

function differs(value: string, expected: &immutable StringSlice): boolean {
    return value != expected.toOwned();
}
"#,
        );

        session.assert_suggestions(
            r#"
import { StringSlice } from "tspp:string";

function differs(value: string, expected: &immutable StringSlice): boolean {
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
import { StringSlice } from "tspp:string";

function copy(value: &immutable StringSlice): ^string {
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

    /// Replace owned conversions for C strings.
    #[test]
    fn test_replaces_c_string_allocation() {
        let session = TestSession::dir(
            &NO_ALLOCATION_FOR_COMPARISON,
            r#"
import { CString, CStringSlice } from "tspp:string";

function matches(value: &immutable CStringSlice, expected: CString): boolean {
    return value.toOwned() == expected;
}
"#,
        );

        session.assert_suggestions(
            r#"
import { CString, CStringSlice } from "tspp:string";

function matches(value: &immutable CStringSlice, expected: CString): boolean {
    return value == expected;
}
"#,
        );
    }

    /// Replace owned conversions for platform strings.
    #[test]
    fn test_replaces_os_string_allocation() {
        let session = TestSession::dir(
            &NO_ALLOCATION_FOR_COMPARISON,
            r#"
import { OsString, OsStringSlice } from "tspp:string";

function matches(value: &immutable OsStringSlice, expected: OsString): boolean {
    return value.toOwned() == expected;
}
"#,
        );

        session.assert_suggestions(
            r#"
import { OsString, OsStringSlice } from "tspp:string";

function matches(value: &immutable OsStringSlice, expected: OsString): boolean {
    return value == expected;
}
"#,
        );
    }

    /// Replace owned conversions for paths.
    #[test]
    fn test_replaces_path_allocation() {
        let session = TestSession::dir(
            &NO_ALLOCATION_FOR_COMPARISON,
            r#"
import { Path, PathSlice } from "tspp:fs";

function matches(value: &immutable PathSlice, expected: Path): boolean {
    return value.toOwned() == expected;
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Path, PathSlice } from "tspp:fs";

function matches(value: &immutable PathSlice, expected: Path): boolean {
    return value == expected;
}
"#,
        );
    }
}
