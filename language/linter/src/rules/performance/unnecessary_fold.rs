use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer a specialized iterator operation over an equivalent fold.
    pub UNNECESSARY_FOLD {
        id: "unnecessary-fold",
        summary: "Prefer a specialized iterator operation over an equivalent fold",
        explanation: r#"
A boolean reduction expresses an existential or universal predicate through an accumulator.
Instead, you SHOULD use `some` or `every` to state the predicate and short circuit the iteration.

The replacement stops evaluating the predicate once its result is known.
"#,
        example: {
            reported: r#"
function containsPositive(values: int32[]): boolean {
    return values.reduce((found, value) => found || value > 0, false);
}
"#,
            accepted: r#"
function containsPositive(values: int32[]): boolean {
    return values.some((value) => value > 0);
}
"#,
        },
        provenance: [Clippy("unnecessary_fold")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One boolean reduction replaceable by a terminal predicate.
struct BooleanReduction {
    /// The replacement terminal method.
    method: &'static str,
    /// The parameters retained after removing the accumulator.
    parameters: Vec<dir::LocalNodeId<dir::Parameter>>,
    /// The predicate evaluated for each value.
    predicate: dir::LocalNodeId<dir::Expression>,
}

/// Report boolean reductions expressible as some or every.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect canonical Array and Iterator reductions
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional() || !call.generic_arguments.is_empty() {
            continue;
        }
        let member = module.language_member(expression)?;
        if member != Some(dir::LanguageItem::Array.member("reduce"))
            && member != Some(dir::LanguageItem::Iterator.member("reduce"))
        {
            continue;
        }
        let [callback, initial] = call.arguments else {
            continue;
        };
        let (Some(callback), Some(initial)) = (
            module.view().get(*callback).value(),
            module.view().get(*initial).value(),
        ) else {
            continue;
        };
        let Some(reduction) = select_boolean_reduction(module, callback, initial, &occurrences)?
        else {
            continue;
        };

        // replace the reduction with its terminal predicate
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("boolean accumulator duplicates a terminal predicate", span);
        if let Some(suggestion) = suggestion(module, lint, expression, call.receiver, &reduction)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the terminal predicate represented by one boolean reduction.
fn select_boolean_reduction(
    module: &DirModule<'_>,
    callback: dir::LocalNodeId<dir::Expression>,
    initial: dir::LocalNodeId<dir::Expression>,
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<BooleanReduction>, ProviderError> {
    let Some(lambda) = module.lambda(callback) else {
        return Ok(None);
    };
    if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
        return Ok(None);
    }
    let [accumulator, parameters @ ..] = lambda.signature.parameters.as_slice() else {
        return Ok(None);
    };
    if parameters.len() > 2
        || !matches!(
            module.view().get(*accumulator),
            dir::Parameter::Named { .. }
        )
    {
        return Ok(None);
    }
    let Some(body) = lambda
        .body
        .and_then(|body| module.sole_value_expression(body))
    else {
        return Ok(None);
    };
    let Some((operator, [left, right])) = module.builtin_binary(body)? else {
        return Ok(None);
    };

    // require the accumulator as the short circuiting left operand
    let accumulator = module.declaration_symbol(*accumulator)?;
    if module.selected_symbol(left.source.local_id)? != Some(accumulator) {
        return Ok(None);
    }
    let predicate = right.source.local_id;
    if !module
        .binding_uses_within(accumulator, predicate.into_any(), occurrences)
        .is_empty()
    {
        return Ok(None);
    }

    // pair the logical identity with its terminal operation
    let method = match (operator, module.view().get(initial).as_boolean()) {
        (dir::BinaryOperator::Or, Some(false)) => "some",
        (dir::BinaryOperator::And, Some(true)) => "every",
        _ => return Ok(None),
    };
    let reduction = BooleanReduction {
        method,
        parameters: parameters.to_vec(),
        predicate,
    };

    Ok(Some(reduction))
}

/// Replace one boolean reduction with its terminal predicate.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
    reduction: &BooleanReduction,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver_span = module.source_extent(receiver.into_any())?;
    let predicate_span = module.source_extent(reduction.predicate.into_any())?;
    let mut retained = vec![receiver_span, predicate_span];
    for parameter in &reduction.parameters {
        retained.push(module.source_extent(parameter.into_any())?);
    }
    if module.has_unretained_comment(extent, &retained)? {
        return Ok(None);
    }

    // retain the receiver, non-accumulator parameters, and predicate source
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Postfix)?;
    let mut parameters = Vec::with_capacity(reduction.parameters.len());
    for parameter in &reduction.parameters {
        let span = module.source_extent(parameter.into_any())?;
        parameters.push(module.source(span)?);
    }
    let parameters = parameters.join(", ");
    let predicate =
        module.expression_source(reduction.predicate, dir::OperatorPrecedence::Lowest)?;
    let replacement = format!(
        "{receiver}.{}(({parameters}) => {predicate})",
        reduction.method
    );
    let suggestion = lint.suggestion(
        format!("express the reduction with `{}`", reduction.method),
        Patch::replace(extent, replacement),
    )?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an existential boolean reduction with some.
    #[test]
    fn test_replaces_existential_reduction() {
        let session = TestSession::dir(
            &UNNECESSARY_FOLD,
            r#"
function containsPositive(values: int32[]): boolean {
    return values.reduce((found, value) => found || value > 0, false);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[unnecessary-fold]: boolean accumulator duplicates a terminal predicate
 ──▶ main.tspp:2:12
  │
1 │ function containsPositive(values: int32[]): boolean {
2 │     return values.reduce((found, value) => found || value > 0, false);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = suggestion: express the reduction with `some` (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function containsPositive(values: int32[]): boolean {
-   2│     return values.reduce((found, value) => found || value > 0, false);
+   2│     return values.some((value) => value > 0);
    3│ }
"#,
        );
        session.assert_suggestions(
            r#"
function containsPositive(values: int32[]): boolean {
    return values.some((value) => value > 0);
}
"#,
        );
    }

    /// Replace a universal boolean reduction with every.
    #[test]
    fn test_replaces_universal_reduction() {
        let session = TestSession::dir(
            &UNNECESSARY_FOLD,
            r#"
function allPositive(values: isize[]): boolean {
    return values.reduce((accepted, value, index) => accepted && value > index, true);
}
"#,
        );

        session.assert_suggestions(
            r#"
function allPositive(values: isize[]): boolean {
    return values.every((value, index) => value > index);
}
"#,
        );
    }

    /// Replace the same boolean reduction on an Iterator.
    #[test]
    fn test_replaces_iterator_reduction() {
        let session = TestSession::dir(
            &UNNECESSARY_FOLD,
            r#"
import { Iterator } from "tspp:iter";

function any(values: Iterator<boolean>): boolean {
    return values.reduce((found, value, index) => found || value && index > 0, false);
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Iterator } from "tspp:iter";

function any(values: Iterator<boolean>): boolean {
    return values.some((value, index) => value && index > 0);
}
"#,
        );
    }

    /// Retain a predicate that ignores each yielded value.
    #[test]
    fn test_replaces_parameterless_predicate() {
        let session = TestSession::dir(
            &UNNECESSARY_FOLD,
            r#"
function any(values: boolean[], condition: boolean): boolean {
    return values.reduce((found) => found || condition, false);
}
"#,
        );

        session.assert_suggestions(
            r#"
function any(values: boolean[], condition: boolean): boolean {
    return values.some(() => condition);
}
"#,
        );
    }

    /// Accept a reduction with the accumulator on the eager side.
    #[test]
    fn test_accepts_reversed_accumulator() {
        let session = TestSession::dir(
            &UNNECESSARY_FOLD,
            r#"
declare function inspect(value: int32): boolean;

function containsInspected(values: int32[]): boolean {
    return values.reduce((found, value) => inspect(value) || found, false);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a reducer that performs work before its boolean result.
    #[test]
    fn test_accepts_preceding_work() {
        let session = TestSession::dir(
            &UNNECESSARY_FOLD,
            r#"
declare function inspect(value: isize): void;

function allPositive(values: isize[]): boolean {
    return values.reduce((accepted, value) => {
        inspect(value);

        accepted && value > 0
    }, true);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a reduction whose initial value is not the logical identity.
    #[test]
    fn test_accepts_nonidentity_initial_value() {
        let session = TestSession::dir(
            &UNNECESSARY_FOLD,
            r#"
function containsPositive(values: int32[]): boolean {
    return values.reduce((found, value) => found || value > 0, true);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a reduction that uses its accumulator inside the predicate.
    #[test]
    fn test_accepts_accumulator_dependent_predicate() {
        let session = TestSession::dir(
            &UNNECESSARY_FOLD,
            r#"
function alternates(values: boolean[]): boolean {
    return values.reduce((previous, value) => previous || value !== previous, false);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined reduce methods.
    #[test]
    fn test_accepts_user_method() {
        let session = TestSession::dir(
            &UNNECESSARY_FOLD,
            r#"
class Values {
    reduce(callback: (left: boolean, right: boolean) => boolean, initial: boolean): boolean {
        return callback(initial, false);
    }
}

function any(values: Values): boolean {
    return values.reduce((found, value) => found || value, false);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
