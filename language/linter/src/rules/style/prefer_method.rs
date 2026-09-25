use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer methods when the first parameter establishes a receiver.
    pub PREFER_METHOD {
        id: "prefer-method",
        summary: "Prefer methods when the first parameter establishes a receiver",
        explanation: r#"
A function whose first parameter supplies the operation's canonical nominal owner hides that operation from member lookup.
Instead, you SHOULD declare an inherent or extension method.

Symmetric operations, constructors, decorated functions, and ambient declarations remain free functions.
"#,
        example: {
            reported: r#"
struct Counter {
    value: int32;
}

function increment(counter: &Counter): void {
    counter.value += 1;
}
"#,
            accepted: r#"
struct Counter {
    value: int32;
}

extension of Counter {
    increment(&this): void {
        this.value += 1;
    }
}
"#,
        },
        provenance: [],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report free functions whose first parameter establishes one nominal receiver.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect concrete ordinary function declarations
    for (node, declaration) in view.iter_nodes::<dir::Declaration>() {
        let dir::Declaration::Function(function) = declaration else {
            continue;
        };
        if function.body.is_none()
            || function.is_ambient
            || function.signature.role.is_some()
            || view.has_decorators_any(node.into_any())
        {
            continue;
        }
        let Some(first) = function.signature.parameters.first().copied() else {
            continue;
        };
        let Some(owner) = parameter_owner(first, module)? else {
            continue;
        };

        // preserve operations whose parameters establish no single receiver
        let mut is_symmetric = false;
        for parameter in function.signature.parameters[1..].iter().copied() {
            if parameter_owner(parameter, module)? == Some(owner) {
                is_symmetric = true;
                break;
            }
        }
        if is_symmetric {
            continue;
        }

        let span = module.main_span(node.into_any())?;
        output.report(lint.diagnostic(
            "first parameter establishes the function's nominal receiver",
            span,
        ));
    }

    Ok(output)
}

/// Return the nominal type beneath one parameter's memory form.
fn parameter_owner(
    parameter: dir::LocalNodeId<dir::Parameter>,
    module: &DirModule<'_>,
) -> Result<Option<dir::GlobalSymbolId>, ProviderError> {
    let Some(declared_type) = module.view().get(parameter).declared_type() else {
        return Ok(None);
    };
    let type_id = module.node_type_id(declared_type.into_any())?;
    let type_id = module.dir.strip_form(type_id)?;
    let Some(symbol) = module.dir.get_type(type_id)?.symbol() else {
        return Ok(None);
    };
    if !module.dir.is_nominal_symbol(symbol)? {
        return Ok(None);
    }

    Ok(Some(symbol))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a free operation owned by its first nominal parameter.
    #[test]
    fn test_reports_receiver_function() {
        let session = TestSession::dir(
            &PREFER_METHOD,
            r#"
struct Counter {
    value: int32;
}

function increment(counter: &Counter): void {
    counter.value += 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-method]: first parameter establishes the function's nominal receiver
 ──▶ main.tspp:5:10
  │
3 │ }
4 │
5 │ function increment(counter: &Counter): void {
  │          ^^^^^^^^^
6 │     counter.value += 1;
7 │ }
  │
"#,
        );
    }

    /// Accept a symmetric operation over two values of the same nominal type.
    #[test]
    fn test_accepts_symmetric_function() {
        let session = TestSession::dir(
            &PREFER_METHOD,
            r#"
struct Point {
    x: int32;
}

function distance(left: &readonly Point, right: &readonly Point): int32 {
    return right.x - left.x;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
