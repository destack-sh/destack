use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer the weakest access form required by a value's uses.
    pub PREFER_WEAKEST_ACCESS {
        id: "prefer-weakest-access",
        summary: "Prefer the weakest access form required by a value's uses",
        explanation: r#"
A borrowed parameter with stronger access than any checked use grants callers unnecessary authority.
Instead, you SHOULD declare the weakest access sufficient for every use of the parameter.
"#,
        example: {
            reported: r#"
struct Counter {
    value: int32;
}

function read(counter: &Counter): int32 {
    return counter.value;
}
"#,
            accepted: r#"
struct Counter {
    value: int32;
}

function read(counter: &readonly Counter): int32 {
    return counter.value;
}
"#,
        },
        provenance: [Clippy("needless_pass_by_ref_mut")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report borrowed parameters that grant unused access.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.access_occurrences().collect::<Vec<_>>();
    let implementations = module
        .definitions
        .member_conformances()
        .map(|conformance| conformance.member)
        .collect::<FxIndexSet<_>>();
    let mut output = LintOutput::default();

    // inspect explicitly typed borrowed parameters with callable bodies
    for (parameter, node) in view.iter_nodes::<dir::Parameter>() {
        let Some(declared_type) = node.declared_type() else {
            continue;
        };
        let dir::TypeExpression::BorrowedOf {
            lifetime,
            mutability,
            target_type,
            ..
        } = view.get(declared_type)
        else {
            continue;
        };
        let declared_access = mutability.unwrap_or(dir::Mutability::Mutable).access();
        if declared_access == dir::Access::Readonly {
            continue;
        }
        let Some(callable) = module.enclosing_callable(parameter.into_any()) else {
            continue;
        };
        let Some(body) = module.callable_body(callable) else {
            continue;
        };

        // leave unfinished bodies until their required access is known
        if let Some(expression) = module.sole_expression(body)
            && matches!(view.get(expression), dir::Expression::Call { .. })
            && module.language_item(expression)? == Some(dir::LanguageItem::Todo)
        {
            continue;
        }

        // preserve signatures imposed by inheritance or conformance
        if let Ok(member) = callable.try_into_typed::<dir::Member>() {
            let dir::Member::Method { signature, .. } = view.get(member) else {
                continue;
            };
            let symbol = module.declaration_symbol(member)?;
            if signature.is_override || implementations.contains(&symbol) {
                continue;
            }
        }

        // combine the access required by every binding in the parameter
        let mut has_binding = false;
        let mut required = dir::Access::Readonly;
        for symbol in module.symbols_declared_within(parameter.into_any()) {
            has_binding = true;
            let access = module.weakest_binding_access(symbol, body.into_any(), &occurrences)?;
            if access.grants(required) {
                required = access;
            }
        }
        if !has_binding || required == declared_access || !declared_access.grants(required) {
            continue;
        }

        // replace only the authored access keyword
        let span = access_span(module, declared_type, *target_type, declared_access)?;
        let diagnostic = lint
            .diagnostic(
                format!(
                    "parameter grants {} access but only {} access is used",
                    declared_access.text(),
                    required.text()
                ),
                span,
            )
            .suggestion(suggestion(module, lint, declared_type, *lifetime)?);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the authored access keyword within one borrowed type prefix.
fn access_span(
    module: &DirModule<'_>,
    borrowed: dir::LocalNodeId<dir::TypeExpression>,
    target: dir::LocalNodeId<dir::TypeExpression>,
    access: dir::Access,
) -> Result<Span, ProviderError> {
    let borrowed = module.source_extent(borrowed.into_any())?;
    let target = module.source_extent(target.into_any())?;
    let prefix = Span::new(borrowed.file, borrowed.start, target.start);
    if access == dir::Access::Mutable {
        return Ok(prefix);
    }
    let source = module.source(prefix)?;
    let keyword = access.text();
    let start = source.find(keyword).ok_or_else(|| {
        ProviderError::internal(format!(
            "borrowed type with {access:?} access has no authored `{keyword}` keyword"
        ))
    })? as u32;

    Ok(Span::new(
        prefix.file,
        prefix.start + start,
        prefix.start + start + keyword.len() as u32,
    ))
}

/// Build the readonly access annotation.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    borrowed: dir::LocalNodeId<dir::TypeExpression>,
    lifetime: Option<dir::LocalNodeId<dir::TypeExpression>>,
) -> Result<DiagnosticSuggestion, ProviderError> {
    let borrowed = module.source_extent(borrowed.into_any())?;
    let (position, text) = if let Some(lifetime) = lifetime {
        let lifetime = module.source_extent(lifetime.into_any())?;

        (lifetime.end, " readonly")
    } else {
        (borrowed.start + 1, "readonly ")
    };

    lint.suggestion(
        "use readonly access".to_string(),
        Patch::insert(borrowed.file, position, text),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept mutable access for projected writes.
    #[test]
    fn test_accepts_a_field_increment() {
        let session = TestSession::dir(
            &PREFER_WEAKEST_ACCESS,
            r#"
struct Counter {
    value: int32;
}

function increment(counter: &Counter): void {
    counter.value += 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Add readonly access to an unqualified mutable borrow.
    #[test]
    fn test_replaces_mutable_with_readonly() {
        let session = TestSession::dir(
            &PREFER_WEAKEST_ACCESS,
            r#"
struct Counter {
    value: int32;
}

function read(counter: &Counter): int32 {
    return counter.value;
}
"#,
        );

        session.assert_suggestions(
            r#"
struct Counter {
    value: int32;
}

function read(counter: &readonly Counter): int32 {
    return counter.value;
}
"#,
        );
    }

    /// Preserve exclusive access used to replace the borrowed place.
    #[test]
    fn test_accepts_a_mutable_replacement() {
        let session = TestSession::dir(
            &PREFER_WEAKEST_ACCESS,
            r#"
struct Counter {
    value: int32;
}

function replace(counter: &Counter, replacement: Counter): void {
    *counter = replacement;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve exclusive access passed to another exclusive parameter.
    #[test]
    fn test_accepts_a_mutable_call() {
        let session = TestSession::dir(
            &PREFER_WEAKEST_ACCESS,
            r#"
struct Counter {
    value: int32;
}

declare function replace(counter: &Counter): void;

function forward(counter: &Counter): void {
    replace(counter);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve exclusive access returned from the callable.
    #[test]
    fn test_accepts_a_returned_mutable_borrow() {
        let session = TestSession::dir(
            &PREFER_WEAKEST_ACCESS,
            r#"
struct Counter {
    value: int32;
}

function identity<'a>(counter: &'a Counter): &'a Counter {
    return counter;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve exclusive access used by a returned function.
    #[test]
    fn test_accepts_a_captured_mutable_borrow() {
        let session = TestSession::dir(
            &PREFER_WEAKEST_ACCESS,
            r#"
struct Counter {
    value: int32;
}

function replaceLater<'a>(counter: &'a Counter): () => void {
    return () => {
        *counter = Counter { value: 0 };
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve access while a callable body remains explicitly unfinished.
    #[test]
    fn test_accepts_todo_body() {
        let session = TestSession::dir(
            &PREFER_WEAKEST_ACCESS,
            r#"
import { todo } from "destack:error";

struct Counter {
    value: int32;
}

function increment(counter: &Counter): void {
    todo("increment");
}

export extension of Counter {
    /// Replace the counter value.
    set(&this, value: int32): void {
        todo("Counter.set");
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
