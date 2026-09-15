use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch};

use crate::rules::declare_lint;
use crate::{DirModule, IntegerStep, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer indexed iteration over manually counting values.
    pub PREFER_ENUMERATE {
        id: "prefer-enumerate",
        summary: "Prefer indexed iteration over manually counting values",
        explanation: r#"
Initializing an index before iteration and incrementing it after every value maintains information already provided by `entries` or `enumerate`.
Instead, you SHOULD bind each index and value from the indexed iterator.
"#,
        example: {
            reported: r#"
function indexes(values: int32[], output: isize[]): void {
    let index: isize = 0;
    for (const value of values) {
        output.push(index);
        index++;
    }
}
"#,
            accepted: r#"
function indexes(values: int32[], output: isize[]): void {
    for (const (index, value) of values.entries()) {
        output.push(index);
    }
}
"#,
        },
        provenance: [Clippy("explicit_counter_loop")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report iteration maintaining a separate zero-based index.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect synchronous for-of loops with a declared pattern
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(iteration) = module.for_of(expression) else {
            continue;
        };
        if iteration.asynchrony != dir::Asynchrony::Sync {
            continue;
        }
        let iterator_item = module.representation_item(iteration.iterator.into_any())?;
        let dir::ForEachBinding::Pattern {
            pattern: value,
            keyword: Some(_),
        } = iteration.binding
        else {
            continue;
        };

        // require an immediately preceding mutable binding initialized to zero
        let Some(block) = view.ancestor::<dir::Block>(expression.into_any()) else {
            continue;
        };
        let expressions = view.get(block).iter_expressions().collect::<Vec<_>>();
        let Some(position) = expressions
            .iter()
            .position(|candidate| *candidate == expression)
        else {
            continue;
        };
        let Some(declaration) = position.checked_sub(1).map(|index| expressions[index]) else {
            continue;
        };
        let Some((dir::Mutability::Mutable, declarator)) = module.binding_declarator(declaration)
        else {
            continue;
        };
        let Some(initializer) = declarator.value else {
            continue;
        };
        if module.integral_constant(initializer)? != Some(0) {
            continue;
        }
        let counter = module.declaration_symbol(declarator.pattern)?;

        // require one unconditional trailing increment of the same counter
        let Some(increment) = view.get(iteration.body).last_expression() else {
            continue;
        };
        let Some(IntegerStep::Increment(increment_target)) = module.integer_update(increment)?
        else {
            continue;
        };
        if module.selected_symbol(increment_target)? != Some(counter) {
            continue;
        }
        if module.has_reachable_continue(expression)? {
            continue;
        }

        // require useful reads and reject every other mutation or capture
        let uses = occurrences
            .iter()
            .filter(|occurrence| occurrence.symbol == counter)
            .collect::<Vec<_>>();
        let is_used_in_body = uses.iter().any(|occurrence| {
            occurrence.uses.contains(dir::BindingUse::READ)
                && view.is_inside(occurrence.node, iteration.body.into_any())
                && !view.is_inside(occurrence.node, increment.into_any())
        });
        let has_other_mutation = uses.iter().any(|occurrence| {
            let is_mutation = occurrence.uses.may_mutate();
            is_mutation && !view.is_inside(occurrence.node, increment.into_any())
        });
        let is_captured = uses
            .iter()
            .any(|occurrence| occurrence.uses.contains(dir::BindingUse::CAPTURE));
        let has_header_use = uses.iter().any(|occurrence| {
            view.is_inside(occurrence.node, expression.into_any())
                && !view.is_inside(occurrence.node, iteration.body.into_any())
        });
        if !is_used_in_body || has_other_mutation || is_captured || has_header_use {
            continue;
        }

        // report the separate index declaration
        let span = module.source_extent(declaration.into_any())?;
        let mut diagnostic = lint.diagnostic("iteration maintains its index manually", span);
        let is_used_after = uses
            .iter()
            .any(|occurrence| !view.is_inside(occurrence.node, expression.into_any()));
        let has_entries_index_type = matches!(
            module.primitive_type(declarator.pattern.into_any())?,
            Some(dir::PrimitiveType::Integer(dir::IntegerType::Pointer {
                is_signed: true
            }))
        );
        if !is_used_after
            && has_entries_index_type
            && let Some(suggestion) = suggestion(
                module,
                lint,
                declaration,
                declarator.pattern,
                *value,
                iteration.iterator,
                increment,
                iterator_item,
            )?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one separate index with an entries binding.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    declaration: dir::LocalNodeId<dir::Expression>,
    counter: dir::LocalNodeId<dir::Pattern>,
    value: dir::LocalNodeId<dir::Pattern>,
    iterator: dir::LocalNodeId<dir::Expression>,
    increment: dir::LocalNodeId<dir::Expression>,
    iterator_item: Option<dir::LanguageItem>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let declaration_extent = module.source_extent(declaration.into_any())?;
    let counter_extent = module.source_extent(counter.into_any())?;
    let value_extent = module.source_extent(value.into_any())?;
    let iterator_extent = module.source_extent(iterator.into_any())?;
    let increment_extent = module.source_extent(increment.into_any())?;
    if module.has_unretained_comment(declaration_extent, &[counter_extent])?
        || module.has_unretained_comment(increment_extent, &[])?
    {
        return Ok(None);
    }

    // preserve the authored names and iterator expression
    let counter = module.source(counter_extent)?;
    let value = module.source(value_extent)?;
    let iterator = module.expression_source(iterator, dir::OperatorPrecedence::Postfix)?;
    let iterator = match iterator_item {
        Some(dir::LanguageItem::Array) => format!("{iterator}.entries()"),
        Some(dir::LanguageItem::Iterator) => format!("{iterator}.enumerate()"),
        _ => format!("{iterator}.iterator().enumerate()"),
    };
    let mut patch = FilePatch::new(declaration_extent.file);
    let declaration = module.statement_span(declaration)?;
    patch.delete(module.line_removal_span(declaration)?);
    patch.replace(value_extent, format!("({counter}, {value})"));
    patch.replace(iterator_extent, iterator);
    let increment = module.statement_span(increment)?;
    patch.delete(module.line_removal_span(increment)?);
    patch.sort();
    let suggestion = lint.suggestion("use the indexed iterator", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;
    /// Replace a separately counted array loop.
    #[test]
    fn test_replaces_manual_index() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function indexes(values: int32[], output: isize[]): void {
    let index: isize = 0;
    for (const value of values) {
        output.push(index);
        index++;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-enumerate]: iteration maintains its index manually
 ──▶ main.ds:2:5
  │
1 │ function indexes(values: int32[], output: isize[]): void {
2 │     let index: isize = 0;
  │     ^^^^^^^^^^^^^^^^^^^^
3 │     for (const value of values) {
4 │         output.push(index);
  │

 = suggestion: use the indexed iterator (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function indexes(values: int32[], output: isize[]): void {
-   2│     let index: isize = 0;
-   3│     for (const value of values) {
+   2│     for (const (index, value) of values.entries()) {
    4│         output.push(index);
-   5│         index++;
    6│     }
"#,
        );
        session.assert_suggestions(
            r#"
function indexes(values: int32[], output: isize[]): void {
    for (const (index, value) of values.entries()) {
        output.push(index);
    }
}
"#,
        );
    }

    /// Replace a separate counter using additive assignment.
    #[test]
    fn test_replaces_additive_counter() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function indexes(values: int32[], output: isize[]): void {
    let index: isize = 0;
    for (const value of values) {
        output.push(index);
        index += 1;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function indexes(values: int32[], output: isize[]): void {
    for (const (index, value) of values.entries()) {
        output.push(index);
    }
}
"#,
        );
    }

    /// Replace a separate counter using an explicit reversed sum assignment.
    #[test]
    fn test_replaces_assigned_counter() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function indexes(values: int32[], output: isize[]): void {
    let index: isize = 0;
    for (const value of values) {
        output.push(index);
        index = 1 + index;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function indexes(values: int32[], output: isize[]): void {
    for (const (index, value) of values.entries()) {
        output.push(index);
    }
}
"#,
        );
    }

    /// Preserve a destructured value pattern in the indexed binding.
    #[test]
    fn test_replaces_destructured_value() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function indexes(values: { value: int32 }[], output: isize[]): void {
    let index: isize = 0;
    for (const { value } of values) {
        output.push(index);
        value;
        index++;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function indexes(values: { value: int32 }[], output: isize[]): void {
    for (const (index, { value }) of values.entries()) {
        output.push(index);
        value;
    }
}
"#,
        );
    }

    /// Report a counter used after the loop while withholding a rewrite.
    #[test]
    fn test_reports_counter_used_after_loop_without_suggestion() {
        let source = r#"
function count(values: int32[]): isize {
    let index: isize = 0;
    for (const value of values) {
        value;
        index;
        index++;
    }
    return index;
}
"#;
        let session = TestSession::dir(&PREFER_ENUMERATE, source);

        session.assert_diagnostics(
            r#"
warning[prefer-enumerate]: iteration maintains its index manually
 ──▶ main.ds:2:5
  │
1 │ function count(values: int32[]): isize {
2 │     let index: isize = 0;
  │     ^^^^^^^^^^^^^^^^^^^^
3 │     for (const value of values) {
4 │         value;
  │
"#,
        );
    }

    /// Report a differently typed counter without changing its type.
    #[test]
    fn test_reports_int32_counter_without_suggestion() {
        let source = r#"
function indexes(values: int32[], output: int32[]): void {
    let index: int32 = 0;
    for (const value of values) {
        output.push(index);
        value;
        index++;
    }
}
"#;
        let session = TestSession::dir(&PREFER_ENUMERATE, source);

        session.assert_diagnostics(
            r#"
warning[prefer-enumerate]: iteration maintains its index manually
 ──▶ main.ds:2:5
  │
1 │ function indexes(values: int32[], output: int32[]): void {
2 │     let index: int32 = 0;
  │     ^^^^^^^^^^^^^^^^^^^^
3 │     for (const value of values) {
4 │         output.push(index);
  │
"#,
        );
    }

    /// Preserve the label on a rewritten loop.
    #[test]
    fn test_preserves_labeled_loop() {
        let source = r#"
function indexes(values: int32[], output: isize[]): void {
    let index: isize = 0;
    outer: for (const value of values) {
        output.push(index);
        if (value < 0) {
            break outer;
        }
        index++;
    }
}
"#;
        let session = TestSession::dir(&PREFER_ENUMERATE, source);

        session.assert_suggestions(
            r#"
function indexes(values: int32[], output: isize[]): void {
    outer: for (const (index, value) of values.entries()) {
        output.push(index);
        if (value < 0) {
            break outer;
        }
    }
}
"#,
        );
    }

    /// Accept a counter that is not used during iteration.
    #[test]
    fn test_accepts_unused_counter() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function count(values: int32[]): isize {
    let index: isize = 0;
    for (const value of values) {
        value;
        index++;
    }
    return index;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace manual counting over another iterable.
    #[test]
    fn test_replaces_other_iterable() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function count(values: Set<int32>, output: isize[]): void {
    let index: isize = 0;
    for (const value of values) {
        output.push(index);
        value;
        index++;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function count(values: Set<int32>, output: isize[]): void {
    for (const (index, value) of values.iterator().enumerate()) {
        output.push(index);
        value;
    }
}
"#,
        );
    }

    /// Replace manual counting over an iterator directly.
    #[test]
    fn test_replaces_iterator() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
import { Iterator } from "destack:iter";

function count(values: Iterator<int32>, output: isize[]): void {
    let index: isize = 0;
    for (const value of values) {
        output.push(index);
        value;
        index++;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Iterator } from "destack:iter";

function count(values: Iterator<int32>, output: isize[]): void {
    for (const (index, value) of values.enumerate()) {
        output.push(index);
        value;
    }
}
"#,
        );
    }

    /// Accept a counter changed before its trailing increment.
    #[test]
    fn test_accepts_counter_mutation() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function indexes(values: int32[], output: isize[]): void {
    let index: isize = 0;
    for (const value of values) {
        index += 1;
        output.push(index);
        value;
        index++;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a counter whose increment can be skipped.
    #[test]
    fn test_accepts_skipped_increment() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function indexes(values: int32[], output: isize[]): void {
    let index: isize = 0;
    for (const value of values) {
        output.push(index);
        if (value < 0) {
            continue;
        }
        index++;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a counter captured by a nested lambda.
    #[test]
    fn test_accepts_captured_counter() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function callbacks(values: int32[]): (() => isize)[] {
    let output: (() => isize)[] = [];
    let index: isize = 0;
    for (const value of values) {
        value;
        output.push(() => index);
        index++;
    }
    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
