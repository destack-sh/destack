use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer `includes` for array membership tests.
    pub PREFER_INCLUDES {
        id: "prefer-includes",
        summary: "Prefer `includes` for array membership tests",
        explanation: r#"
Comparing an array index search with `undefined` expresses membership indirectly through a discarded index.
Instead, you SHOULD call `includes` to express membership directly.

`lastIndexOf` searches from the end, so substituting `includes` changes the order of element comparisons.
"#,
        example: {
            reported: r#"
function contains(values: int32[], target: int32): boolean {
    return values.indexOf(target) !== undefined;
}
"#,
            accepted: r#"
function contains(values: int32[], target: int32): boolean {
    return values.includes(target);
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report canonical array index searches compared with undefined.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect builtin equality expressions with one exact undefined operand
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };
        if !operator.is_equality() {
            continue;
        }

        // select one canonical array index search
        let left = left.source.local_id;
        let right = right.source.local_id;
        let mut membership = None;
        for (search, absence) in [(left, right), (right, left)] {
            if view.get(absence).as_scalar() != Some(dir::ScalarLiteral::Undefined) {
                continue;
            }
            let Some(call) = module.member_call(search) else {
                continue;
            };
            if call.is_optional || call.is_member_optional {
                continue;
            }

            // distinguish forward searches from equivalent unbounded reverse searches
            let selected = module.language_member(search)?;
            let index_of = dir::LanguageItem::Array.member("indexOf");
            let last_index_of = dir::LanguageItem::Array.member("lastIndexOf");
            let is_reverse = if selected == Some(index_of) {
                false
            } else if selected == Some(last_index_of) && call.arguments.len() == 1 {
                true
            } else {
                continue;
            };

            membership = Some((search, call.callee, call.receiver, is_reverse));
            break;
        }
        let Some((search, member, receiver, is_reverse)) = membership else {
            continue;
        };

        // retain the defining search inside Array.includes
        let is_this = matches!(view.get(receiver), dir::Expression::This);
        let includes = dir::LanguageItem::Array.member("includes");
        let is_implementation =
            module.is_within_language_member(expression.into_any(), includes)?;
        if is_this && is_implementation {
            continue;
        }

        // replace the complete membership comparison
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("array index is only used for membership", span);
        let is_negated = !operator.is_negative_equality();
        if let Some(suggestion) =
            suggestion(module, lint, span, search, member, is_negated, is_reverse)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the equivalent array membership query.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: Span,
    search: dir::LocalNodeId<dir::Expression>,
    member: dir::LocalNodeId<dir::Expression>,
    is_negated: bool,
    is_reverse: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let search = module.source_extent(search.into_any())?;
    if !extent.contains_span(search) {
        return Err(ProviderError::internal(
            "membership comparison extent does not contain its index search",
        ));
    }
    if module.has_unretained_comment(extent, &[search])? {
        return Ok(None);
    }

    // retain the complete call while replacing its method and surrounding comparison
    let mut file = FilePatch::new(extent.file);
    let prefix = Span::new(extent.file, extent.start, search.start);
    let suffix = Span::new(extent.file, search.end, extent.end);
    if !prefix.is_empty() {
        file.delete(prefix);
    }
    if is_negated {
        file.insert(search.start, "!");
    }
    file.replace(module.main_span(member.into_any())?, "includes");
    if !suffix.is_empty() {
        file.delete(suffix);
    }
    file.sort();

    // preserve forward traversal automatically and review changed traversal order
    let suggestion = if is_reverse {
        lint.suggestion("query array membership directly", file)?
    } else {
        lint.fix("query array membership directly", file)?
    };

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a forward index search with a direct membership query.
    #[test]
    fn test_replaces_index_search() {
        let session = TestSession::dir(
            &PREFER_INCLUDES,
            r#"
function contains(values: int32[], target: int32): boolean {
    return values.indexOf(target) !== undefined;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-includes]: array index is only used for membership
 ──▶ main.ds:2:12
  │
1 │ function contains(values: int32[], target: int32): boolean {
2 │     return values.indexOf(target) !== undefined;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: query array membership directly
--- a/main.ds
+++ b/main.ds

    1│ function contains(values: int32[], target: int32): boolean {
-   2│     return values.indexOf(target) !== undefined;
+   2│     return values.includes(target);
"#,
        );
        session.assert_fixes(PREFER_INCLUDES.example.accepted());
    }

    /// Negate equality with undefined in either operand order.
    #[test]
    fn test_replaces_absent_index_searches() {
        let session = TestSession::dir(
            &PREFER_INCLUDES,
            r#"
function missing(values: int32[], target: int32): boolean {
    return undefined == values.indexOf(target);
}
function missingAfter(values: int32[], target: int32, start: isize): boolean {
    return values.indexOf(target, start) === undefined;
}
"#,
        );

        session.assert_fixes(
            r#"
function missing(values: int32[], target: int32): boolean {
    return !values.includes(target);
}
function missingAfter(values: int32[], target: int32, start: isize): boolean {
    return !values.includes(target, start);
}
"#,
        );
    }

    /// Require review when replacing an unbounded reverse search.
    #[test]
    fn test_suggests_replacing_reverse_search() {
        let session = TestSession::dir(
            &PREFER_INCLUDES,
            r#"
function contains(values: int32[], target: int32): boolean {
    return values.lastIndexOf(target) != undefined;
}
"#,
        );

        session.assert_suggestions(
            r#"
function contains(values: int32[], target: int32): boolean {
    return values.includes(target);
}
"#,
        );
    }

    /// Accept a bounded reverse search because includes scans the other suffix.
    #[test]
    fn test_accepts_bounded_reverse_search() {
        let session = TestSession::dir(
            &PREFER_INCLUDES,
            r#"
function containsBefore(values: int32[], target: int32, end: isize): boolean {
    return values.lastIndexOf(target, end) != undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept String.indexOf until its absent-result contract is explicit.
    #[test]
    fn test_accepts_string_index_search() {
        let session = TestSession::dir(
            &PREFER_INCLUDES,
            r#"
function contains(value: string, search: string): boolean {
    return value.indexOf(search) >= 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an optional receiver because absence is not a boolean false result.
    #[test]
    fn test_accepts_optional_search() {
        let session = TestSession::dir(
            &PREFER_INCLUDES,
            r#"
function contains(values: int32[] | undefined, target: int32): boolean {
    return values?.indexOf(target) !== undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined method named indexOf.
    #[test]
    fn test_accepts_user_index_search() {
        let session = TestSession::dir(
            &PREFER_INCLUDES,
            r#"
class Values {
    indexOf(value: int32): usize | undefined {
        return undefined;
    }
}
function contains(values: Values, target: int32): boolean {
    return values.indexOf(target) !== undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve comparison comments by omitting the correction.
    #[test]
    fn test_reports_commented_comparison_without_suggestion() {
        let session = TestSession::dir(
            &PREFER_INCLUDES,
            r#"
function contains(values: int32[], target: int32): boolean {
    return values.indexOf(target) /* membership */ !== undefined;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-includes]: array index is only used for membership
 ──▶ main.ds:2:12
  │
1 │ function contains(values: int32[], target: int32): boolean {
2 │     return values.indexOf(target) /* membership */ !== undefined;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
