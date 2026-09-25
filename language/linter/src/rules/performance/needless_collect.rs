use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow materializing an Iterator before a terminal operation.
    pub NEEDLESS_COLLECT {
        id: "needless-collect",
        summary: "Disallow materializing an Iterator before a terminal operation",
        explanation: r#"
Materializing an Iterator before a terminal operation allocates storage for values that are immediately discarded.
Instead, you SHOULD apply the terminal operation to the Iterator directly.

Materialization may intentionally isolate the terminal operation from later mutations, so the rewrite requires review.
"#,
        example: {
            reported: r#"
function length(values: Iterator<int32>): isize {
    return values.toArray().length;
}
"#,
            accepted: r#"
function length(values: Iterator<int32>): isize {
    return values.count();
}
"#,
        },
        provenance: [Clippy("needless_collect")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report Iterator collections immediately consumed by a streaming operation.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical Array length reads
    for (expression, node) in module.view().iter_nodes::<dir::Expression>() {
        let dir::Expression::Member {
            left: collection,
            is_optional: false,
            ..
        } = node
        else {
            continue;
        };
        if module.language_member(expression)? != Some(dir::LanguageItem::Array.member("length")) {
            continue;
        }
        let Some(iterator) = collected_iterator(module, *collection)? else {
            continue;
        };

        // replace allocation and stored length with Iterator.count
        let span = module.source_extent(collection.into_any())?;
        let mut diagnostic =
            lint.diagnostic("Iterator is materialized only to read its length", span);
        if let Some(suggestion) = replace_length(module, lint, expression, iterator)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    // inspect Array terminal calls already available on Iterator
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(terminal) = module.member_call(expression) else {
            continue;
        };
        if terminal.is_optional() {
            continue;
        }
        let Some(member) = module.language_member(expression)? else {
            continue;
        };
        if member != dir::LanguageItem::Array.member("every")
            && member != dir::LanguageItem::Array.member("find")
            && member != dir::LanguageItem::Array.member("findIndex")
            && member != dir::LanguageItem::Array.member("first")
            && member != dir::LanguageItem::Array.member("forEach")
            && member != dir::LanguageItem::Array.member("last")
            && member != dir::LanguageItem::Array.member("reduce")
            && member != dir::LanguageItem::Array.member("some")
        {
            continue;
        }
        let Some(iterator) = collected_iterator(module, terminal.receiver)? else {
            continue;
        };

        // require one direct argument free member call
        let Some(iteration) = module.member_call(iterator) else {
            continue;
        };
        if iteration.is_optional()
            || !iteration.generic_arguments.is_empty()
            || !iteration.arguments.is_empty()
        {
            continue;
        }

        // require direct array iteration before changing evaluation order
        let member = module.language_member(iterator)?;
        if member != Some(dir::LanguageItem::Array.member("iterator"))
            && member != Some(dir::LanguageItem::Array.member("values"))
            && member != Some(dir::LanguageItem::Array.member("keys"))
            && member != Some(dir::LanguageItem::Array.member("entries"))
        {
            continue;
        }

        // suggest direct iteration because collection may establish a snapshot
        let span = module.source_extent(terminal.receiver.into_any())?;
        let mut diagnostic = lint.diagnostic(
            "Iterator is materialized before an equivalent terminal operation",
            span,
        );
        if let Some(suggestion) = remove_collection(module, lint, terminal.receiver, iterator)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return one array collection call and its source Iterator.
fn collected_iterator(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some(collection) = module.member_call(expression) else {
        return Ok(None);
    };
    let member = module.language_member(expression)?;
    let is_collection = member == Some(dir::LanguageItem::Iterator.member("toArray"))
        || member == Some(dir::LanguageItem::Iterator.member("collect"));
    if collection.is_optional()
        || !collection.arguments.is_empty()
        || !is_collection
        || module.representation_item(expression.into_any())? != Some(dir::LanguageItem::Array)
    {
        return Ok(None);
    }

    Ok(Some(collection.receiver))
}

/// Replace one collected array length with Iterator.count.
fn replace_length(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    iterator: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let expression = module.source_extent(expression.into_any())?;
    let iterator = module.source_extent(iterator.into_any())?;
    if !expression.contains_span(iterator) {
        return Err(ProviderError::internal(
            "array length extent does not contain its Iterator source",
        ));
    }
    if module.has_unretained_comment(expression, &[iterator])? {
        return Ok(None);
    }

    // replace collection and length suffixes together
    let suffix = Span::new(expression.file, iterator.end, expression.end);
    let mut file = FilePatch::new(expression.file);
    file.replace(suffix, ".count()");
    let suggestion = lint.fix("count the Iterator directly", file)?;

    Ok(Some(suggestion))
}

/// Remove one intermediate Array collection while retaining its Iterator.
fn remove_collection(
    module: &DirModule<'_>,
    lint: &Lint,
    collection: dir::LocalNodeId<dir::Expression>,
    iterator: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let collection = module.source_extent(collection.into_any())?;
    let iterator = module.source_extent(iterator.into_any())?;
    if !collection.contains_span(iterator) {
        return Err(ProviderError::internal(
            "array collection extent does not contain its Iterator source",
        ));
    }
    if module.has_unretained_comment(collection, &[iterator])? {
        return Ok(None);
    }

    // delete the collection suffix before the retained terminal call
    let suffix = Span::new(collection.file, iterator.end, collection.end);
    let mut file = FilePatch::new(collection.file);
    file.delete(suffix);
    let suggestion = lint.suggestion("apply the terminal operation to the Iterator", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an intermediate array length with Iterator.count.
    #[test]
    fn test_replaces_collection_length() {
        let session = TestSession::dir(
            &NEEDLESS_COLLECT,
            r#"
function length(values: Iterator<int32>): isize {
    return values.toArray().length;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[needless-collect]: Iterator is materialized only to read its length
 ──▶ main.tspp:2:12
  │
1 │ function length(values: Iterator<int32>): isize {
2 │     return values.toArray().length;
  │            ^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: count the Iterator directly
--- a/main.tspp
+++ b/main.tspp

    1│ function length(values: Iterator<int32>): isize {
-   2│     return values.toArray().length;
+   2│     return values.count();
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function length(values: Iterator<int32>): isize {
    return values.count();
}
"#,
        );
    }

    /// Remove collection before equivalent Iterator terminal operations.
    #[test]
    fn test_suggests_direct_terminal_operations() {
        let session = TestSession::dir(
            &NEEDLESS_COLLECT,
            r#"
function inspect(first: int32[], second: int32[]): boolean {
    first.iterator().toArray().forEach((value, index) => {
        value;
        index;
    });

    return second.iterator().toArray().some((value, index) => value > index.truncate<int32>());
}

function findIndex(values: int32[]): isize | undefined {
    return values.iterator().toArray().findIndex((value) => value > 0);
}

function firstValue(values: int32[]): int32 | undefined {
    return values.iterator().toArray().first();
}

function lastValue(values: int32[]): int32 | undefined {
    return values.iterator().toArray().last();
}
"#,
        );

        session.assert_suggestions(
            r#"
function inspect(first: int32[], second: int32[]): boolean {
    first.iterator().forEach((value, index) => {
        value;
        index;
    });

    return second.iterator().some((value, index) => value > index.truncate<int32>());
}

function findIndex(values: int32[]): isize | undefined {
    return values.iterator().findIndex((value) => value > 0);
}

function firstValue(values: int32[]): int32 | undefined {
    return values.iterator().first();
}

function lastValue(values: int32[]): int32 | undefined {
    return values.iterator().last();
}
"#,
        );
    }

    /// Retain collection before a terminal operation on an arbitrary Iterator.
    #[test]
    fn test_accepts_effectful_iterator_terminal() {
        let session = TestSession::dir(
            &NEEDLESS_COLLECT,
            r#"
function any(values: Iterator<int32>): boolean {
    return values.toArray().some((value) => value > 0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Retain collection before an Array operation without an Iterator equivalent.
    #[test]
    fn test_accepts_collection_before_array_adapter() {
        let session = TestSession::dir(
            &NEEDLESS_COLLECT,
            r#"
function positive(values: Iterator<int32>): int32[] {
    return values.toArray().filter((value) => value > 0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a generic Array collection consumed by length.
    #[test]
    fn test_replaces_generic_collection_length() {
        let session = TestSession::dir(
            &NEEDLESS_COLLECT,
            r#"
function length(values: Iterator<int32>): isize {
    return values.collect<^int32[]>().length;
}
"#,
        );

        session.assert_fixes(
            r#"
function length(values: Iterator<int32>): isize {
    return values.count();
}
"#,
        );
    }

    /// Accept a collection retained as the final value.
    #[test]
    fn test_accepts_retained_collection() {
        let session = TestSession::dir(
            &NEEDLESS_COLLECT,
            r#"
function collect(values: Iterator<int32>): int32[] {
    return values.toArray();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined collection methods.
    #[test]
    fn test_accepts_user_methods() {
        let session = TestSession::dir(
            &NEEDLESS_COLLECT,
            r#"
class Values {
    toArray(): int32[] {
        return [];
    }
}

function length(values: Values): isize {
    return values.toArray().length;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Retain comments attached to the removed collection suffix.
    #[test]
    fn test_retains_collection_comment() {
        let session = TestSession::dir(
            &NEEDLESS_COLLECT,
            r#"
function length(values: Iterator<int32>): isize {
    return values.toArray() /* materialized values */.length;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[needless-collect]: Iterator is materialized only to read its length
 ──▶ main.tspp:2:12
  │
1 │ function length(values: Iterator<int32>): isize {
2 │     return values.toArray() /* materialized values */.length;
  │            ^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Accept a stored field initialized by its constructor.
    #[test]
    fn test_accepts_field_initialization() {
        let session = TestSession::dir(
            &NEEDLESS_COLLECT,
            r#"
class Holder {
    readonly value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
