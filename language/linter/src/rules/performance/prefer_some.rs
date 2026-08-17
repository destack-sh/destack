use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer some when a filtered array is observed only for emptiness.
    pub PREFER_SOME {
        id: "prefer-some",
        summary: "Prefer some when a filtered array is observed only for emptiness",
        explanation: r#"
Filtering an array to determine whether a matching element exists allocates every match.
Instead, you SHOULD call `some` to stop after determining the result.

`some` stops invoking the predicate after the first matching element.
"#,
        example: {
            reported: r#"
function hasPositive(values: int32[]): boolean {
    return !values.filter((value) => value > 0).isEmpty;
}
"#,
            accepted: r#"
function hasPositive(values: int32[]): boolean {
    return values.some((value) => value > 0);
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report filtered arrays used only by an emptiness property.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Array.isEmpty property reads
    for (is_empty, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Member {
            left: filter,
            is_optional: false,
            ..
        } = node
        else {
            continue;
        };
        if module.language_member(is_empty)? != Some(dir::LanguageItem::Array.member("isEmpty")) {
            continue;
        }

        // select an optional builtin negation around the emptiness read
        let parent = view.get_parent_for(is_empty);
        let (expression, is_negated) = match parent.and_then(|parent| {
            parent
                .try_into_typed::<dir::Expression>()
                .ok()
                .map(|parent| (parent, view.get(parent)))
        }) {
            Some((parent, dir::Expression::Unary { right, .. }))
                if *right == is_empty
                    && module
                        .builtin_unary(parent)?
                        .is_some_and(|(operator, _)| operator == dir::UnaryOperator::Not) =>
            {
                (parent, true)
            }
            _ => (is_empty, false),
        };

        // require the observed value to be one canonical Array.filter call
        let Some(filter_call) = module.member_call(*filter) else {
            continue;
        };
        if filter_call.is_optional()
            || filter_call.arguments.len() != 1
            || module.language_member(*filter)? != Some(dir::LanguageItem::Array.member("filter"))
        {
            continue;
        }

        // replace existence through allocation with Array.some
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("filtered array is used only for emptiness", span);
        if let Some(suggestion) = suggestion(
            module,
            lint,
            expression,
            *filter,
            filter_call.callee,
            is_negated,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the equivalent some query.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    filter: dir::LocalNodeId<dir::Expression>,
    filter_member: dir::LocalNodeId<dir::Expression>,
    is_negated: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let filter_extent = module.source_extent(filter.into_any())?;
    if !extent.contains_span(filter_extent) {
        return Err(ProviderError::internal(
            "filtered emptiness extent does not contain its filter call",
        ));
    }
    if module.has_unretained_comment(extent, &[filter_extent])? {
        return Ok(None);
    }

    // retain the filter call while replacing its member and surrounding query
    let prefix = Span::new(extent.file, extent.start, filter_extent.start);
    let suffix = Span::new(extent.file, filter_extent.end, extent.end);
    let mut file = FilePatch::new(extent.file);
    if !prefix.is_empty() {
        file.delete(prefix);
    }
    if !is_negated {
        file.insert(filter_extent.start, "!");
    }
    file.replace(module.main_span(filter_member.into_any())?, "some");
    file.delete(suffix);
    file.sort();
    let suggestion = lint.suggestion("test matching elements directly", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace nonempty filtered-array observation with some.
    #[test]
    fn test_replaces_negated_filter_is_empty() {
        let session = TestSession::dir(
            &PREFER_SOME,
            r#"
function hasPositive(values: int32[]): boolean {
    return !values.filter((value) => value > 0).isEmpty;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-some]: filtered array is used only for emptiness
 ──▶ main.ds:2:12
  │
1 │ function hasPositive(values: int32[]): boolean {
2 │     return !values.filter((value) => value > 0).isEmpty;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = suggestion: test matching elements directly (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function hasPositive(values: int32[]): boolean {
-   2│     return !values.filter((value) => value > 0).isEmpty;
+   2│     return values.some((value) => value > 0);
"#,
        );
        session.assert_suggestions(PREFER_SOME.example.accepted());
    }

    /// Negate some when the filtered array is required to be empty.
    #[test]
    fn test_replaces_filter_is_empty() {
        let session = TestSession::dir(
            &PREFER_SOME,
            r#"
function hasNoPositive(values: int32[]): boolean {
    return values.filter((value) => value > 0).isEmpty;
}
"#,
        );

        session.assert_suggestions(
            r#"
function hasNoPositive(values: int32[]): boolean {
    return !values.some((value) => value > 0);
}
"#,
        );
    }

    /// Accept a filtered array used as a value.
    #[test]
    fn test_accepts_filtered_values() {
        let session = TestSession::dir(
            &PREFER_SOME,
            r#"
function positives(values: int32[]): int32[] {
    return values.filter((value) => value > 0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined filter and isEmpty members.
    #[test]
    fn test_accepts_user_members() {
        let session = TestSession::dir(
            &PREFER_SOME,
            r#"
class Values {
    get isEmpty(): boolean {
        return false;
    }

    filter(predicate: (value: int32) => boolean): this {
        return this;
    }
}

function hasValue(values: Values): boolean {
    return !values.filter((value) => value > 0).isEmpty;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
