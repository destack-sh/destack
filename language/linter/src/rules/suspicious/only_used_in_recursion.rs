use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow parameters that only contribute to corresponding recursive arguments.
    pub ONLY_USED_IN_RECURSION {
        id: "only-used-in-recursion",
        summary: "Disallow parameters used only to calculate their own recursive argument",
        explanation: r#"
A parameter read only to calculate its corresponding recursive argument does not affect the result.
Instead, you SHOULD remove the parameter and its argument from every recursive call.
"#,
        example: {
            reported: r#"
function depth(remaining: int32, unused: int32): int32 {
    if (remaining === 0) {
        return 0;
    }

    return depth(remaining - 1, unused);
}
"#,
            accepted: r#"
function depth(remaining: int32): int32 {
    if (remaining === 0) {
        return 0;
    }

    return depth(remaining - 1);
}
"#,
        },
        provenance: [
            Clippy("only_used_in_recursion"),
            Oxc("only-used-in-recursion"),
        ],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report parameters whose recorded reads only feed recursive arguments.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect free function declarations
    for (declaration_id, declaration) in view.iter_nodes::<dir::Declaration>() {
        let dir::Declaration::Function(function) = declaration else {
            continue;
        };
        let Some(body) = function.body else {
            continue;
        };
        let symbol = module.declaration_symbol(declaration_id)?;
        report_parameters(
            module,
            lint,
            symbol,
            &function.signature,
            body,
            &occurrences,
            &mut output,
        )?;
    }

    // inspect named declaration members
    for (member_id, member) in view.iter_nodes::<dir::Member>() {
        let dir::Member::Method {
            name: Some(_),
            signature,
            body: Some(body),
            ..
        } = member
        else {
            continue;
        };
        let symbol = module.declaration_symbol(member_id)?;
        report_parameters(
            module,
            lint,
            symbol,
            signature,
            *body,
            &occurrences,
            &mut output,
        )?;
    }

    Ok(output)
}

/// Report qualifying parameters from one callable body.
fn report_parameters(
    module: &DirModule<'_>,
    lint: &Lint,
    callable: dir::GlobalSymbolId,
    signature: &dir::FunctionSignature,
    body: dir::LocalNodeId<dir::Expression>,
    occurrences: &[dir::BindingOccurrence],
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    for (position, parameter) in signature.parameters.iter().enumerate() {
        // pattern bindings do not represent the complete argument position
        if module.view().get(*parameter).name().is_none() {
            continue;
        }

        // compare every use with its recursive argument position
        let symbol = module.declaration_symbol(*parameter)?;
        let mut uses = occurrences
            .iter()
            .filter(|occurrence| occurrence.symbol == symbol);
        let Some(first) = uses.next() else {
            continue;
        };

        // require every occurrence to feed a uniquely resolved recursive call
        let mut is_recursive_only = is_recursive_argument(module, callable, body, position, first)?;
        for occurrence in uses {
            if !is_recursive_argument(module, callable, body, position, occurrence)? {
                is_recursive_only = false;
                break;
            }
        }
        if !is_recursive_only {
            continue;
        }

        let span = module.main_span(parameter.into_any())?;
        output
            .report(lint.diagnostic("parameter only contributes to its recursive argument", span));
    }

    Ok(())
}

/// Return whether one occurrence feeds a speculatable argument to the same callable.
fn is_recursive_argument(
    module: &DirModule<'_>,
    callable: dir::GlobalSymbolId,
    body: dir::LocalNodeId<dir::Expression>,
    position: usize,
    occurrence: &dir::BindingOccurrence,
) -> Result<bool, ProviderError> {
    let view = module.view();
    if !view.is_inside(occurrence.node, body.into_any()) {
        return Ok(false);
    }
    let Some(argument) = view.ancestor::<dir::Argument>(occurrence.node) else {
        return Ok(false);
    };
    let Some(call) = view.ancestor::<dir::Expression>(argument.into_any()) else {
        return Ok(false);
    };
    if !matches!(view.get(call), dir::Expression::Call { .. }) {
        return Ok(false);
    }
    let Some(decision) = module.call_decision(call)? else {
        return Ok(false);
    };
    if decision.target_symbols().as_slice() != [callable] {
        return Ok(false);
    }
    let dir::Expression::Call { arguments, .. } = view.get(call) else {
        return Ok(false);
    };
    if arguments.get(position) != Some(&argument) {
        return Ok(false);
    }
    let dir::Argument::Positional { value } = view.get(argument) else {
        return Ok(false);
    };

    module.is_speculatable_expression(*value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a parameter forwarded unchanged through recursion.
    #[test]
    fn test_reports_forwarded_parameter() {
        let session = TestSession::dir(
            &ONLY_USED_IN_RECURSION,
            r#"
function depth(remaining: int32, unused: int32): int32 {
    if (remaining === 0) {
        return 0;
    }

    return depth(remaining - 1, unused);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[only-used-in-recursion]: parameter only contributes to its recursive argument
 ──▶ main.tspp:1:34
  │
1 │ function depth(remaining: int32, unused: int32): int32 {
  │                                  ^^^^^^
2 │     if (remaining === 0) {
3 │         return 0;
  │
"#,
        );
    }

    /// Accept a parameter observed before the recursive call.
    #[test]
    fn test_accepts_observed_parameter() {
        let session = TestSession::dir(
            &ONLY_USED_IN_RECURSION,
            r#"
function depth(remaining: int32, offset: int32): int32 {
    if (remaining === 0) {
        return offset;
    }

    return depth(remaining - 1, offset);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a parameter used by an effectful recursive argument.
    #[test]
    fn test_accepts_effectful_argument() {
        let session = TestSession::dir(
            &ONLY_USED_IN_RECURSION,
            r#"
declare function observe(value: int32): int32;

function depth(remaining: int32, value: int32): int32 {
    if (remaining === 0) {
        return 0;
    }

    return depth(remaining - 1, observe(value));
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a parameter forwarded into another recursive parameter.
    #[test]
    fn test_accepts_permuted_parameter() {
        let session = TestSession::dir(
            &ONLY_USED_IN_RECURSION,
            r#"
function reachesZero(current: int32, next: int32): boolean {
    if (current === 0) {
        return true;
    }

    return reachesZero(next, current);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a method parameter only forwarded to the same method.
    #[test]
    fn test_reports_recursive_method_parameter() {
        let session = TestSession::dir(
            &ONLY_USED_IN_RECURSION,
            r#"
class Counter {
    depth(remaining: int32, unused: int32): int32 {
        if (remaining === 0) {
            return 0;
        }

        return this.depth(remaining - 1, unused);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[only-used-in-recursion]: parameter only contributes to its recursive argument
 ──▶ main.tspp:2:29
  │
1 │ class Counter {
2 │     depth(remaining: int32, unused: int32): int32 {
  │                             ^^^^^^
3 │         if (remaining === 0) {
4 │             return 0;
  │
"#,
        );
    }

    /// Accept pattern parameters that introduce independently used bindings.
    #[test]
    fn test_accepts_pattern_parameter() {
        let session = TestSession::dir(
            &ONLY_USED_IN_RECURSION,
            r#"
function visit([head, ...tail]: int32[]): int32 {
    if (tail.isEmpty) {
        return head;
    }

    return visit(tail);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
