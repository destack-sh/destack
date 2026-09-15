use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow peekable iterators that are never peeked.
    pub UNUSED_PEEKABLE {
        id: "unused-peekable",
        summary: "Disallow peekable iterators that are never peeked",
        explanation: r#"
Wrapping an iterator with `peekable` adds buffered state when no use observes the next value.
Instead, you SHOULD use the original iterator until lookahead is required.
"#,
        example: {
            reported: r#"
function consume(values: int32[]): void {
    let iterator = values.iterator().peekable();
    iterator.next();
}
"#,
            accepted: r#"
function consume(values: int32[]): void {
    let iterator = values.iterator();
    iterator.next();
}
"#,
        },
        provenance: [Clippy("unused_peekable")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report directly bound peekable adapters whose binding never calls peek.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect direct inferred local bindings
    for (expression, _) in view.iter_nodes::<dir::Expression>() {
        let Some((_, declarator)) = module.binding_declarator(expression) else {
            continue;
        };
        if declarator.ty.is_some() {
            continue;
        }
        let Some(initializer) = declarator.value else {
            continue;
        };
        let Some(call) = module.member_call(initializer) else {
            continue;
        };
        if call.is_optional()
            || !call.arguments.is_empty()
            || module.language_member(initializer)?
                != Some(dir::LanguageItem::Iterator.member("peekable"))
        {
            continue;
        }

        // require every binding use to remain direct iteration or an Iterator method call
        let symbol = module.declaration_symbol(declarator.pattern)?;
        let mut can_remove = true;
        for occurrence in occurrences
            .iter()
            .filter(|occurrence| occurrence.symbol == symbol)
        {
            if is_iterated_directly(module, occurrence.node) {
                continue;
            }
            let Some((call, member)) = select_direct_member_call(module, occurrence.node) else {
                can_remove = false;
                break;
            };
            if member == "peek"
                || module
                    .language_member(call)?
                    .is_none_or(|member| member.owner != dir::LanguageItem::Iterator)
            {
                can_remove = false;
                break;
            }
        }
        if !can_remove {
            continue;
        }

        // remove the unused adapter suffix
        let span = module.source_extent(initializer.into_any())?;
        let mut diagnostic = lint.diagnostic("peekable iterator is never peeked", span);
        if let Some(suggestion) = suggestion(module, lint, initializer, call.receiver)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one binding occurrence is the iterator of a for-of expression.
fn is_iterated_directly(module: &DirModule<'_>, node: dir::LocalNodeIdAny) -> bool {
    let Ok(iterator) = node.try_into_typed::<dir::Expression>() else {
        return false;
    };
    let Some(parent) = module.view().get_parent_for(iterator) else {
        return false;
    };
    let Ok(expression) = parent.try_into_typed::<dir::Expression>() else {
        return false;
    };

    module
        .for_of(expression)
        .is_some_and(|for_of| for_of.iterator == iterator)
}

/// Select the direct member call for one binding occurrence.
fn select_direct_member_call<'a>(
    module: &'a DirModule<'_>,
    node: dir::LocalNodeIdAny,
) -> Option<(dir::LocalNodeId<dir::Expression>, &'a str)> {
    let view = module.view();
    let expression = node.try_into_typed::<dir::Expression>().ok()?;
    let parent = view.get_parent_for(expression)?;
    let member = parent.try_into_typed::<dir::Expression>().ok()?;
    let dir::Expression::Member {
        left,
        name: Some(name),
        is_optional: false,
    } = view.get(member)
    else {
        return None;
    };
    if *left != expression {
        return None;
    }

    // require the member value to be called directly
    let call = view.get_parent_for(member)?;
    let call = call.try_into_typed::<dir::Expression>().ok()?;
    let dir::Expression::Call {
        left,
        is_optional: false,
        ..
    } = view.get(call)
    else {
        return None;
    };
    if *left != member {
        return None;
    }

    Some((call, module.dir.strings.get(*name)))
}

/// Remove one unused peekable adapter suffix.
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
            "peekable adapter extent does not contain its receiver",
        ));
    }
    if module.has_unretained_comment(extent, &[receiver])? {
        return Ok(None);
    }

    // retain the complete iterator and remove the adapter call
    let suffix = Span::new(extent.file, receiver.end, extent.end);
    let mut file = FilePatch::new(extent.file);
    file.delete(suffix);
    let suggestion = lint.fix("use the original iterator", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove a peekable adapter when only next is called.
    #[test]
    fn test_removes_unobserved_peekable() {
        let session = TestSession::dir(
            &UNUSED_PEEKABLE,
            r#"
function consume(values: int32[]): void {
    let iterator = values.iterator().peekable();
    iterator.next();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[unused-peekable]: peekable iterator is never peeked
 ──▶ main.ds:2:20
  │
1 │ function consume(values: int32[]): void {
2 │     let iterator = values.iterator().peekable();
  │                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │     iterator.next();
4 │ }
  │

 = fix: use the original iterator
--- a/main.ds
+++ b/main.ds

    1│ function consume(values: int32[]): void {
-   2│     let iterator = values.iterator().peekable();
+   2│     let iterator = values.iterator();
    3│     iterator.next();
"#,
        );
        session.assert_fixes(
            r#"
function consume(values: int32[]): void {
    let iterator = values.iterator();
    iterator.next();
}
"#,
        );
    }

    /// Remove peekable before several ordinary Iterator operations.
    #[test]
    fn test_removes_multiple_iterator_uses() {
        let session = TestSession::dir(
            &UNUSED_PEEKABLE,
            r#"
function consume(values: int32[]): isize {
    let iterator = values.iterator().peekable();
    iterator.next();

    return iterator.count();
}
"#,
        );

        session.assert_fixes(
            r#"
function consume(values: int32[]): isize {
    let iterator = values.iterator();
    iterator.next();

    return iterator.count();
}
"#,
        );
    }

    /// Remove peekable from an iterator consumed by for-of.
    #[test]
    fn test_removes_iterated_peekable() {
        let session = TestSession::dir(
            &UNUSED_PEEKABLE,
            r#"
function consume(values: int32[]): void {
    let iterator = values.iterator().peekable();
    for (const value of iterator) {
        value;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function consume(values: int32[]): void {
    let iterator = values.iterator();
    for (const value of iterator) {
        value;
    }
}
"#,
        );
    }

    /// Accept a peekable iterator whose next value is observed.
    #[test]
    fn test_accepts_peek() {
        let session = TestSession::dir(
            &UNUSED_PEEKABLE,
            r#"
function inspect(values: int32[]): void {
    let iterator = values.iterator().peekable();
    iterator.peek();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a peekable iterator passed to another callable.
    #[test]
    fn test_accepts_escaped_iterator() {
        let session = TestSession::dir(
            &UNUSED_PEEKABLE,
            r#"
declare function consume(value: unknown): void;

function forward(values: int32[]): void {
    let iterator = values.iterator().peekable();
    consume(iterator);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an explicitly typed peekable binding whose type would change.
    #[test]
    fn test_accepts_explicit_type() {
        let session = TestSession::dir(
            &UNUSED_PEEKABLE,
            r#"
import { Iterator, PeekableIterator } from "destack:iter";

function consume(values: Iterator<int32>): void {
    let iterator: PeekableIterator<Iterator<int32>, int32> = values.peekable();
    iterator.next();
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
