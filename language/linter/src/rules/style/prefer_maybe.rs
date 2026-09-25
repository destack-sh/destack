use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer the maybe operator over manual propagation.
    pub PREFER_MAYBE {
        id: "prefer-maybe",
        summary: "Prefer the maybe operator over manual propagation",
        explanation: r#"
Explicit branching that immediately propagates a Result error or nullish value repeats the behavior of the maybe operator.
Instead, you SHOULD use the maybe operator to propagate absence or failure.
"#,
        example: {
            reported: r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(error)
    };
    return Result.ok(value);
}
"#,
            accepted: r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = result?;
    return Result.ok(value);
}
"#,
        },
        provenance: [Clippy("question_mark")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report control flow that manually propagates absence or failure.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect matches and conditions that may implement propagation
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let propagation = match node {
            dir::Expression::Match { value, arms } => {
                match_propagation(module, expression, *value, arms)?.map(|value| (value, false))
            }
            dir::Expression::If { .. } => {
                condition_propagation(module, expression)?.map(|value| (value, true))
            }
            _ => None,
        };
        let Some((value, is_statement)) = propagation else {
            continue;
        };
        if module.catching_try(expression.into_any()).is_some() {
            continue;
        }

        // preserve the narrowing established by a conditional pattern
        if is_statement
            && matches!(view.get(value), dir::Expression::Identifier { .. })
            && let Some(symbol) = module.selected_symbol(value)?
            && !module
                .binding_uses_after(symbol, expression.into_any(), &occurrences)?
                .is_empty()
        {
            continue;
        }

        // replace the complete control expression with propagation
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic(
            "control flow manually propagates absence or failure",
            extent,
        );
        if let Some(suggestion) = suggestion(module, lint, extent, value, is_statement)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select a value propagated manually by one match.
fn match_propagation(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
    arms: &[dir::LocalNodeId<dir::MatchArm>],
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let [first, second] = arms else {
        return Ok(None);
    };
    let Some(coverage) = module.match_coverage(expression) else {
        return Ok(None);
    };
    if !coverage.is_exhaustive {
        return Ok(None);
    }

    // recognize complete Result propagation in either arm order
    if let Some(output) = result_output(module, value)?
        && !module.dir.type_includes(output, |ty| {
            matches!(ty, dir::Type::Null | dir::Type::Undefined)
        })?
        && (result_arms_propagate(module, *first, *second)?
            || result_arms_propagate(module, *second, *first)?)
    {
        return Ok(Some(value));
    }

    // recognize complete propagation of one nullish member
    let value_type = module.node_type_id(value.into_any())?;
    if let Some(nullish) = module.dir.sole_nullish(value_type)?
        && nullish_arms_propagate(module, *second, *first, nullish)?
    {
        return Ok(Some(value));
    }

    Ok(None)
}

/// Return whether two arms extract success and return the unchanged Result error.
fn result_arms_propagate(
    module: &DirModule<'_>,
    success_arm: dir::LocalNodeId<dir::MatchArm>,
    error_arm: dir::LocalNodeId<dir::MatchArm>,
) -> Result<bool, ProviderError> {
    // require the successful payload to be returned unchanged
    let Some((success_pattern, success_body)) = module.match_arm_value(success_arm) else {
        return Ok(false);
    };
    if module.pattern_language_item(success_pattern)? != Some(dir::LanguageItem::Ok) {
        return Ok(false);
    }
    if module
        .selected_pattern_binding(success_pattern, success_body)?
        .is_none()
    {
        return Ok(false);
    }

    // require the error payload to be returned through Result.err unchanged
    let Some((error_pattern, error_body)) = module.match_arm_expression(error_arm) else {
        return Ok(false);
    };

    returns_result_error(module, error_pattern, error_body)
}

/// Return whether two arms extract a value and return its nullish alternative.
fn nullish_arms_propagate(
    module: &DirModule<'_>,
    success_arm: dir::LocalNodeId<dir::MatchArm>,
    nullish_arm: dir::LocalNodeId<dir::MatchArm>,
    nullish: dir::Literal,
) -> Result<bool, ProviderError> {
    // require the present value to be returned unchanged
    let Some((success_pattern, success_body)) = module.match_arm_value(success_arm) else {
        return Ok(false);
    };
    if module
        .selected_pattern_binding(success_pattern, success_body)?
        .is_none()
    {
        return Ok(false);
    }

    // require the exact absent value to be returned unchanged
    let Some((nullish_pattern, nullish_body)) = module.match_arm_expression(nullish_arm) else {
        return Ok(false);
    };
    if module.pattern_literal(nullish_pattern)? != Some(nullish) {
        return Ok(false);
    }

    returns_nullish(module, nullish_body, nullish)
}

/// Select a value propagated manually by one condition.
fn condition_propagation(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    // select one condition without an else branch
    let dir::Expression::If {
        form: dir::IfForm::If,
        condition,
        then_expression,
        else_expression: None,
    } = module.view().get(expression)
    else {
        return Ok(None);
    };
    let Some(body) = module.sole_expression(*then_expression) else {
        return Ok(None);
    };

    // recognize a failed Result binding returned unchanged
    if let Some(result) = result_condition_propagation(module, condition, body)? {
        return Ok(Some(result));
    }

    // recognize an exact nullish condition returned unchanged
    if let Some(value) = nullish_condition_propagation(module, condition, body)? {
        return Ok(Some(value));
    }

    Ok(None)
}

/// Select a Result propagated by one failed-pattern condition.
fn result_condition_propagation(
    module: &DirModule<'_>,
    condition: &dir::Condition,
    body: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some((_, _, declarator)) = condition.as_binding() else {
        return Ok(None);
    };
    let declarator = module.view().get(declarator);
    if declarator.ty.is_some() {
        return Ok(None);
    }
    let Some(result) = declarator.value else {
        return Ok(None);
    };
    let Some(output) = result_output(module, result)? else {
        return Ok(None);
    };
    if module.dir.type_includes(output, |ty| {
        matches!(ty, dir::Type::Null | dir::Type::Undefined)
    })? || !returns_result_error(module, declarator.pattern, body)?
    {
        return Ok(None);
    }

    Ok(Some(result))
}

/// Select a nullish value propagated by one exact condition.
fn nullish_condition_propagation(
    module: &DirModule<'_>,
    condition: &dir::Condition,
    body: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some(condition) = condition.as_expression() else {
        return Ok(None);
    };
    let Some(test) = module.nullish_test(condition)? else {
        return Ok(None);
    };
    if test.is_defined {
        return Ok(None);
    }
    let value_type = module.node_type_id(test.value.into_any())?;
    let Some(nullish) = module.dir.sole_nullish(value_type)? else {
        return Ok(None);
    };
    if !returns_nullish(module, body, nullish)? {
        return Ok(None);
    }

    Ok(Some(test.value))
}

/// Return the opened output type of one owned Result value.
fn result_output(
    module: &DirModule<'_>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::GlobalTypeId>, ProviderError> {
    if module.representation_item(value.into_any())? != Some(dir::LanguageItem::Result) {
        return Ok(None);
    }
    let value_type = module.node_type_id(value.into_any())?;
    if module.dir.borrow_access(value_type)?.is_some() {
        return Ok(None);
    }

    module.dir.application_argument(value_type, 0)
}

/// Return whether one failed Result pattern is returned unchanged.
fn returns_result_error(
    module: &DirModule<'_>,
    pattern: dir::LocalNodeId<dir::Pattern>,
    body: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let view = module.view();

    // require one failed Result pattern returned through Result.err
    if module.pattern_language_item(pattern)? != Some(dir::LanguageItem::Err) {
        return Ok(false);
    }
    let dir::Expression::Return {
        value: Some(returned),
    } = view.get(body)
    else {
        return Ok(false);
    };
    let returned_type = module.adjusted_type_id(returned.into_any())?;
    if module.dir.representation_item(returned_type)? != Some(dir::LanguageItem::Result) {
        return Ok(false);
    }
    if module.language_member(*returned)? != Some(dir::LanguageItem::Result.member("err")) {
        return Ok(false);
    }

    // require the unchanged error as the sole argument
    let dir::Expression::Call { arguments, .. } = view.get(*returned) else {
        return Ok(false);
    };
    let [argument] = arguments.as_slice() else {
        return Ok(false);
    };
    let dir::Argument::Positional { value } = view.get(*argument) else {
        return Ok(false);
    };
    if module
        .coercions
        .coercion(value.into_global_any(module.id))
        .is_some()
    {
        return Ok(false);
    }

    Ok(module.selected_pattern_binding(pattern, *value)?.is_some())
}

/// Return whether one expression returns the given nullish literal.
fn returns_nullish(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    nullish: dir::Literal,
) -> Result<bool, ProviderError> {
    let dir::Expression::Return {
        value: Some(returned),
    } = module.view().get(expression)
    else {
        return Ok(false);
    };

    Ok(module.scalar_constant(*returned)? == Some(nullish))
}

/// Build one maybe propagation expression.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    result: dir::LocalNodeId<dir::Expression>,
    is_statement: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(result.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // preserve authored grouping before the postfix operator
    let result = module.expression_source(result, dir::OperatorPrecedence::Postfix)?;
    let terminator = if is_statement { ";" } else { "" };
    let patch = Patch::replace(extent, format!("{result}?{terminator}"));
    let suggestion = lint.fix("use the maybe operator", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Recognize reversed Result arm ordering.
    #[test]
    fn test_replaces_reversed_arms() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Err { error } => return Result.err(error)
        Ok { value } => value
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_fixes(
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = result?;
    return Result.ok(value);
}
"#,
        );
    }

    /// Replace propagation written in a sole-expression block.
    #[test]
    fn test_replaces_error_block() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => {
            return Result.err(error);
        }
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_fixes(
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = result?;
    return Result.ok(value);
}
"#,
        );
    }

    /// Replace a failed-pattern condition that returns its error unchanged.
    #[test]
    fn test_replaces_error_condition() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function consume(result: Result<int32, string>): Result<int32, string> {
    if (let Err { error } = result) {
        return Result.err(error);
    }
    return Result.ok(0);
}
"#,
        );

        session.assert_fixes(
            r#"
function consume(result: Result<int32, string>): Result<int32, string> {
    result?;
    return Result.ok(0);
}
"#,
        );
    }

    /// Replace a match that propagates undefined unchanged.
    #[test]
    fn test_replaces_undefined_match() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(maybe: int32 | undefined): int32 | undefined {
    const value = match (maybe) {
        undefined => return undefined
        value => value
    };
    return value;
}
"#,
        );

        session.assert_fixes(
            r#"
function value(maybe: int32 | undefined): int32 | undefined {
    const value = maybe?;
    return value;
}
"#,
        );
    }

    /// Replace a condition that propagates undefined unchanged.
    #[test]
    fn test_replaces_undefined_condition() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(maybe: int32 | undefined): int32 | undefined {
    if (maybe === undefined) {
        return undefined;
    }
    return 0;
}
"#,
        );

        session.assert_fixes(
            r#"
function value(maybe: int32 | undefined): int32 | undefined {
    maybe?;
    return 0;
}
"#,
        );
    }

    /// Replace a match that propagates null unchanged.
    #[test]
    fn test_replaces_null_match() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(maybe: int32 | null): int32 | null {
    const value = match (maybe) {
        null => return null
        value => value
    };
    return value;
}
"#,
        );

        session.assert_fixes(
            r#"
function value(maybe: int32 | null): int32 | null {
    const value = maybe?;
    return value;
}
"#,
        );
    }

    /// Preserve a Result whose successful variant remains in use.
    #[test]
    fn test_accepts_result_used_after_condition() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function consume(result: Result<int32, string>): Result<int32, string> {
    if (let Err { error } = result) {
        return Result.err(error);
    }
    return Result.ok(result.value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an error transformation before returning.
    #[test]
    fn test_accepts_transformed_error() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(`invalid: ${error}`)
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a Result whose successful value can propagate absence.
    #[test]
    fn test_accepts_nullish_result_output() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(result: Result<int32 | undefined, string>): Result<int32 | undefined, string> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(error)
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a condition that normalizes two absent values into one.
    #[test]
    fn test_accepts_combined_nullish_condition() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(maybe: int32 | null | undefined): int32 | undefined {
    if (maybe == null) {
        return undefined;
    }
    return maybe;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a nullish value whose narrowed form remains in use.
    #[test]
    fn test_accepts_nullish_value_used_after_condition() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(maybe: int32 | undefined): int32 | undefined {
    if (maybe === undefined) {
        return undefined;
    }
    return maybe;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve explicit returns inside a catching try expression.
    #[test]
    fn test_accepts_return_inside_try() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(
    result: Result<int32, string>,
    fallback: Result<void, string>,
): Result<int32, string> {
    try {
        fallback?;
        const value = match (result) {
            Ok { value } => value
            Err { error } => return Result.err(error)
        };
        return Result.ok(value);
    } catch (error) {
        return Result.err(`caught: ${error}`);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept manual propagation from a raw variant union without Try conformance.
    #[test]
    fn test_accepts_raw_variant_union() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(result: Ok<int32> | Err<string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(error)
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve propagation from a borrowed Result that cannot satisfy Try by value.
    #[test]
    fn test_accepts_borrowed_result() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(result: &readonly Result<int32, int32>): Result<int32, int32> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(error)
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve propagation into a return type that does not implement Try.
    #[test]
    fn test_accepts_union_return() {
        let session = TestSession::dir(
            &PREFER_MAYBE,
            r#"
function value(result: Result<int32, string>): Result<int32, string> | undefined {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(error)
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
