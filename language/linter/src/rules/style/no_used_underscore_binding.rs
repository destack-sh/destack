use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow using bindings named as unused.
    pub NO_USED_UNDERSCORE_BINDING {
        id: "no-used-underscore-binding",
        summary: "Disallow using bindings named as unused",
        explanation: r#"
A leading underscore declares that a binding is intentionally unused, so later uses contradict its name.
Instead, you SHOULD remove the underscore or remove every use of the binding.
"#,
        example: {
            reported: r#"
function double(_value: int32): int32 {
    return _value * 2;
}
"#,
            accepted: r#"
function double(value: int32): int32 {
    return value * 2;
}
"#,
        },
        provenance: [Clippy("used_underscore_binding")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report underscore-prefixed bindings with recorded uses.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect each used local binding once
    for (symbol_id, uses) in module.flows.binding_uses() {
        if uses.is_empty() {
            continue;
        }
        let symbol = module.bindings.get_symbol(symbol_id);
        let (Some(name), Some(declaration)) = (symbol.name(), symbol.declaration) else {
            continue;
        };
        let name = module.dir.strings.get(name);
        if !name.starts_with('_') {
            continue;
        }

        // report only value bindings whose names promise nonuse
        if !matches!(
            symbol.kind,
            dir::SymbolKind::Parameter | dir::SymbolKind::Variable
        ) {
            continue;
        }
        let span = module.main_span(declaration.local_id)?;
        let message = format!("underscore-prefixed binding `{name}` is used");
        output.report(lint.diagnostic(message, span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report one used underscore-prefixed local binding.
    #[test]
    fn test_reports_used_local_binding() {
        let session = TestSession::dir(
            &NO_USED_UNDERSCORE_BINDING,
            r#"
function calculate(): int32 {
    const _value = 4;
    return _value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-used-underscore-binding]: underscore-prefixed binding `_value` is used
 ──▶ main.tspp:2:11
  │
1 │ function calculate(): int32 {
2 │     const _value = 4;
  │           ^^^^^^
3 │     return _value;
4 │ }
  │
"#,
        );
    }

    /// Report a used underscore-prefixed parameter.
    #[test]
    fn test_reports_used_parameter() {
        let session = TestSession::dir(
            &NO_USED_UNDERSCORE_BINDING,
            r#"
function identity(_value: int32): int32 {
    return _value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-used-underscore-binding]: underscore-prefixed binding `_value` is used
 ──▶ main.tspp:1:19
  │
1 │ function identity(_value: int32): int32 {
  │                   ^^^^^^
2 │     return _value;
3 │ }
  │
"#,
        );
    }

    /// Accept an underscore-prefixed binding without uses.
    #[test]
    fn test_accepts_unused_binding() {
        let session = TestSession::dir(
            &NO_USED_UNDERSCORE_BINDING,
            r#"
function observe(_value: int32): void {}
"#,
        );

        session.assert_no_diagnostics();
    }
}
