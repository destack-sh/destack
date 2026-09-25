use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Iterator.tryFold over reducing a propagated Result accumulator.
    pub MANUAL_TRY_FOLD {
        id: "manual-try-fold",
        summary: "Prefer Iterator.tryFold over reducing a propagated Result accumulator",
        explanation: r#"
Reducing a Result accumulator continues calling the reducer after failure only to propagate the same error again.
Instead, you SHOULD call `tryFold` so iteration stops at the first failed Result.

The rule requires propagation to be the reducer's first operation, preserving every observable effect.
"#,
        example: {
            reported: r#"
import { Iterator } from "tspp:iter";

function sum(values: Iterator<int32>): Result<int32, string> {
    return values.reduce((result, value) => {
        const total = result?;

        Result.ok(total + value)
    }, Result.ok(0));
}
"#,
            accepted: r#"
import { Iterator } from "tspp:iter";

function sum(values: Iterator<int32>): Result<int32, string> {
    return values.tryFold(0, (total, value) => Result.ok(total + value));
}
"#,
        },
        provenance: [Clippy("manual_try_fold")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One Result reduction that begins by propagating its accumulator.
struct TryReduction {
    /// The unwrapped initial accumulator.
    initial: dir::LocalNodeId<dir::Expression>,
    /// The pattern bound to the propagated accumulator.
    accumulator_pattern: dir::LocalNodeId<dir::Pattern>,
    /// The retained non-accumulator parameters.
    parameters: Vec<dir::LocalNodeId<dir::Parameter>>,
    /// The retained reducer result.
    result: dir::LocalNodeId<dir::Expression>,
}

/// Report Result reductions that can stop at the first failure.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect canonical Iterator.reduce calls with an explicit initial value
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Iterator.member("reduce"))
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
        let Some(reduction) = try_reduction(module, callback, initial, &occurrences)? else {
            continue;
        };

        // replace the eager reduction with one short circuiting fold
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("Result reduction continues after its first failure", span);
        if let Some(suggestion) = suggestion(
            module,
            lint,
            expression,
            call.receiver,
            call.generic_arguments,
            &reduction,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one reduction whose first operation propagates its Result accumulator.
fn try_reduction(
    module: &DirModule<'_>,
    callback: dir::LocalNodeId<dir::Expression>,
    initial_result: dir::LocalNodeId<dir::Expression>,
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<TryReduction>, ProviderError> {
    let view = module.view();

    // require one synchronous block reducer with a named accumulator
    let Some(lambda) = module.lambda(callback) else {
        return Ok(None);
    };
    if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
        return Ok(None);
    }
    let [carrier, parameters @ ..] = lambda.signature.parameters.as_slice() else {
        return Ok(None);
    };
    if parameters.len() > 2 || !matches!(view.get(*carrier), dir::Parameter::Named { .. }) {
        return Ok(None);
    }
    let Some(body) = lambda.body else {
        return Ok(None);
    };
    let dir::Expression::Block(block) = view.get(body) else {
        return Ok(None);
    };
    let block = view.get(*block);
    let ([propagation], Some(result)) =
        (block.leading_expressions.as_slice(), block.tail_expression)
    else {
        return Ok(None);
    };

    // require one direct immutable binding initialized by accumulator propagation
    let Some((dir::Mutability::Immutable, declarator)) = module.binding_declarator(*propagation)
    else {
        return Ok(None);
    };
    let Some(initializer) = declarator.value else {
        return Ok(None);
    };
    let dir::Expression::Maybe {
        left: propagated, ..
    } = view.get(initializer)
    else {
        return Ok(None);
    };
    let carrier = module.declaration_symbol(*carrier)?;
    if module.selected_symbol(*propagated)? != Some(carrier) {
        return Ok(None);
    }
    let mut carrier_uses = occurrences.iter().filter(|occurrence| {
        occurrence.symbol == carrier && view.is_inside(occurrence.node, body.into_any())
    });
    let Some(carrier_use) = carrier_uses.next() else {
        return Ok(None);
    };
    if carrier_uses.next().is_some()
        || !carrier_use.uses.contains(dir::BindingUse::READ)
        || !carrier_use.uses.without(dir::BindingUse::READ).is_empty()
    {
        return Ok(None);
    }

    // unwrap one canonical Result.ok initial accumulator of the same carrier type
    if module.language_member(initial_result)? != Some(dir::LanguageItem::Result.member("ok")) {
        return Ok(None);
    }
    let dir::Expression::Call { arguments, .. } = view.get(initial_result) else {
        return Ok(None);
    };
    let [argument] = arguments.as_slice() else {
        return Ok(None);
    };
    let dir::Argument::Positional { value: initial } = view.get(*argument) else {
        return Ok(None);
    };
    let carrier_type = module.adjusted_type_id(propagated.into_any())?;
    if !module.dir.types_match(
        carrier_type,
        module.adjusted_type_id(initial_result.into_any())?,
    )? || !module
        .dir
        .types_match(carrier_type, module.adjusted_type_id(result.into_any())?)?
    {
        return Ok(None);
    }

    Ok(Some(TryReduction {
        initial: *initial,
        accumulator_pattern: declarator.pattern,
        parameters: parameters.to_vec(),
        result,
    }))
}

/// Build one Iterator.tryFold call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
    generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    reduction: &TryReduction,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver_span = module.source_extent(receiver.into_any())?;
    let initial_span = module.source_extent(reduction.initial.into_any())?;
    let accumulator_span = module.source_extent(reduction.accumulator_pattern.into_any())?;
    let result_span = module.source_extent(reduction.result.into_any())?;
    let mut retained = vec![receiver_span, initial_span, accumulator_span, result_span];
    for parameter in &reduction.parameters {
        retained.push(module.source_extent(parameter.into_any())?);
    }
    for argument in generic_arguments {
        retained.push(module.source_extent(argument.into_any())?);
    }
    if module.has_unretained_comment(extent, &retained)? {
        return Ok(None);
    }

    // retain the unwrapped initial value and reducer expression
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Postfix)?;
    let mut generics = Vec::with_capacity(generic_arguments.len());
    for argument in generic_arguments {
        let span = module.source_extent(argument.into_any())?;
        generics.push(module.source(span)?);
    }
    let generics = if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    };
    let initial = module.expression_source(reduction.initial, dir::OperatorPrecedence::Lowest)?;
    let accumulator = module.source(accumulator_span)?;
    let mut parameters = Vec::with_capacity(1 + reduction.parameters.len());
    parameters.push(accumulator);
    for parameter in &reduction.parameters {
        let span = module.source_extent(parameter.into_any())?;
        parameters.push(module.source(span)?);
    }
    let parameters = parameters.join(", ");
    let result = module.expression_source(reduction.result, dir::OperatorPrecedence::Lowest)?;
    let replacement =
        format!("{receiver}.tryFold{generics}({initial}, ({parameters}) => {result})");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("stop the reduction with Iterator.tryFold", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Retain a fallible reducer result.
    #[test]
    fn test_replaces_fallible_reducer() {
        let session = TestSession::dir(
            &MANUAL_TRY_FOLD,
            r#"
import { Iterator } from "tspp:iter";

declare function add(left: int32, right: int32): Result<int32, string>;

function sum(values: Iterator<int32>): Result<int32, string> {
    return values.reduce((result, value, index) => {
        const total = result?;

        add(total, value)
    }, Result.ok(0));
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Iterator } from "tspp:iter";

declare function add(left: int32, right: int32): Result<int32, string>;

function sum(values: Iterator<int32>): Result<int32, string> {
    return values.tryFold(0, (total, value, index) => add(total, value));
}
"#,
        );
    }

    /// Accept reducers that perform work before propagating the accumulator.
    #[test]
    fn test_accepts_work_before_propagation() {
        let session = TestSession::dir(
            &MANUAL_TRY_FOLD,
            r#"
import { Iterator } from "tspp:iter";

declare function inspect(value: int32): void;

function sum(values: Iterator<int32>): Result<int32, string> {
    return values.reduce((result, value, index) => {
        inspect(value);
        const total = result?;

        Result.ok(total + value)
    }, Result.ok(0));
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept reducers that inspect a failed accumulator more than once.
    #[test]
    fn test_accepts_additional_accumulator_use() {
        let session = TestSession::dir(
            &MANUAL_TRY_FOLD,
            r#"
import { Iterator } from "tspp:iter";

function sum(values: Iterator<int32>): Result<int32, string> {
    return values.reduce((result, value, index) => {
        result;
        const total = result?;

        Result.ok(total + value)
    }, Result.ok(0));
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
