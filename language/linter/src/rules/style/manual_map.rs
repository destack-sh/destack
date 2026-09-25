use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Array.map over manually collecting transformed elements.
    pub MANUAL_MAP {
        id: "manual-map",
        summary: "Prefer Array.map over manually collecting transformed elements",
        explanation: r#"
Creating an array, pushing one transformed value for every input element, and returning it spells Array.map manually.
Instead, you SHOULD return the result of `map` directly.
"#,
        example: {
            reported: r#"
function doubled(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        result.push(value * 2);
    }
    return result;
}
"#,
            accepted: r#"
function doubled(values: int32[]): int32[] {
    return values.map((value) => value * 2);
}
"#,
        },
        provenance: [Clippy("manual_map")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report arrays built by pushing one mapped value per input element.
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

            // require one canonical push and return of the collection binding
            let Some(action) = view.get(iteration.body).only_expression() else {
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
            let dir::Argument::Positional { value: mapped } = view.get(*argument) else {
                continue;
            };
            let mutates_binding = module
                .symbols_declared_within(value.into_any())
                .any(|binding| {
                    module
                        .binding_uses_within(binding, mapped.into_any(), &occurrences)
                        .may_mutate()
                });
            if mutates_binding || module.uses_enclosing_control(mapped.into_any())? {
                continue;
            }
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
                    .binding_uses_within(result, mapped.into_any(), &occurrences)
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
            let mut diagnostic = lint.diagnostic("loop manually collects mapped values", span);
            if let Some(suggestion) = suggestion(
                module,
                lint,
                *declaration,
                *return_,
                *value,
                iteration.iterator,
                *mapped,
            )? {
                diagnostic = diagnostic.suggestion(suggestion);
            }
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Build one Array.map return from a manual collection loop.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    declaration: dir::LocalNodeId<dir::Expression>,
    return_: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Pattern>,
    iterator: dir::LocalNodeId<dir::Expression>,
    mapped: dir::LocalNodeId<dir::Expression>,
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
    let mapped_extent = module.source_extent(mapped.into_any())?;
    if module.has_unretained_comment(extent, &[value_extent, iterator_extent, mapped_extent])? {
        return Ok(None);
    }

    // preserve the authored binding, iterator, and mapped expression
    let value = module.source(value_extent)?;
    let iterator = module.expression_source(iterator, dir::OperatorPrecedence::Postfix)?;
    let is_object = matches!(
        module.view().get(mapped),
        dir::Expression::ObjectExpression { .. }
    );
    let mapped = module.source(mapped_extent)?;
    let mapped = if is_object {
        format!("({mapped})")
    } else {
        mapped.to_owned()
    };
    let replacement = format!("return {iterator}.map(({value}) => {mapped});");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("return the mapped array directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a complete push-and-return mapping loop.
    #[test]
    fn test_replaces_manual_map() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
function doubled(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        result.push(value * 2);
    }
    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-map]: loop manually collects mapped values
 ──▶ main.tspp:3:5
  │
1 │ function doubled(values: int32[]): int32[] {
2 │     let result: int32[] = [];
3 │     for (const value of values) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │         result.push(value * 2);
  │         ^^^^^^^^^^^^^^^^^^^^^^^
5 │     }
  │     ^
6 │     return result;
7 │ }
  │

 = suggestion: return the mapped array directly (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function doubled(values: int32[]): int32[] {
-   2│     let result: int32[] = [];
-   3│     for (const value of values) {
-   4│         result.push(value * 2);
-   5│     }
-   6│     return result;
+   2│     return values.map((value) => value * 2);
    7│ }
"#,
        );
        session.assert_suggestions(
            r#"
function doubled(values: int32[]): int32[] {
    return values.map((value) => value * 2);
}
"#,
        );
    }

    /// Parenthesize an object literal used as the generated lambda value.
    #[test]
    fn test_maps_object_literal() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
function wrap(values: int32[]): { value: int32 }[] {
    let result: { value: int32 }[] = [];
    for (const value of values) {
        result.push({ value });
    }
    return result;
}
"#,
        );

        session.assert_suggestions(
            r#"
function wrap(values: int32[]): { value: int32 }[] {
    return values.map((value) => ({ value }));
}
"#,
        );
    }

    /// Preserve a destructured iteration pattern as the generated lambda parameter.
    #[test]
    fn test_maps_destructured_value() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
function values(entries: { value: int32 }[]): int32[] {
    let result: int32[] = [];
    for (const { value } of entries) {
        result.push(value);
    }
    return result;
}
"#,
        );

        session.assert_suggestions(
            r#"
function values(entries: { value: int32 }[]): int32[] {
    return entries.map(({ value }) => value);
}
"#,
        );
    }

    /// Preserve a trapping unwrap in the generated synchronous callback.
    #[test]
    fn test_maps_must_value() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
function unwrap(values: (int32 | undefined)[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        result.push(value!);
    }
    return result;
}
"#,
        );

        session.assert_suggestions(
            r#"
function unwrap(values: (int32 | undefined)[]): int32[] {
    return values.map((value) => value!);
}
"#,
        );
    }

    /// Accept a mapped expression that changes its iteration binding.
    #[test]
    fn test_accepts_mutated_value() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
declare function increment(value: &int32): int32;

function incremented(values: int32[]): int32[] {
    let result: int32[] = [];
    for (let value of values) {
        result.push(increment(&value));
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop with another body action.
    #[test]
    fn test_accepts_additional_action() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
function doubled(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        result.push(value);
        result.push(value);
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a mapped value that awaits in its enclosing async function.
    #[test]
    fn test_accepts_awaited_mapping() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
async function double(value: int32): Promise<int32> {
    return value * 2;
}

async function doubled(values: int32[]): Promise<int32[]> {
    let result: int32[] = [];
    for (const value of values) {
        result.push(await double(value));
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
            &MANUAL_MAP,
            r#"
function copy(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        result.push(value);
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
            &MANUAL_MAP,
            r#"
declare function select(result: int32[], values: int32[]): int32[];

function copy(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of select(result, values)) {
        result.push(value);
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
            &MANUAL_MAP,
            r#"
function copy(values: int32[]): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        result.push(value);
    }
    return result;
    result.push(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept collecting through a user-defined push method.
    #[test]
    fn test_accepts_user_push() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
class Values {
    push(value: int32): void {
        // intentionally empty
    }
}

function copy(source: int32[]): Values {
    const result = new Values();
    for (const value of source) {
        result.push(value);
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept manual mapping over a non-array iterable.
    #[test]
    fn test_accepts_other_iterable() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
function doubled(values: Set<int32>): int32[] {
    let result: int32[] = [];
    for (const value of values) {
        result.push(value * 2);
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
