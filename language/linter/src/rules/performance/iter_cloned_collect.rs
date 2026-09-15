use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer direct collection copying over iterating elements into a new array.
    pub ITER_CLONED_COLLECT {
        id: "iter-cloned-collect",
        summary: "Prefer direct collection copying over iterator collection",
        explanation: r#"
Iterating a borrowed contiguous collection, copying every element, and collecting them rebuilds the same owned array indirectly.
Instead, you SHOULD create the owned array directly from the collection.
"#,
        example: {
            reported: r#"
struct Label {
    values: ^int32[];
}

function copy(values: &immutable Label[]): Label[] {
    return values.iterator().cloned().toArray();
}
"#,
            accepted: r#"
struct Label {
    values: ^int32[];
}

function copy(values: &immutable Label[]): Label[] {
    return values.clone();
}
"#,
        },
        provenance: [Clippy("iter_cloned_collect")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One contiguous collection copied indirectly through its Iterator.
#[derive(Debug, Clone, Copy)]
struct CollectionCopy {
    /// The complete collection expression.
    expression: dir::LocalNodeId<dir::Expression>,
    /// The contiguous source collection.
    source: dir::LocalNodeId<dir::Expression>,
    /// The direct ownership method.
    method: &'static str,
    /// Whether direct ownership must retain an optional chain.
    is_optional: bool,
}

impl CollectionCopy {
    /// Select one complete copied or cloned Iterator collection.
    fn select(
        module: &DirModule<'_>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Self>, ProviderError> {
        let Some(collection) = module.member_call(expression) else {
            return Ok(None);
        };
        let member = module.language_member(expression)?;
        let is_collection = member == Some(dir::LanguageItem::Iterator.member("toArray"))
            || member == Some(dir::LanguageItem::Iterator.member("collect"));
        if !collection.arguments.is_empty()
            || !is_collection
            || module.representation_item(expression.into_any())? != Some(dir::LanguageItem::Array)
        {
            return Ok(None);
        }

        // peel one canonical copied or cloned adapter when present
        let possible_copy = module.member_call(collection.receiver);
        let copy_member = module.language_member(collection.receiver)?;
        let is_copy = copy_member == Some(dir::LanguageItem::Iterator.member("cloned"))
            || copy_member == Some(dir::LanguageItem::Iterator.member("copied"));
        let iterator_expression = if is_copy {
            let Some(copy) = possible_copy else {
                return Ok(None);
            };
            if !copy.generic_arguments.is_empty() || !copy.arguments.is_empty() {
                return Ok(None);
            }

            copy.receiver
        } else {
            collection.receiver
        };

        // require one direct canonical contiguous iterator
        let Some(iterator) = module.member_call(iterator_expression) else {
            return Ok(None);
        };
        let iterator_member = module.language_member(iterator_expression)?;
        let method = match iterator_member {
            Some(member)
                if member == dir::LanguageItem::Array.member("iterator")
                    || member == dir::LanguageItem::Array.member("values") =>
            {
                "clone"
            }
            Some(member) if member == dir::LanguageItem::Slice.member("iterator") => "toOwned",
            _ => return Ok(None),
        };
        if !iterator.generic_arguments.is_empty() || !iterator.arguments.is_empty() {
            return Ok(None);
        }

        // require the direct ownership operation to preserve element type
        let source_type = module.adjusted_type_id(iterator.receiver.into_any())?;
        let source_element = Self::element_type(module, source_type)?;
        let result_type = module.adjusted_type_id(expression.into_any())?;
        let result_element = Self::element_type(module, result_type)?;
        let (Some(source_element), Some(result_element)) = (source_element, result_element) else {
            return Ok(None);
        };
        if !module.dir.types_match(source_element, result_element)? {
            return Ok(None);
        }

        Ok(Some(Self {
            expression,
            source: iterator.receiver,
            method,
            is_optional: iterator.is_optional(),
        }))
    }

    /// Return the common element type of one possibly optional contiguous collection.
    fn element_type(
        module: &DirModule<'_>,
        collection: dir::GlobalTypeId,
    ) -> Result<Option<dir::GlobalTypeId>, ProviderError> {
        let mut selected = None;

        // inspect every defined arm of an optional collection
        for element in module.dir.union_elements(collection)? {
            if matches!(module.dir.get_type(element)?, dir::Type::Undefined) {
                continue;
            }
            let element = module.dir.strip_form(element)?;
            let element = match module.dir.get_type(element)? {
                dir::Type::Slice(slice) => slice.element,
                dir::Type::Application(_) => {
                    let Some(argument) = module.dir.application_argument(element, 0)? else {
                        return Ok(None);
                    };

                    argument
                }
                _ => return Ok(None),
            };

            // require every represented collection to carry the same element
            if let Some(current) = selected
                && !module.dir.types_match(current, element)?
            {
                return Ok(None);
            }
            selected = Some(element);
        }

        Ok(selected)
    }
}

/// Report borrowed contiguous collections copied completely into a new array.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical iterator collection into arrays
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(copy) = CollectionCopy::select(module, expression)? else {
            continue;
        };

        // replace the indirect element copy with direct collection ownership
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("collection is copied through its iterator", span);
        if let Some(suggestion) = suggestion(module, lint, copy)? {
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
    copy: CollectionCopy,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(copy.expression.into_any())?;
    let collection_extent = module.source_extent(copy.source.into_any())?;
    if !extent.contains_span(collection_extent) {
        return Err(ProviderError::internal(
            "iterator collection extent does not contain its collection receiver",
        ));
    }
    if module.has_unretained_comment(extent, &[collection_extent])? {
        return Ok(None);
    }

    // preserve authored grouping around the copied collection expression
    let collection = module.expression_source(copy.source, dir::OperatorPrecedence::Postfix)?;
    let access = if copy.is_optional { "?." } else { "." };
    let patch = Patch::replace(extent, format!("{collection}{access}{}()", copy.method));
    let suggestion = lint.fix("copy the collection directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a cloned iterator collected through toArray.
    #[test]
    fn test_replaces_cloned_to_array() {
        let session = TestSession::dir(
            &ITER_CLONED_COLLECT,
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &immutable Label[]): Label[] {
    return values.iterator().cloned().toArray();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[iter-cloned-collect]: collection is copied through its iterator
 ──▶ main.ds:6:12
  │
4 │
5 │ function copy(values: &immutable Label[]): Label[] {
6 │     return values.iterator().cloned().toArray();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │ }
  │

 = fix: copy the collection directly
--- a/main.ds
+++ b/main.ds

    5│ function copy(values: &immutable Label[]): Label[] {
-   6│     return values.iterator().cloned().toArray();
+   6│     return values.clone();
    7│ }
"#,
        );
        session.assert_fixes(
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &immutable Label[]): Label[] {
    return values.clone();
}
"#,
        );
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

function copy(values: &immutable Label[]): Label[] {
    return values.values().cloned().collect();
}
"#,
        );

        session.assert_fixes(
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &immutable Label[]): Label[] {
    return values.clone();
}
"#,
        );
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

function copy(values: &immutable [Label]): Label[] {
    return values.iterator().cloned().toArray();
}
"#,
        );

        session.assert_fixes(
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &immutable [Label]): Label[] {
    return values.toOwned();
}
"#,
        );
    }

    /// Replace copied slice iteration with direct owned collection creation.
    #[test]
    fn test_replaces_copied_slice() {
        let session = TestSession::dir(
            &ITER_CLONED_COLLECT,
            r#"
function copy(values: &immutable [int32]): int32[] {
    return values.iterator().copied().toArray();
}
"#,
        );

        session.assert_fixes(
            r#"
function copy(values: &immutable [int32]): int32[] {
    return values.toOwned();
}
"#,
        );
    }

    /// Preserve an optional collection receiver during direct copying.
    #[test]
    fn test_replaces_optional_array_copy() {
        let session = TestSession::dir(
            &ITER_CLONED_COLLECT,
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &immutable Label[] | undefined): Label[] | undefined {
    return values?.iterator().cloned().toArray();
}
"#,
        );

        session.assert_fixes(
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &immutable Label[] | undefined): Label[] | undefined {
    return values?.clone();
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

function lengths(values: &immutable Label[]): isize[] {
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

function copy(values: Iterator<&immutable Label>): Label[] {
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

function copy(values: &immutable Label[]): Label[] {
    return values.iterator().cloned(/* retain */).toArray();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[iter-cloned-collect]: collection is copied through its iterator
 ──▶ main.ds:6:12
  │
4 │
5 │ function copy(values: &immutable Label[]): Label[] {
6 │     return values.iterator().cloned(/* retain */).toArray();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │ }
  │
"#,
        );
    }
}
