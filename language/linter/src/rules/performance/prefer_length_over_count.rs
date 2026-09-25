use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer stored array length over counting an unchanged iterator.
    pub PREFER_LENGTH_OVER_COUNT {
        id: "prefer-length-over-count",
        summary: "Prefer stored array length over counting an unchanged iterator",
        explanation: r#"
Counting every value from an unchanged array iterator performs linear work to recover the array's stored length.
Instead, you SHOULD read `length` from the array directly.
"#,
        example: {
            reported: r#"
function length(values: int32[]): isize {
    return values.iterator().count();
}
"#,
            accepted: r#"
function length(values: int32[]): isize {
    return values.length;
}
"#,
        },
        provenance: [Clippy("iter_count")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report counts of canonical unchanged array iterators.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical Iterator.count calls without arguments
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(count) = module.member_call(expression) else {
            continue;
        };
        if count.is_optional()
            || !count.arguments.is_empty()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Iterator.member("count"))
        {
            continue;
        }

        // require one direct argument free member call
        let Some(iterator) = module.member_call(count.receiver) else {
            continue;
        };
        if iterator.is_optional()
            || !iterator.generic_arguments.is_empty()
            || !iterator.arguments.is_empty()
        {
            continue;
        }

        // accept array iterations that preserve cardinality
        let member = module.language_member(count.receiver)?;
        if member != Some(dir::LanguageItem::Array.member("iterator"))
            && member != Some(dir::LanguageItem::Array.member("values"))
            && member != Some(dir::LanguageItem::Array.member("keys"))
            && member != Some(dir::LanguageItem::Array.member("entries"))
        {
            continue;
        }
        let Some(receiver_type) = module.call_receiver_type_id(count.receiver)? else {
            continue;
        };
        if matches!(
            module.dir.get_type(receiver_type)?,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Owned,
                ..
            })
        ) {
            continue;
        }

        // replace iterator traversal with the stored array length
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("array iterator is counted in full", span);
        if let Some(suggestion) = suggestion(module, lint, expression, iterator.receiver)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one direct array length access while retaining its receiver.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver = module.source_extent(receiver.into_any())?;
    if !extent.contains_span(receiver) {
        return Err(ProviderError::internal(
            "iterator count extent does not contain its array receiver",
        ));
    }
    if module.has_unretained_comment(extent, &[receiver])? {
        return Ok(None);
    }

    // retain the receiver and replace the complete iterator suffix
    let suffix = Span::new(extent.file, receiver.end, extent.end);
    let mut file = FilePatch::new(extent.file);
    file.replace(suffix, ".length");
    let suggestion = lint.fix("read the stored array length", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace counting an array iterator with its length.
    #[test]
    fn test_replaces_array_iterator_count() {
        let session = TestSession::dir(
            &PREFER_LENGTH_OVER_COUNT,
            r#"
function length(values: int32[]): isize {
    return values.iterator().count();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-length-over-count]: array iterator is counted in full
 ──▶ main.tspp:2:12
  │
1 │ function length(values: int32[]): isize {
2 │     return values.iterator().count();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: read the stored array length
--- a/main.tspp
+++ b/main.tspp

    1│ function length(values: int32[]): isize {
-   2│     return values.iterator().count();
+   2│     return values.length;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function length(values: int32[]): isize {
    return values.length;
}
"#,
        );
    }

    /// Replace counting array keys, values, and entries with the same length.
    #[test]
    fn test_replaces_array_views() {
        let session = TestSession::dir(
            &PREFER_LENGTH_OVER_COUNT,
            r#"
function lengths(values: int32[]): isize {
    return values.keys().count() + values.values().count() + values.entries().count();
}
"#,
        );

        session.assert_fixes(
            r#"
function lengths(values: int32[]): isize {
    return values.length + values.length + values.length;
}
"#,
        );
    }

    /// Accept counting an iterator after an adapter changes its length.
    #[test]
    fn test_accepts_adapted_iterator() {
        let session = TestSession::dir(
            &PREFER_LENGTH_OVER_COUNT,
            r#"
function positiveLength(values: int32[]): isize {
    return values.iterator().filter((value) => value > 0).count();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Retain an owned Array iterator whose count consumes the Array.
    #[test]
    fn test_accepts_consuming_array_iterator() {
        let session = TestSession::dir(
            &PREFER_LENGTH_OVER_COUNT,
            r#"
function length(values: ^int32[]): isize {
    return values.iterator().count();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined iterator method.
    #[test]
    fn test_accepts_user_iterator() {
        let session = TestSession::dir(
            &PREFER_LENGTH_OVER_COUNT,
            r#"
class Values {
    iterator(): Iterator<int32> {
        let values: int32[] = [1];

        return values.iterator();
    }
}

function length(values: Values): isize {
    return values.iterator().count();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Retain comments without offering a destructive fix.
    #[test]
    fn test_retains_iterator_comment() {
        let session = TestSession::dir(
            &PREFER_LENGTH_OVER_COUNT,
            r#"
function length(values: int32[]): isize {
    return values.iterator() /* every value */.count();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-length-over-count]: array iterator is counted in full
 ──▶ main.tspp:2:12
  │
1 │ function length(values: int32[]): isize {
2 │     return values.iterator() /* every value */.count();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
