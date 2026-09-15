use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

const PREFIXES: &[&str] = &["is", "has", "can", "should", "did", "will"];

declare_lint! {
    /// Require predicate prefixes for boolean values.
    pub BOOLEAN_PREFIX {
        id: "boolean-prefix",
        summary: "Require predicate prefixes for boolean values",
        explanation: r#"
A boolean value represents a predicate, but an unprefixed name does not identify it as one at use sites.
Instead, you SHOULD begin boolean bindings, parameters, fields, and constants with `is`, `has`, `can`, `should`, `did`, or `will`.

Functions and methods are exempt because verbs such as `contains`, `matches`, and `startsWith` already express a predicate.
"#,
        example: {
            reported: r#"
function send(ready: boolean): void {}
"#,
            accepted: r#"
function send(isReady: boolean): void {}
"#,
        },
        provenance: [Unicorn("consistent-boolean-name")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report boolean value symbols without one standard predicate prefix.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect value-bearing declaration symbols through their types
    for (declaration, symbol_id) in module.bindings.declaration_symbols() {
        if declaration.module_id != module.id {
            continue;
        }

        // select local value declarations
        let symbol = module.bindings.get_symbol(symbol_id);
        if !matches!(
            symbol.kind,
            dir::SymbolKind::AssociatedConst
                | dir::SymbolKind::GenericConstParameter
                | dir::SymbolKind::Parameter
                | dir::SymbolKind::Variable
        ) {
            continue;
        }
        let Some(name) = symbol.name() else {
            continue;
        };
        let name = module.dir.strings.get(name);

        // read the declared value or const constraint type
        let symbol_id = symbol_id.into_global(module.id);
        let is_const_parameter = symbol.kind == dir::SymbolKind::GenericConstParameter;
        let type_id = if is_const_parameter {
            let parameter_id = module
                .generics
                .parameter_by_symbol(symbol_id)
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "generic symbol {symbol_id:?} has no generic parameter"
                    ))
                })?;
            let binding = module.generics.get_parameter(parameter_id);
            let Some(constraint) = binding.constraint else {
                continue;
            };

            constraint
        } else {
            let Some(type_id) = module.types.get_symbol_type_id(symbol_id) else {
                return Err(ProviderError::internal(format!(
                    "value symbol `{name}` ({symbol_id:?}) has no reduced type"
                )));
            };

            type_id
        };

        // require a boolean without a predicate prefix
        if !module.dir.get_type(type_id)?.is_boolean() {
            continue;
        }
        if has_predicate_prefix(name) {
            continue;
        }

        // report the declaration name
        let span = module.main_span(declaration.local_id)?;
        let message = format!("boolean value `{name}` needs a predicate prefix");
        output.report(lint.diagnostic(message, span));
    }

    Ok(output)
}

/// Return whether one name starts with a complete predicate word.
fn has_predicate_prefix(name: &str) -> bool {
    PREFIXES.iter().any(|prefix| {
        let Some(rest) = name.strip_prefix(prefix) else {
            return false;
        };

        rest.is_empty() || rest.starts_with(|character: char| character.is_uppercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a boolean field without a predicate prefix.
    #[test]
    fn test_reports_boolean_field() {
        let session = TestSession::dir(
            &BOOLEAN_PREFIX,
            r#"
struct Options {
    enabled: boolean;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[boolean-prefix]: boolean value `enabled` needs a predicate prefix
 ──▶ main.ds:2:5
  │
1 │ struct Options {
2 │     enabled: boolean;
  │     ^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Accept every standard predicate prefix.
    #[test]
    fn test_accepts_predicate_prefixes() {
        let session = TestSession::dir(
            &BOOLEAN_PREFIX,
            r#"
function send(
    isReady: boolean,
    hasCapacity: boolean,
    canRetry: boolean,
    shouldFlush: boolean,
    didConnect: boolean,
    willClose: boolean,
): void {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept boolean-returning function names without a prescribed prefix.
    #[test]
    fn test_accepts_predicate_function_name() {
        let session = TestSession::dir(
            &BOOLEAN_PREFIX,
            r#"
function contains(value: string): boolean {
    return value.length > 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Reject a lowercase word that merely begins with predicate letters.
    #[test]
    fn test_reports_incomplete_predicate_prefix() {
        let session = TestSession::dir(
            &BOOLEAN_PREFIX,
            r#"
function visit(island: boolean): void {}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[boolean-prefix]: boolean value `island` needs a predicate prefix
 ──▶ main.ds:1:16
  │
1 │ function visit(island: boolean): void {}
  │                ^^^^^^
  │
"#,
        );
    }

    /// Report a boolean constant without a predicate prefix.
    #[test]
    fn test_reports_boolean_constant() {
        let session = TestSession::dir(
            &BOOLEAN_PREFIX,
            r#"
const ready: boolean = true;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[boolean-prefix]: boolean value `ready` needs a predicate prefix
 ──▶ main.ds:1:7
  │
1 │ const ready: boolean = true;
  │       ^^^^^
  │
"#,
        );
    }

    /// Report a boolean const parameter without a predicate prefix.
    #[test]
    fn test_reports_boolean_const_parameter() {
        let session = TestSession::dir(
            &BOOLEAN_PREFIX,
            r#"
function choose<const enabled: boolean>(value: int32): int32 {
    return value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[boolean-prefix]: boolean value `enabled` needs a predicate prefix
 ──▶ main.ds:1:23
  │
1 │ function choose<const enabled: boolean>(value: int32): int32 {
  │                       ^^^^^^^
2 │     return value;
3 │ }
  │
"#,
        );
    }

    /// Report a boolean parameter declared by a nested function type.
    #[test]
    fn test_reports_nested_function_parameter() {
        let session = TestSession::dir(
            &BOOLEAN_PREFIX,
            r#"
function schedule(callback: (ready: boolean) => void): void {}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[boolean-prefix]: boolean value `ready` needs a predicate prefix
 ──▶ main.ds:1:30
  │
1 │ function schedule(callback: (ready: boolean) => void): void {}
  │                              ^^^^^
  │
"#,
        );
    }

    /// Accept non-boolean discriminator fields in structural newtype variants.
    #[test]
    fn test_accepts_structural_discriminators() {
        let session = TestSession::dir(
            &BOOLEAN_PREFIX,
            r#"
import { Panic } from "destack:error";

/// Worker exit status observed by the supervising parent.
newtype WorkerExit =
    | { kind: "completed" }
    | { kind: "terminated" }
    | { kind: "panicked"; panic: shared readonly Panic };

/// Worker request failure.
newtype WorkerError =
    | { kind: "terminated" }
    | { kind: "panicked"; panic: shared readonly Panic };
"#,
        );

        session.assert_no_diagnostics();
    }
}
