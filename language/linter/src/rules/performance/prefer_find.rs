use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, PatchSet, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer find when consuming the first filtered element.
    pub PREFER_FIND {
        id: "prefer-find",
        summary: "Prefer find when consuming the first filtered element",
        explanation: "Filtering an array before optionally reading its first element allocates and examines every input. Use `find` to stop at the first match. This changes how often an effectful predicate runs, so the correction requires review.",
        example: {
            reported: r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).at(0);
}
"#,
            accepted: r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.find((value) => value > 0);
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report filtered arrays whose first element is consumed optionally.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect checked calls that optionally consume one first element
    for expression in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Call {
            left, arguments, ..
        } = view.get(expression)
        else {
            continue;
        };
        let dir::Expression::Member { left: filter, .. } = view.get(*left) else {
            continue;
        };

        // require optional first-element behavior from the canonical Array member
        let consumer = module.language_member(expression)?;
        let at = dir::LanguageMember::named(dir::LanguageItem::Array, "at");
        let first = dir::LanguageMember::named(dir::LanguageItem::Array, "first");
        let is_first = if consumer == Some(at) {
            let [argument] = arguments.as_slice() else {
                continue;
            };
            let Some(index) = view.get(*argument).value() else {
                continue;
            };

            matches!(
                module.scalar_constant(index)?,
                Some(dir::ScalarLiteral::Integer(0))
            )
        } else if consumer == Some(first) {
            arguments.is_empty()
        } else {
            false
        };
        if !is_first {
            continue;
        }

        // require the receiver to be one canonical Array.filter call
        let dir::Expression::Call {
            left: filter_member,
            ..
        } = view.get(*filter)
        else {
            continue;
        };
        let dir::Expression::Member { .. } = view.get(*filter_member) else {
            continue;
        };
        let filter_language_member = dir::LanguageMember::named(dir::LanguageItem::Array, "filter");
        if module.language_member(*filter)? != Some(filter_language_member) {
            continue;
        }

        // report the allocation and offer the short-circuiting search
        let span = module.span(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("filtered array is only used for its first element", span);
        if let Some(suggestion) = suggest_find(module, expression, *filter, *filter_member, lint)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Suggest replacing one filtered first-element read with `find`.
fn suggest_find(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    filter: dir::LocalNodeId<dir::Expression>,
    filter_member: dir::LocalNodeId<dir::Expression>,
    lint: &Lint,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let expression = module.source_extent(expression.into_any())?;
    let filter = module.source_extent(filter.into_any())?;
    if expression.file != filter.file || filter.end > expression.end {
        return Err(ProviderError::internal(
            "filtered first-element extent does not contain its filter call",
        ));
    }
    if module.has_unretained_comment(expression, &[filter])? {
        return Ok(None);
    }

    // replace the method name and remove the optional first-element suffix
    let filter_member = module.main_span(filter_member.into_any())?;
    let suffix = Span::new(expression.file, filter.end, expression.end);
    let mut file = FilePatch::new(expression.file);
    file.replace(filter_member, "find");
    file.delete(suffix);
    file.sort();
    let patches = PatchSet::single(file);
    let suggestion = lint.suggestion("search the array directly", patches)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report `first()` after the canonical Array filter.
    #[test]
    fn test_reports_filter_first() {
        let session = TestSession::new(
            &PREFER_FIND,
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).first();
}
"#,
        );

        session.assert_suggestions(PREFER_FIND.example.accepted());
    }

    /// Accept checked indexing because it traps instead of returning undefined.
    #[test]
    fn test_accepts_filter_index_zero() {
        let session = TestSession::new(
            &PREFER_FIND,
            r#"
function firstPositive(values: int32[]): int32 {
    return values.filter((value) => value > 0)[0];
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept optional lookup at any position other than the first.
    #[test]
    fn test_accepts_filter_at_nonzero_index() {
        let session = TestSession::new(
            &PREFER_FIND,
            r#"
function secondPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).at(1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve an optional receiver when replacing the filtered read.
    #[test]
    fn test_reports_optional_filter() {
        let session = TestSession::new(
            &PREFER_FIND,
            r#"
function firstPositive(values: int32[] | undefined): int32 | undefined {
    return values?.filter((value) => value > 0).at(0);
}
"#,
        );

        session.assert_suggestions(
            r#"
function firstPositive(values: int32[] | undefined): int32 | undefined {
    return values?.find((value) => value > 0);
}
"#,
        );
    }

    /// Accept a user-defined method named filter.
    #[test]
    fn test_accepts_user_filter_method() {
        let session = TestSession::new(
            &PREFER_FIND,
            r#"
class Values {
    filter(predicate: (value: int32) => boolean): this {
        return this;
    }

    at(index: number): int32 | undefined {
        return undefined;
    }
}

function firstPositive(values: Values): int32 | undefined {
    return values.filter((value) => value > 0).at(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
