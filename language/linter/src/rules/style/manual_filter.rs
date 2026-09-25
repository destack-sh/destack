use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Array.filter over manually collecting matching elements.
    pub MANUAL_FILTER {
        id: "manual-filter",
        summary: "Prefer Array.filter over manually collecting matching elements",
        explanation: r#"
Creating an array, conditionally pushing each input element, and returning it spells Array.filter manually.
Instead, you SHOULD return the result of `filter` directly.
"#,
        example: {
            reported: r#"
function positive(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value);
        }
    }
    return result;
}
"#,
            accepted: r#"
function positive(values: int32[]): int32[] {
    return values.filter((value) => value > 0);
}
"#,
        },
        provenance: [Clippy("manual_filter")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report arrays built by conditionally pushing each input value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect consecutive declaration, loop, and return statements
    for block in view.iter_node_ids_of_type::<dir::Block>() {
        let expressions = view.get(block).iter_expressions().collect::<Vec<_>>();
        for sequence in expressions.windows(3) {
            let [declaration, expression, return_] = sequence else {
                continue;
            };
            let Some((_, declarator)) = module.binding_declarator(*declaration) else {
                continue;
            };
            let Some(initializer) = declarator.value else {
                continue;
            };
            if !matches!(
                view.get(initializer),
                dir::Expression::ArrayExpression { elements } if elements.is_empty()
            ) {
                continue;
            }
            let result = module.declaration_symbol(declarator.pattern)?;
            let Some(iteration) = module.for_of(*expression) else {
                continue;
            };
            if iteration.asynchrony != dir::Asynchrony::Sync
                || module.representation_item(iteration.iterator.into_any())?
                    != Some(dir::LanguageItem::Array)
            {
                continue;
            }
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

            // require one conditional push of the exact bound value
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
            let Some(action) = view.get(*then_block).only_expression() else {
                continue;
            };
            let Some(push) = module.member_call(action) else {
                continue;
            };
            if push.is_optional()
                || module.language_member(action)? != Some(dir::LanguageItem::Array.member("push"))
                || module.selected_symbol(push.receiver)? != Some(result)
            {
                continue;
            }
            let [argument] = push.arguments else {
                continue;
            };
            let dir::Argument::Positional { value: pushed } = view.get(*argument) else {
                continue;
            };
            let binding = module.declaration_symbol(*value)?;
            let binding_uses =
                module.binding_uses_within(binding, condition.into_any(), &occurrences);
            if module.selected_symbol(*pushed)? != Some(binding) || binding_uses.may_mutate() {
                continue;
            }

            // require the matching collection return and an independent predicate
            let dir::Expression::Return {
                value: Some(returned),
            } = view.get(*return_)
            else {
                continue;
            };
            if module.selected_symbol(*returned)? != Some(result)
                || !module
                    .binding_uses_within(result, iteration.iterator.into_any(), &occurrences)
                    .is_empty()
                || !module
                    .binding_uses_within(result, condition.into_any(), &occurrences)
                    .is_empty()
                || !module
                    .binding_uses_outside(
                        result,
                        &[
                            declaration.into_any(),
                            expression.into_any(),
                            return_.into_any(),
                        ],
                        &occurrences,
                    )
                    .is_empty()
            {
                continue;
            }

            // replace the complete collection sequence
            let span = module.source_extent(expression.into_any())?;
            let mut diagnostic = lint.diagnostic("loop manually collects matching values", span);
            if let Some(suggestion) = suggestion(
                module,
                lint,
                *declaration,
                *return_,
                *value,
                iteration.iterator,
                condition,
            )? {
                diagnostic = diagnostic.suggestion(suggestion);
            }
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Build one Array.filter return from a manual collection loop.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    declaration: dir::LocalNodeId<dir::Expression>,
    return_: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Pattern>,
    iterator: dir::LocalNodeId<dir::Expression>,
    condition: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let declaration_extent = module.source_extent(declaration.into_any())?;
    let return_extent = module.statement_span(return_)?;
    let extent = Span::new(
        declaration_extent.file,
        declaration_extent.start,
        return_extent.end,
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
    let condition = module.source(condition_extent)?;
    let replacement = format!("return {iterator}.filter(({value}) => {condition});");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("return the filtered array directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a complete conditional push-and-return loop.
    #[test]
    fn test_replaces_manual_filter() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
function positive(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value);
        }
    }
    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-filter]: loop manually collects matching values
 ──▶ main.tspp:3:5
  │
1 │ function positive(values: int32[]): int32[] {
2 │     let result: int32[] = [];
3 │     for (const value of values) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │         if (value > 0) {
  │         ^^^^^^^^^^^^^^^^
5 │             result.push(value);
  │             ^^^^^^^^^^^^^^^^^^^
6 │         }
  │         ^
7 │     }
  │     ^
8 │     return result;
9 │ }
  │

 = suggestion: return the filtered array directly (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function positive(values: int32[]): int32[] {
-   2│     let result: int32[] = [];
-   3│     for (const value of values) {
-   4│         if (value > 0) {
-   5│             result.push(value);
-   6│         }
-   7│     }
-   8│     return result;
+   2│     return values.filter((value) => value > 0);
    9│ }
"#,
        );
        session.assert_suggestions(
            r#"
function positive(values: int32[]): int32[] {
    return values.filter((value) => value > 0);
}
"#,
        );
    }

    /// Accept pushing a transformed element.
    #[test]
    fn test_accepts_transformed_push() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
function positive(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value + 1);
        }
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a predicate that changes its iteration binding before collecting it.
    #[test]
    fn test_accepts_mutated_value() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
declare function increment(value: &int32): int32;

function positive(values: int32[]): int32[] {
    let result: int32[] = [];
    for (let value of values) {
        if (increment(&value) > 0) {
            result.push(value);
        }
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a predicate that awaits in its enclosing async function.
    #[test]
    fn test_accepts_awaited_predicate() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
async function isPositive(value: int32): Promise<boolean> {
    return value > 0;
}

async function positive(values: int32[]): Promise<int32[]> {
    let result: int32[] = [];
    for (const value of values) {
        if (await isPositive(value)) {
            result.push(value);
        }
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a conditional push with an else branch.
    #[test]
    fn test_accepts_else_branch() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
function partition(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value);
        } else {
            result.push(0);
        }
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept returning a different array.
    #[test]
    fn test_accepts_different_return() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
function positive(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value);
        }
    }
    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an iterable expression that depends on the collection binding.
    #[test]
    fn test_accepts_result_used_by_iterator() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
declare function select(result: int32[], values: int32[]): int32[];

function positive(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of select(result, values)) {
        if (value > 0) {
            result.push(value);
        }
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a collection binding referenced after the replaced sequence.
    #[test]
    fn test_accepts_result_used_after_return() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
function positive(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value);
        }
    }
    return result;
    result.push(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept manual filtering over a non-array iterable.
    #[test]
    fn test_accepts_other_iterable() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
function positive(values: Set<int32>): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value);
        }
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
