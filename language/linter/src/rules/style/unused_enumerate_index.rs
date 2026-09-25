use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow indexed iteration when its index is unused.
    pub UNUSED_ENUMERATE_INDEX {
        id: "unused-enumerate-index",
        summary: "Disallow indexed iteration when its index is unused",
        explanation: r#"
Indexed iteration constructs an index-value pair for every element even when the index is unused.
Instead, you SHOULD iterate over the values directly.
"#,
        example: {
            reported: r#"
function copy(values: int32[], output: int32[]): void {
    for (const (_, value) of values.entries()) {
        output.push(value);
    }
}
"#,
            accepted: r#"
function copy(values: int32[], output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        },
        provenance: [Clippy("unused_enumerate_index")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report indexed iteration whose index binding is unused.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect for-of loops over canonical entries or enumerate calls
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(iteration) = module.for_of(expression) else {
            continue;
        };
        if iteration.asynchrony != dir::Asynchrony::Sync {
            continue;
        }
        let Some(enumeration) = module.member_call(iteration.iterator) else {
            continue;
        };
        let member = module.language_member(iteration.iterator)?;
        if enumeration.is_optional()
            || !enumeration.arguments.is_empty()
            || member != Some(dir::LanguageItem::Array.member("entries"))
                && member != Some(dir::LanguageItem::Iterator.member("enumerate"))
        {
            continue;
        }
        let dir::ForEachBinding::Pattern { pattern, .. } = iteration.binding else {
            continue;
        };
        let dir::Pattern::Tuple { fields } = view.get(*pattern) else {
            continue;
        };
        let [index_field, value_field] = fields.as_slice() else {
            continue;
        };
        let dir::PatternField::Positional { pattern: index } = view.get(*index_field) else {
            continue;
        };
        let dir::PatternField::Positional { pattern: value } = view.get(*value_field) else {
            continue;
        };

        // require a discarded or entirely unused index binding
        let is_unused = matches!(
            view.get(*index),
            dir::Pattern::Wildcard | dir::Pattern::Binding { pattern: None, .. }
        ) && module
            .declared_binding_uses(index.into_any(), &occurrences)
            .is_empty();
        if !is_unused {
            continue;
        }

        // replace the tuple binding and indexed call
        let span = module.span(index.into_any())?;
        let mut diagnostic = lint.diagnostic("indexed iteration index is unused", span);
        if let Some(suggestion) = suggestion(
            module,
            lint,
            *pattern,
            *value,
            iteration.iterator,
            enumeration.receiver,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Remove one unused index from a for-of loop.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    tuple: dir::LocalNodeId<dir::Pattern>,
    value: dir::LocalNodeId<dir::Pattern>,
    call: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // remove a nested explicit iterator call with the indexed adapter
    let receiver = module.iterator_receiver(receiver)?.unwrap_or(receiver);

    // retain every source fragment copied into the replacement
    let tuple_extent = module.source_extent(tuple.into_any())?;
    let value_extent = module.source_extent(value.into_any())?;
    let call_extent = module.source_extent(call.into_any())?;
    let receiver_extent = module.source_extent(receiver.into_any())?;
    if module.has_unretained_comment(tuple_extent, &[value_extent])?
        || module.has_unretained_comment(call_extent, &[receiver_extent])?
    {
        return Ok(None);
    }

    // preserve the authored value pattern and iteration receiver
    let value = module.source(value_extent)?;
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Lowest)?;
    let mut patch = FilePatch::new(tuple_extent.file);
    patch.replace(tuple_extent, value);
    patch.replace(call_extent, receiver);
    patch.sort();
    let suggestion = lint.fix("iterate over values directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove entries when a wildcard discards the index.
    #[test]
    fn test_removes_wildcard_index() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
function copy(values: int32[], output: int32[]): void {
    for (const (_, value) of values.entries()) {
        output.push(value);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[unused-enumerate-index]: indexed iteration index is unused
 ──▶ main.tspp:2:17
  │
1 │ function copy(values: int32[], output: int32[]): void {
2 │     for (const (_, value) of values.entries()) {
  │                 ^
3 │         output.push(value);
4 │     }
  │

 = fix: iterate over values directly
--- a/main.tspp
+++ b/main.tspp

    1│ function copy(values: int32[], output: int32[]): void {
-   2│     for (const (_, value) of values.entries()) {
+   2│     for (const value of values) {
    3│         output.push(value);
"#,
        );
        session.assert_fixes(
            r#"
function copy(values: int32[], output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        );
    }

    /// Remove entries when a named index has no references.
    #[test]
    fn test_removes_unused_named_index() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
function copy(values: int32[], output: int32[]): void {
    for (const (index, value) of values.entries()) {
        output.push(value);
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function copy(values: int32[], output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        );
    }

    /// Remove enumerate when its iterator index is discarded.
    #[test]
    fn test_removes_unused_iterator_index() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
import { Iterator } from "tspp:iter";

function copy(values: Iterator<int32>, output: int32[]): void {
    for (const (_, value) of values.enumerate()) {
        output.push(value);
    }
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "tspp:iter";

function copy(values: Iterator<int32>, output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        );
    }

    /// Remove both enumerate and an explicit iterator call when the index is unused.
    #[test]
    fn test_removes_unused_index_from_explicit_iterator() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
function copy(values: Set<int32>, output: int32[]): void {
    for (const (_, value) of values.iterator().enumerate()) {
        output.push(value);
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function copy(values: Set<int32>, output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        );
    }

    /// Accept entries when the index is referenced.
    #[test]
    fn test_accepts_used_index() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
function copy(values: int32[], indexes: isize[], output: int32[]): void {
    for (const (index, value) of values.entries()) {
        indexes.push(index);
        output.push(value);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept async iteration because removing entries would await each array value.
    #[test]
    fn test_accepts_async_iteration() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
async function copy(values: int32[], output: int32[]): Promise<void> {
    for await (const (_, value) of values.entries()) {
        output.push(value);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined entries method.
    #[test]
    fn test_accepts_user_entries() {
        let session = TestSession::dir(
            &UNUSED_ENUMERATE_INDEX,
            r#"
class Values {
    entries(): (usize, int32)[] {
        return [];
    }
}

function visit(values: Values): void {
    for (const (_, value) of values.entries()) {
        // intentionally empty
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
