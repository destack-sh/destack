use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer direct collection cloning over iterating, cloning, and collecting.
    pub ITER_CLONED_COLLECT {
        id: "iter-cloned-collect",
        summary: "Prefer direct collection cloning over iterating, cloning, and collecting",
        explanation: r#"
Iterating a borrowed contiguous collection, cloning every element, and collecting them rebuilds the same owned array indirectly.
Instead, you SHOULD create the owned array directly from the collection.
"#,
        example: {
            reported: r#"
struct Label {
    values: ^int32[];
}

function copy(values: &readonly Label[]): Label[] {
    return values.iterator().cloned().toArray();
}
"#,
            accepted: r#"
struct Label {
    values: ^int32[];
}

function copy(values: &readonly Label[]): Label[] {
    return values.clone();
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report borrowed contiguous collections cloned completely into a new array.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical iterator collection into arrays
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(collect) = module.member_call(expression) else {
            continue;
        };
        let collection = module.language_member(expression)?;
        let is_collection = collection == Some(dir::LanguageItem::Iterator.member("toArray"))
            || collection == Some(dir::LanguageItem::Iterator.member("collect"));
        if collect.is_optional()
            || !collect.arguments.is_empty()
            || !is_collection
            || module.representation_item(expression.into_any())? != Some(dir::LanguageItem::Array)
        {
            continue;
        }

        // require one canonical cloned adapter
        let Some(cloned) = module.member_call(collect.receiver) else {
            continue;
        };
        if cloned.is_optional()
            || !cloned.generic_arguments.is_empty()
            || !cloned.arguments.is_empty()
            || module.language_member(collect.receiver)?
                != Some(dir::LanguageItem::Iterator.member("cloned"))
        {
            continue;
        }

        // require one direct canonical borrowed contiguous iterator
        let Some(iterator) = module.member_call(cloned.receiver) else {
            continue;
        };
        let iterator_member = module.language_member(cloned.receiver)?;
        let method = match iterator_member {
            Some(member)
                if member == dir::LanguageItem::Array.member("iterator")
                    || member == dir::LanguageItem::Array.member("values") =>
            {
                "clone"
            }
            Some(member) if member == dir::LanguageItem::Slice.member("iterator") => "toOwned",
            _ => continue,
        };
        if iterator.is_optional()
            || !iterator.generic_arguments.is_empty()
            || !iterator.arguments.is_empty()
        {
            continue;
        }

        // replace the indirect element clone with direct collection ownership
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("collection is cloned through its iterator", span);
        if let Some(suggestion) = suggestion(module, lint, expression, iterator.receiver, method)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace an indirect collection clone with its direct ownership operation.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    collection: dir::LocalNodeId<dir::Expression>,
    method: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let collection_extent = module.source_extent(collection.into_any())?;
    if !extent.contains_span(collection_extent) {
        return Err(ProviderError::internal(
            "iterator collection extent does not contain its collection receiver",
        ));
    }
    if module.has_unretained_comment(extent, &[collection_extent])? {
        return Ok(None);
    }

    // preserve authored grouping around the cloned collection expression
    let collection = module.expression_source(collection, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, format!("{collection}.{method}()"));
    let suggestion = lint.fix("clone the collection directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a cloned iterator collected through toArray.
    #[test]
    fn test_replaces_cloned_to_array() {
        let session =
            TestSession::dir(&ITER_CLONED_COLLECT, ITER_CLONED_COLLECT.example.reported());

        session.assert_diagnostics(
            r#"
warning[iter-cloned-collect]: collection is cloned through its iterator
 ──▶ main.ds:6:12
  │
5 │ function copy(values: &readonly Label[]): Label[] {
6 │     return values.iterator().cloned().toArray();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │ }
  │

 = fix: clone the collection directly
--- a/main.ds
+++ b/main.ds

    5│ function copy(values: &readonly Label[]): Label[] {
-   6│     return values.iterator().cloned().toArray();
+   6│     return values.clone();
"#,
        );
        session.assert_fixes(ITER_CLONED_COLLECT.example.accepted());
    }

    /// Replace a values iterator collected through collect.
    #[test]
    fn test_replaces_cloned_collect() {
        let session = TestSession::dir(
            &ITER_CLONED_COLLECT,
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &readonly Label[]): Label[] {
    return values.values().cloned().collect();
}
"#,
        );

        session.assert_fixes(ITER_CLONED_COLLECT.example.accepted());
    }

    /// Replace cloned slice iteration with direct owned collection creation.
    #[test]
    fn test_replaces_cloned_slice() {
        let session = TestSession::dir(
            &ITER_CLONED_COLLECT,
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &readonly [Label]): Label[] {
    return values.iterator().cloned().toArray();
}
"#,
        );

        session.assert_fixes(
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &readonly [Label]): Label[] {
    return values.toOwned();
}
"#,
        );
    }

    /// Accept an iterator transformed before collection.
    #[test]
    fn test_accepts_transformed_iterator() {
        let session = TestSession::dir(
            &ITER_CLONED_COLLECT,
            r#"
struct Label {
    values: ^int32[];
}

function lengths(values: &readonly Label[]): isize[] {
    return values.iterator().cloned().map((value) => value.values.length).toArray();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept cloning values from a non-array iterator.
    #[test]
    fn test_accepts_other_iterator() {
        let session = TestSession::dir(
            &ITER_CLONED_COLLECT,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function copy(values: Iterator<&readonly Label>): Label[] {
    return values.cloned().toArray();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined methods with the same names.
    #[test]
    fn test_accepts_user_methods() {
        let session = TestSession::dir(
            &ITER_CLONED_COLLECT,
            r#"
class Values {
    iterator(): this {
        return this;
    }

    cloned(): this {
        return this;
    }

    toArray(): string[] {
        return [];
    }
}

function copy(values: Values): string[] {
    return values.iterator().cloned().toArray();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Retain comments without offering a destructive fix.
    #[test]
    fn test_retains_comment() {
        let session = TestSession::dir(
            &ITER_CLONED_COLLECT,
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &readonly Label[]): Label[] {
    return values.iterator().cloned(/* retain */).toArray();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[iter-cloned-collect]: collection is cloned through its iterator
 ──▶ main.ds:6:12
  │
5 │ function copy(values: &readonly Label[]): Label[] {
6 │     return values.iterator().cloned(/* retain */).toArray();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │ }
  │
"#,
        );
    }
}
