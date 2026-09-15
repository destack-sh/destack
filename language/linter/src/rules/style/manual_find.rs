use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer find over a loop returning the first matching element.
    pub MANUAL_FIND {
        id: "manual-find",
        summary: "Prefer find over a loop returning the first matching element",
        explanation: r#"
A loop that returns its first match and otherwise returns undefined implements `find` manually.
Instead, you SHOULD return the result of `find` directly.
"#,
        example: {
            reported: r#"
function firstPositive(values: int32[]): int32 | undefined {
    for (const value of values) {
        if (value > 0) {
            return value;
        }
    }
    return undefined;
}
"#,
            accepted: r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.find((value) => value > 0);
}
"#,
        },
        provenance: [Clippy("manual_find")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report iterable loops returning their first matching value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect loops immediately followed by an undefined return
    for block in view.iter_node_ids_of_type::<dir::Block>() {
        let expressions = view.get(block).iter_expressions().collect::<Vec<_>>();
        for sequence in expressions.windows(2) {
            let [expression, undefined_return] = sequence else {
                continue;
            };
            let dir::Expression::Return {
                value: Some(undefined),
            } = view.get(*undefined_return)
            else {
                continue;
            };
            if module.scalar_constant(*undefined)? != Some(dir::Literal::Undefined) {
                continue;
            }
            let Some(iteration) = module.for_of(*expression) else {
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
            if !matches!(
                view.get(*value),
                dir::Pattern::Binding { pattern: None, .. }
            ) {
                continue;
            }

            // require one conditional return of the exact bound value
            let Some(conditional) = view.get(iteration.body).only_expression() else {
                continue;
            };
            let dir::Expression::If {
                condition,
                then_expression,
                else_expression: None,
                ..
            } = view.get(conditional)
            else {
                continue;
            };
            let Some(condition) = condition.as_expression() else {
                continue;
            };
            if module.uses_enclosing_control(condition.into_any())? {
                continue;
            }
            let dir::Expression::Block(then_block) = view.get(*then_expression) else {
                continue;
            };
            let Some(return_) = view.get(*then_block).only_expression() else {
                continue;
            };
            let dir::Expression::Return {
                value: Some(returned),
            } = view.get(return_)
            else {
                continue;
            };
            let binding = module.declaration_symbol(*value)?;
            let binding_uses =
                module.binding_uses_within(binding, condition.into_any(), &occurrences);
            if module.selected_symbol(*returned)? != Some(binding) || binding_uses.may_mutate() {
                continue;
            }

            // replace the loop and undefined return
            let span = module.source_extent(expression.into_any())?;
            let mut diagnostic =
                lint.diagnostic("loop manually finds its first matching value", span);
            if let Some(suggestion) = suggestion(
                module,
                lint,
                *expression,
                *undefined_return,
                *value,
                iteration.iterator,
                condition,
                iterator_item,
            )? {
                diagnostic = diagnostic.suggestion(suggestion);
            }
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Build one find return from a first-match loop.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    iteration: dir::LocalNodeId<dir::Expression>,
    undefined_return: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Pattern>,
    iterator: dir::LocalNodeId<dir::Expression>,
    condition: dir::LocalNodeId<dir::Expression>,
    iterator_item: Option<dir::LanguageItem>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let iteration_extent = module.source_extent(iteration.into_any())?;
    let undefined_return_extent = module.statement_span(undefined_return)?;
    let extent = Span::new(
        iteration_extent.file,
        iteration_extent.start,
        undefined_return_extent.end,
    );
    let value_extent = module.source_extent(value.into_any())?;
    let iterator_extent = module.source_extent(iterator.into_any())?;
    let condition_extent = module.source_extent(condition.into_any())?;
    if module.has_unretained_comment(extent, &[value_extent, iterator_extent, condition_extent])? {
        return Ok(None);
    }

    // preserve the authored binding, iterator, and predicate
    let value = module.source(value_extent)?;
    let iterator = module.expression_source(iterator, dir::OperatorPrecedence::Postfix)?;
    let iterator = match iterator_item {
        Some(dir::LanguageItem::Array | dir::LanguageItem::Iterator) => iterator.into_owned(),
        _ => format!("{iterator}.iterator()"),
    };
    let condition = module.source(condition_extent)?;
    let replacement = format!("return {iterator}.find(({value}) => {condition});");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("return the first matching value directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a complete first-match loop.
    #[test]
    fn test_replaces_manual_find() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    for (const value of values) {
        if (value > 0) {
            return value;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-find]: loop manually finds its first matching value
 ──▶ main.ds:2:5
  │
1 │ function firstPositive(values: int32[]): int32 | undefined {
2 │     for (const value of values) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         if (value > 0) {
  │         ^^^^^^^^^^^^^^^^
4 │             return value;
  │             ^^^^^^^^^^^^^
5 │         }
  │         ^
6 │     }
  │     ^
7 │     return undefined;
8 │ }
  │

 = suggestion: return the first matching value directly (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function firstPositive(values: int32[]): int32 | undefined {
-   2│     for (const value of values) {
-   3│         if (value > 0) {
-   4│             return value;
-   5│         }
-   6│     }
-   7│     return undefined;
+   2│     return values.find((value) => value > 0);
    8│ }
"#,
        );
        session.assert_suggestions(
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.find((value) => value > 0);
}
"#,
        );
    }

    /// Accept returning a transformed matching value.
    #[test]
    fn test_accepts_transformed_return() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    for (const value of values) {
        if (value > 0) {
            return value + 1;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a predicate that changes its iteration binding before returning it.
    #[test]
    fn test_accepts_mutated_value() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
declare function increment(value: &int32): int32;

function firstPositive(values: int32[]): int32 | undefined {
    for (let value of values) {
        if (increment(&value) > 0) {
            return value;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a predicate that awaits in its enclosing async function.
    #[test]
    fn test_accepts_awaited_predicate() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
async function isPositive(value: int32): Promise<boolean> {
    return value > 0;
}

async function firstPositive(values: int32[]): Promise<int32 | undefined> {
    for (const value of values) {
        if (await isPositive(value)) {
            return value;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a different trailing return value.
    #[test]
    fn test_accepts_other_return() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
function firstPositive(values: int32[]): int32 {
    for (const value of values) {
        if (value > 0) {
            return value;
        }
    }
    return 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop with an additional body action.
    #[test]
    fn test_accepts_additional_action() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
function firstPositive(values: int32[], seen: int32[]): int32 | undefined {
    for (const value of values) {
        seen.push(value);
        if (value > 0) {
            return value;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace manual searching over another iterable.
    #[test]
    fn test_replaces_other_iterable() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
function firstPositive(values: Set<int32>): int32 | undefined {
    for (const value of values) {
        if (value > 0) {
            return value;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_suggestions(
            r#"
function firstPositive(values: Set<int32>): int32 | undefined {
    return values.iterator().find((value) => value > 0);
}
"#,
        );
    }

    /// Replace manual searching over an iterator directly.
    #[test]
    fn test_replaces_iterator() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
import { Iterator } from "destack:iter";

function firstPositive(values: Iterator<int32>): int32 | undefined {
    for (const value of values) {
        if (value > 0) {
            return value;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Iterator } from "destack:iter";

function firstPositive(values: Iterator<int32>): int32 | undefined {
    return values.find((value) => value > 0);
}
"#,
        );
    }

    /// Replace a trailing undefined constant while preserving its value.
    #[test]
    fn test_replaces_undefined_constant() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
const absent: undefined = undefined;

function firstPositive(values: int32[]): int32 | undefined {
    for (const value of values) {
        if (value > 0) {
            return value;
        }
    }
    return absent;
}
"#,
        );

        session.assert_suggestions(
            r#"
const absent: undefined = undefined;

function firstPositive(values: int32[]): int32 | undefined {
    return values.find((value) => value > 0);
}
"#,
        );
    }

    /// Preserve comments in a reported first-match loop by omitting the suggestion.
    #[test]
    fn test_reports_commented_loop_without_suggestion() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    for (const value of values) {
        // retain the reason for this threshold
        if (value > 0) {
            return value;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-find]: loop manually finds its first matching value
 ──▶ main.ds:2:5
  │
1 │ function firstPositive(values: int32[]): int32 | undefined {
2 │     for (const value of values) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         // retain the reason for this threshold
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │         if (value > 0) {
  │         ^^^^^^^^^^^^^^^^
5 │             return value;
  │             ^^^^^^^^^^^^^
6 │         }
  │         ^
7 │     }
  │     ^
8 │     return undefined;
9 │ }
  │
"#,
        );
    }
}
