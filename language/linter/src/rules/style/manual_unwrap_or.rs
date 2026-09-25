use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Result fallback methods over equivalent pattern matching.
    pub MANUAL_UNWRAP_OR {
        id: "manual-unwrap-or",
        summary: "Prefer Result fallback methods over equivalent pattern matching",
        explanation: r#"
Branching on a Result to select its successful payload or a fallback manually unwraps the Result.
Instead, you SHOULD use `unwrapOr` for eager values or `unwrapOrElse` for computed fallbacks.
"#,
        example: {
            reported: r#"
function value(result: Result<int32, string>): int32 {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => 0
    };
}
"#,
            accepted: r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrapOr(0);
}
"#,
        },
        provenance: [Clippy("manual_unwrap_or")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report Result matches that manually select a fallback value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect match and if-let Result fallbacks
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let matched = match node {
            dir::Expression::Match { value, arms } => {
                let [first, second] = arms.as_slice() else {
                    continue;
                };

                // reverse only disjoint Result variants, never a leading wildcard
                let selected = fallback(module, *first, *second, &occurrences)?;
                let selected = if selected.is_some()
                    || matches!(view.get(view.get(*first).pattern()), dir::Pattern::Wildcard)
                {
                    selected
                } else {
                    fallback(module, *second, *first, &occurrences)?
                };

                selected.map(|fallback| (*value, fallback))
            }
            dir::Expression::If { .. } => conditional_fallback(module, expression)?,
            _ => None,
        };
        let Some((value, fallback)) = matched else {
            continue;
        };
        if module.representation_item(value.into_any())? != Some(dir::LanguageItem::Result) {
            continue;
        }
        let value_type = module.node_type_id(value.into_any())?;
        if module.dir.borrow_access(value_type)?.is_some() {
            continue;
        }
        if module.uses_enclosing_control(fallback.body)? {
            continue;
        }
        let result = dir::LanguageItem::Result;
        if module.is_within_language_member(expression.into_any(), result.member("unwrapOr"))?
            || module
                .is_within_language_member(expression.into_any(), result.member("unwrapOrElse"))?
        {
            continue;
        }

        // require the fallback to retain the Result payload type
        let payload_type = module
            .dir
            .strip_form(module.node_type_id(fallback.payload.into_any())?)?;
        let fallback_type = module
            .dir
            .strip_form(module.adjusted_type_id(fallback.value.into_any())?)?;
        if payload_type != fallback_type {
            continue;
        }
        if fallback.is_single_value
            && module.language_member(fallback.value)?
                == Some(dir::LanguageItem::Default.member("default"))
        {
            continue;
        }

        // replace the complete match with the corresponding fallback method
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("match manually unwraps a Result fallback", extent);
        if let Some(suggestion) = suggestion(module, lint, extent, value, fallback)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// One matched fallback body and its optional error binding.
#[derive(Debug, Clone, Copy)]
struct ResultFallback {
    /// The complete fallback body.
    body: dir::LocalNodeIdAny,
    /// The successful Result payload.
    payload: dir::LocalNodeId<dir::Expression>,
    /// The terminal fallback value.
    value: dir::LocalNodeId<dir::Expression>,
    /// Whether the fallback consists only of its terminal value.
    is_single_value: bool,
    /// The error binding read by the fallback.
    binding: Option<dir::GlobalSymbolId>,
}

/// Return the Result and fallback selected by one if-let expression.
fn conditional_fallback(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<(dir::LocalNodeId<dir::Expression>, ResultFallback)>, ProviderError> {
    let view = module.view();

    // select one unannotated successful binding condition with a fallback
    let dir::Expression::If {
        form: dir::IfForm::If,
        condition,
        then_expression,
        else_expression: Some(else_expression),
    } = view.get(expression)
    else {
        return Ok(None);
    };
    let Some((_, _, declarator)) = condition.as_binding() else {
        return Ok(None);
    };
    let declarator = view.get(declarator);
    if declarator.ty.is_some()
        || module.pattern_language_item(declarator.pattern)? != Some(dir::LanguageItem::Ok)
    {
        return Ok(None);
    }
    let Some(result) = declarator.value else {
        return Ok(None);
    };
    let Some(payload) = module.sole_value_expression(*then_expression) else {
        return Ok(None);
    };

    // require the successful payload to be returned unchanged
    if module
        .selected_pattern_binding(declarator.pattern, payload)?
        .is_none()
    {
        return Ok(None);
    }

    // retain the complete fallback and its terminal value
    let Some(value) = module.value_expression(*else_expression) else {
        return Ok(None);
    };
    let is_single_value = module.sole_value_expression(*else_expression).is_some();
    let fallback = ResultFallback {
        body: else_expression.into_any(),
        payload,
        value,
        is_single_value,
        binding: None,
    };

    Ok(Some((result, fallback)))
}

/// Return one successful payload arm followed by an error fallback arm.
fn fallback(
    module: &DirModule<'_>,
    success_arm: dir::LocalNodeId<dir::MatchArm>,
    fallback_arm: dir::LocalNodeId<dir::MatchArm>,
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<ResultFallback>, ProviderError> {
    let view = module.view();

    // require a successful payload returned unchanged
    let Some((success_pattern, success_body)) = module.match_arm_value(success_arm) else {
        return Ok(None);
    };
    if module.pattern_language_item(success_pattern)? != Some(dir::LanguageItem::Ok) {
        return Ok(None);
    }
    if module
        .selected_pattern_binding(success_pattern, success_body)?
        .is_none()
    {
        return Ok(None);
    }

    // require one unguarded error arm with at most one binding
    let (pattern, body, value, is_single_value) = match view.get(fallback_arm) {
        dir::MatchArm::Expression {
            pattern,
            guard: None,
            body,
        } => (*pattern, body.into_any(), *body, true),
        dir::MatchArm::Block {
            pattern,
            guard: None,
            body,
        } => {
            let block = module.view().get(*body);
            let Some(value) = block.value_expression() else {
                return Ok(None);
            };

            (
                *pattern,
                body.into_any(),
                value,
                block.leading_expressions.is_empty(),
            )
        }
        _ => return Ok(None),
    };
    if !matches!(view.get(pattern), dir::Pattern::Wildcard)
        && module.pattern_language_item(pattern)? != Some(dir::LanguageItem::Err)
    {
        return Ok(None);
    }
    let mut bindings = module.symbols_declared_within(pattern.into_any());
    let binding = bindings.next();
    if bindings.next().is_some() {
        return Ok(None);
    }

    // retain the binding only when the fallback reads it
    let binding = binding.filter(|binding| {
        !module
            .binding_uses_within(*binding, fallback_arm.into_any(), occurrences)
            .is_empty()
    });

    Ok(Some(ResultFallback {
        body,
        payload: success_body,
        value,
        is_single_value,
        binding,
    }))
}

/// Build one Result fallback call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: Span,
    result: dir::LocalNodeId<dir::Expression>,
    fallback: ResultFallback,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let result_extent = module.source_extent(result.into_any())?;
    let fallback_extent = module.source_extent(fallback.body)?;

    // read the optional error parameter from its declaration
    let binding = if let Some(binding) = fallback.binding {
        let declaration = module.symbol_declaration(binding)?;
        let declaration = module.source_extent(declaration.local_id)?;

        Some((declaration, module.source(declaration)?))
    } else {
        None
    };

    // preserve eager evaluation only for a speculatable Copy fallback
    let result = module.expression_source(result, dir::OperatorPrecedence::Postfix)?;
    let is_eager =
        if fallback.is_single_value && module.is_speculatable_expression(fallback.value)? {
            let value_type = module.node_type_id(fallback.value.into_any())?;
            module.satisfies_copy(value_type)?
        } else {
            false
        };
    let replacement = if binding.is_none() && is_eager {
        let value = fallback.value;
        let value_extent = module.source_extent(value.into_any())?;
        if module.has_unretained_comment(extent, &[result_extent, value_extent])? {
            return Ok(None);
        }
        let value = module.expression_source(value, dir::OperatorPrecedence::Lowest)?;

        format!("{result}.unwrapOr({value})")
    } else {
        let mut retained = vec![result_extent, fallback_extent];
        if let Some((declaration, _)) = binding {
            retained.push(declaration);
        }
        if module.has_unretained_comment(extent, &retained)? {
            return Ok(None);
        }

        // align a retained match block with its containing expression
        let body = if fallback.body.ty == dir::NodeType::Block {
            let expression_indentation = module.source_indentation(extent)?.len();
            let fallback_indentation = module.source_indentation(fallback_extent)?.len();
            let indentation = fallback_indentation
                .checked_sub(expression_indentation)
                .ok_or_else(|| {
                    ProviderError::internal("match arm is less indented than its expression")
                })? as u32;
            let Some(body) = module.dedent_source(fallback_extent, indentation)? else {
                return Ok(None);
            };

            body
        } else {
            module.source(fallback_extent)?.to_owned()
        };

        // retain the error parameter only when the fallback reads it
        let callback = match binding {
            Some((_, binding)) => format!("({binding}) => {body}"),
            None => format!("() => {body}"),
        };

        format!("{result}.unwrapOrElse({callback})")
    };
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("use a Result fallback method", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Leave canonical default fallbacks to manual-unwrap-or-default.
    #[test]
    fn test_accepts_default_fallback() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
function value<T: Default>(result: Result<T, string>): T {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => T.default()
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve an error-dependent fallback with unwrapOrElse.
    #[test]
    fn test_replaces_error_fallback() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
function length(result: Result<isize, string>): isize {
    return match (result) {
        Ok { value } => value
        Err { error } => error.length
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
function length(result: Result<isize, string>): isize {
    return result.unwrapOrElse((error) => error.length);
}
"#,
        );
    }

    /// Preserve lazy evaluation for a called fallback.
    #[test]
    fn test_replaces_computed_fallback() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
declare function fallback(): int32;

function value(result: Result<int32, string>): int32 {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => fallback()
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function fallback(): int32;

function value(result: Result<int32, string>): int32 {
    return result.unwrapOrElse(() => fallback());
}
"#,
        );
    }

    /// Preserve conditional ownership transfer for a move-only fallback.
    #[test]
    fn test_replaces_move_only_fallback_lazily() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
struct Label {
    values: ^int32[];
}

function value(result: Result<Label, string>, fallback: Label): Label {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => fallback
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
struct Label {
    values: ^int32[];
}

function value(result: Result<Label, string>, fallback: Label): Label {
    return result.unwrapOrElse(() => fallback);
}
"#,
        );
    }

    /// Preserve statements in a lazy fallback block.
    #[test]
    fn test_replaces_fallback_block() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
declare function observe(): void;

function value(result: Result<int32, string>): int32 {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => {
            observe();
            0
        }
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function observe(): void;

function value(result: Result<int32, string>): int32 {
    return result.unwrapOrElse(() => {
        observe();
        0
    });
}
"#,
        );
    }

    /// Replace an if-let Result fallback.
    #[test]
    fn test_replaces_conditional_fallback() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
function value(result: Result<int32, string>): int32 {
    return if (let Ok { value } = result) {
        value
    } else {
        0
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrapOr(0);
}
"#,
        );
    }

    /// Replace an exhaustive wildcard fallback.
    #[test]
    fn test_replaces_wildcard_fallback() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
function value(result: Result<int32, string>): int32 {
    return match (result) {
        Ok { value } => value
        _ => 0
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrapOr(0);
}
"#,
        );
    }

    /// Preserve a fallback that returns from the enclosing function.
    #[test]
    fn test_accepts_returning_fallback() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
function value(result: Result<int32, string>): int32 {
    const selected = match (result) {
        Ok { value } => value
        Err { error: _ } => return 0
    };

    return selected;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a transformed success payload.
    #[test]
    fn test_accepts_transformed_success() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
function increment(result: Result<int32, string>): int32 {
    return match (result) {
        Ok { value } => value + 1
        Err { error: _ } => 0
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept fallback selection from a raw variant union.
    #[test]
    fn test_accepts_raw_variant_union() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
function value(result: Ok<int32> | Err<string>): int32 {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => 0
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve fallback selection from a borrowed Result.
    #[test]
    fn test_accepts_borrowed_result() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
function value(result: &readonly Result<int32, int32>): int32 {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => 0
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve fallback selection from a Result borrowed at an access parameter.
    #[test]
    fn test_accepts_result_borrowed_at_an_access_parameter() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
function value<'a, const A: Access>(result: Borrowed<Result<int32, int32>, 'a, A>): int32 {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => 0
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a match that widens both Result branches.
    #[test]
    fn test_accepts_widened_result() {
        let session = TestSession::dir(
            &MANUAL_UNWRAP_OR,
            r#"
function value(result: Result<int32, string>): int32 | float64 {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => 0.0
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
