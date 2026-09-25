use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Simplify boolean expressions with redundant or contradictory terms.
    pub NONMINIMAL_BOOL {
        id: "nonminimal-bool",
        summary: "Simplify boolean expressions with redundant or contradictory terms",
        explanation: r#"
Repeated, complementary, or absorbed terms make a boolean expression more complex without changing its value.
Instead, you SHOULD remove the redundant terms or use the resulting boolean constant.
"#,
        example: {
            reported: r#"
function canRead(isOwner: boolean, isShared: boolean): boolean {
    return isOwner || (isOwner && isShared);
}
"#,
            accepted: r#"
function canRead(isOwner: boolean, isShared: boolean): boolean {
    return isOwner;
}
"#,
        },
        provenance: [Clippy("nonminimal_bool")],
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report builtin logical chains containing terms removable by boolean identities.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect only roots of builtin and and or chains
    for expression in module.view().iter_node_ids_of_type::<dir::Expression>() {
        let Some((operator, _)) = module.builtin_binary(expression)? else {
            continue;
        };
        if !matches!(operator, dir::BinaryOperator::And | dir::BinaryOperator::Or)
            || module.has_builtin_binary_parent(expression, operator)?
        {
            continue;
        }
        let terms = module.short_circuit_operands(expression, operator)?;

        // replace an absorbing constant only when discarded terms are speculatable
        let absorbing = match operator {
            dir::BinaryOperator::And => false,
            dir::BinaryOperator::Or => true,
            _ => continue,
        };
        let has_absorbing = terms
            .iter()
            .any(|term| module.view().get(*term).as_boolean() == Some(absorbing));
        if has_absorbing && all_speculatable(module, &terms)? {
            report_chain(
                module,
                lint,
                expression,
                &[],
                if absorbing { "true" } else { "false" },
                &mut output,
            )?;

            continue;
        }

        // reduce complementary chains only when discarded terms are speculatable
        if has_complementary_terms(module, &terms)? && all_speculatable(module, &terms)? {
            let replacement = match operator {
                dir::BinaryOperator::And => "false",
                dir::BinaryOperator::Or => "true",
                _ => continue,
            };
            report_chain(module, lint, expression, &[], replacement, &mut output)?;

            continue;
        }

        // remove duplicate and absorbed terms while preserving source order
        let retained = retained_terms(module, &terms, operator)?;
        if retained.len() == terms.len() {
            continue;
        }
        let replacement = if retained.is_empty() {
            match operator {
                dir::BinaryOperator::And => "true".to_string(),
                dir::BinaryOperator::Or => "false".to_string(),
                _ => continue,
            }
        } else {
            logical_source(module, &retained, operator)?
        };
        report_chain(
            module,
            lint,
            expression,
            &retained,
            &replacement,
            &mut output,
        )?;
    }

    Ok(output)
}

/// Return whether every term may be evaluated on an additional control-flow path.
fn all_speculatable(
    module: &DirModule<'_>,
    terms: &[dir::LocalNodeId<dir::Expression>],
) -> Result<bool, ProviderError> {
    for term in terms.iter().copied() {
        if !module.is_speculatable_expression(term)? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Return whether every expression may be evaluated twice at the same program point.
fn all_duplicable(
    module: &DirModule<'_>,
    expressions: &[dir::LocalNodeId<dir::Expression>],
) -> Result<bool, ProviderError> {
    for expression in expressions.iter().copied() {
        if !module.is_duplicable_expression(expression)? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Return whether a chain contains one duplicable term and its builtin negation.
fn has_complementary_terms(
    module: &DirModule<'_>,
    terms: &[dir::LocalNodeId<dir::Expression>],
) -> Result<bool, ProviderError> {
    for (index, left) in terms.iter().copied().enumerate() {
        for right in terms[index + 1..].iter().copied() {
            if negated_pair(module, left, right)? || negated_pair(module, right, left)? {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Return whether `negated` is the builtin boolean negation of `value`.
fn negated_pair(
    module: &DirModule<'_>,
    negated: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let dir::Expression::Unary {
        operator: dir::UnaryOperator::Not,
        right,
    } = module.view().get(negated)
    else {
        return Ok(false);
    };
    if module.builtin_unary(negated)?.is_none() {
        return Ok(false);
    }

    module.is_same_computation(*right, value)
}

/// Retain each logical term that contributes to the chain result.
fn retained_terms(
    module: &DirModule<'_>,
    terms: &[dir::LocalNodeId<dir::Expression>],
    operator: dir::BinaryOperator,
) -> Result<Vec<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let mut retained = Vec::new();

    // retain terms that contribute to the chain result
    for (index, term) in terms.iter().copied().enumerate() {
        let identity = match operator {
            dir::BinaryOperator::And => true,
            dir::BinaryOperator::Or => false,
            _ => return Ok(terms.to_vec()),
        };
        if module.view().get(term).as_boolean() == Some(identity) {
            continue;
        }
        if is_term_redundant(module, terms, index, operator)? {
            continue;
        }

        retained.push(term);
    }

    Ok(retained)
}

/// Return whether another chain term makes one duplicable term redundant.
fn is_term_redundant(
    module: &DirModule<'_>,
    terms: &[dir::LocalNodeId<dir::Expression>],
    index: usize,
    operator: dir::BinaryOperator,
) -> Result<bool, ProviderError> {
    let term = terms[index];

    // compare the implication direction selected by the outer operator
    for (other_index, other) in terms.iter().copied().enumerate() {
        if other_index == index {
            continue;
        }

        // reject mutations between the candidate and its logical witness
        let first = index.min(other_index) + 1;
        let end = index.max(other_index);
        if !all_duplicable(module, &terms[first..end])? {
            continue;
        }

        let (premise, conclusion) = match operator {
            dir::BinaryOperator::And => (other, term),
            dir::BinaryOperator::Or => (term, other),
            _ => return Ok(false),
        };
        if !module.boolean_implies(premise, conclusion)? {
            continue;
        }

        // preserve the first of two logically equivalent terms
        let is_equivalent = module.boolean_implies(conclusion, premise)?;
        if !is_equivalent || other_index < index {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Build source for one reduced logical chain.
fn logical_source(
    module: &DirModule<'_>,
    terms: &[dir::LocalNodeId<dir::Expression>],
    operator: dir::BinaryOperator,
) -> Result<String, ProviderError> {
    let mut source = String::new();

    // join retained terms at the original logical precedence
    for (index, term) in terms.iter().copied().enumerate() {
        if index > 0 {
            source.push(' ');
            source.push_str(operator.text());
            source.push(' ');
        }
        let term = module.expression_source(term, operator.precedence())?;
        source.push_str(&term);
    }

    Ok(source)
}

/// Report one nonminimal chain and attach its safe replacement when comments permit.
fn report_chain(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    retained: &[dir::LocalNodeId<dir::Expression>],
    replacement: &str,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let mut diagnostic = lint.diagnostic("boolean expression contains redundant terms", extent);
    if let Some(fix) = chain_fix(module, lint, extent, retained, replacement)? {
        diagnostic = diagnostic.suggestion(fix);
    }
    output.report(diagnostic);

    Ok(())
}

/// Replace one logical chain while retaining comments carried by preserved terms.
fn chain_fix(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: Span,
    retained: &[dir::LocalNodeId<dir::Expression>],
    replacement: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = retained
        .iter()
        .map(|term| module.source_extent(term.into_any()))
        .collect::<Result<Vec<_>, _>>()?;
    if module.has_unretained_comment(extent, &retained)? {
        return Ok(None);
    }

    // replace the complete chain with the reduced expression
    let patch = Patch::replace(extent, replacement);
    let fix = lint.fix("simplify the boolean expression", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Apply boolean absorption to a duplicable term.
    #[test]
    fn test_removes_absorbed_term() {
        let session = TestSession::dir(
            &NONMINIMAL_BOOL,
            r#"
function canRead(isOwner: boolean, isShared: boolean): boolean {
    const any = isOwner || (isOwner && isShared);
    const all = isOwner && (isOwner || isShared);
    return any && all;
}
"#,
        );

        session.assert_fixes(
            r#"
function canRead(isOwner: boolean, isShared: boolean): boolean {
    const any = isOwner;
    const all = isOwner;
    return any && all;
}
"#,
        );
    }

    /// Remove repeated terms from one logical chain.
    #[test]
    fn test_removes_duplicate_term() {
        let session = TestSession::dir(
            &NONMINIMAL_BOOL,
            r#"
function ready(isReady: boolean, isForced: boolean): boolean {
    return isReady || isForced || isReady;
}
"#,
        );

        session.assert_fixes(
            r#"
function ready(isReady: boolean, isForced: boolean): boolean {
    return isReady || isForced;
}
"#,
        );
    }

    /// Remove a duplicate term whose deterministic evaluation may trap.
    #[test]
    fn test_removes_duplicate_trapping_term() {
        let session = TestSession::dir(
            &NONMINIMAL_BOOL,
            r#"
function isPositiveRatio(left: int32, right: int32): boolean {
    return left / right > 0 || left / right > 0;
}
"#,
        );

        session.assert_fixes(
            r#"
function isPositiveRatio(left: int32, right: int32): boolean {
    return left / right > 0;
}
"#,
        );
    }

    /// Remove a duplicate separated by a deterministic trapping term.
    #[test]
    fn test_removes_duplicate_around_trapping_term() {
        let session = TestSession::dir(
            &NONMINIMAL_BOOL,
            r#"
function ready(isReady: boolean, left: int32, right: int32): boolean {
    return isReady || left / right > 0 || isReady;
}
"#,
        );

        session.assert_fixes(
            r#"
function ready(isReady: boolean, left: int32, right: int32): boolean {
    return isReady || left / right > 0;
}
"#,
        );
    }

    /// Remove identity constants and replace absorbing constants.
    #[test]
    fn test_reduces_boolean_constants() {
        let session = TestSession::dir(
            &NONMINIMAL_BOOL,
            r#"
function ready(isReady: boolean): boolean {
    const retained = true && isReady && true;
    const absorbed = isReady || true;
    return retained && absorbed;
}
"#,
        );

        session.assert_fixes(
            r#"
function ready(isReady: boolean): boolean {
    const retained = isReady;
    const absorbed = true;
    return retained && absorbed;
}
"#,
        );
    }

    /// Preserve a trapping term before an absorbing constant.
    #[test]
    fn test_accepts_trapping_absorbed_term() {
        let session = TestSession::dir(
            &NONMINIMAL_BOOL,
            r#"
function isPositiveRatio(left: int32, right: int32): boolean {
    return left / right > 0 || true;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a contradiction with false.
    #[test]
    fn test_replaces_contradiction() {
        let session = TestSession::dir(
            &NONMINIMAL_BOOL,
            r#"
function impossible(isReady: boolean): boolean {
    return isReady && !isReady;
}
"#,
        );

        session.assert_fixes(
            r#"
function impossible(isReady: boolean): boolean {
    return false;
}
"#,
        );
    }

    /// Accept repeated calls whose results may differ.
    #[test]
    fn test_accepts_repeated_calls() {
        let session = TestSession::dir(
            &NONMINIMAL_BOOL,
            r#"
declare function ready(): boolean;

function both(): boolean {
    return ready() && ready();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep repeated reads separated by an effectful term.
    #[test]
    fn test_accepts_repeated_reads_around_effect() {
        let session = TestSession::dir(
            &NONMINIMAL_BOOL,
            r#"
declare function update(): boolean;

function ready(isReady: boolean): boolean {
    return isReady || update() || isReady;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept absorption that would remove an effectful nested term.
    #[test]
    fn test_accepts_effectful_absorption() {
        let session = TestSession::dir(
            &NONMINIMAL_BOOL,
            r#"
declare function observe(): boolean;

function ready(isReady: boolean): boolean {
    return (observe() && isReady) || isReady;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
