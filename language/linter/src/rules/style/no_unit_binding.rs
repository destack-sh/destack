use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow binding unit values.
    pub NO_UNIT_BINDING {
        id: "no-unit-binding",
        summary: "Disallow binding unit values",
        explanation: r#"
A unit binding cannot carry information for later use.
Instead, you SHOULD evaluate the expression as a statement without declaring a binding.
"#,
        example: {
            reported: r#"
function record(): void {}

const result = record();
"#,
            accepted: r#"
function record(): void {}

record();
"#,
        },
        provenance: [Clippy("let_unit_value")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report initialized local bindings whose type is unit.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect initialized declarators with one declared symbol
    for (declarator, value) in view.iter_nodes::<dir::Declarator>() {
        let Some(initializer) = value.value else {
            continue;
        };
        let Some(symbol) = module.sole_declared_symbol(value.pattern.into_any()) else {
            continue;
        };
        let Some(type_id) = module.types.get_symbol_type_id(symbol) else {
            return Err(ProviderError::internal(format!(
                "binding {symbol:?} has no reduced type"
            )));
        };
        if !module.dir.get_type(type_id)?.is_unit() {
            continue;
        }

        // suggest preserving the initializer as a statement when structurally safe
        let span = module.main_span(value.pattern.into_any())?;
        let mut diagnostic = lint.diagnostic("binding has unit type", span);
        if let Some(suggestion) = statement_suggestion(module, lint, declarator, initializer)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one single-declarator binding with its initializer statement.
fn statement_suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    declarator: dir::LocalNodeId<dir::Declarator>,
    initializer: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<destack_source::DiagnosticSuggestion>, ProviderError> {
    let view = module.view();
    let Some(expression) = view.ancestor::<dir::Expression>(declarator.into_any()) else {
        return Ok(None);
    };
    let is_single = match view.get(expression) {
        dir::Expression::Let { declarators, .. } => declarators.as_slice() == [declarator],
        _ => false,
    };
    if !is_single {
        return Ok(None);
    }

    // preserve comments and expression evaluation exactly
    let extent = module.statement_span(expression)?;
    let initializer = module.source_extent(initializer.into_any())?;
    if module.has_unretained_comment(extent, &[initializer])? {
        return Ok(None);
    }
    let replacement = format!("{};", module.source(initializer)?);
    let patch = Patch::replace(extent, replacement);

    lint.fix("evaluate the expression without a binding", patch)
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a void-returning binding with its initializer statement.
    #[test]
    fn test_replaces_void_return_binding() {
        let session = TestSession::dir(
            &NO_UNIT_BINDING,
            r#"
function record(): void {}

const result = record();
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-unit-binding]: binding has unit type
 ──▶ main.ds:3:7
  │
1 │ function record(): void {}
2 │
3 │ const result = record();
  │       ^^^^^^
  │

 = fix: evaluate the expression without a binding
--- a/main.ds
+++ b/main.ds

    2│
-   3│ const result = record();
+   3│ record();
"#,
        );
        session.assert_fixes(
            r#"
function record(): void {}

record();
"#,
        );
    }

    /// Replace an empty-tuple binding with its initializer statement.
    #[test]
    fn test_replaces_empty_tuple_binding() {
        let session = TestSession::dir(
            &NO_UNIT_BINDING,
            r#"
const unit = ();
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-unit-binding]: binding has unit type
 ──▶ main.ds:1:7
  │
1 │ const unit = ();
  │       ^^^^
  │

 = fix: evaluate the expression without a binding
--- a/main.ds
+++ b/main.ds

-   1│ const unit = ();
+   1│ ();
"#,
        );
        session.assert_fixes(
            r#"
();
"#,
        );
    }

    /// Report each unit binding in a multi-declarator statement without rewriting siblings.
    #[test]
    fn test_reports_multiple_void_bindings() {
        let session = TestSession::dir(
            &NO_UNIT_BINDING,
            r#"
function record(): void {}

const first = record(), second = record();
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-unit-binding]: binding has unit type
 ──▶ main.ds:3:7
  │
1 │ function record(): void {}
2 │
3 │ const first = record(), second = record();
  │       ^^^^^
  │

warning[no-unit-binding]: binding has unit type
 ──▶ main.ds:3:25
  │
1 │ function record(): void {}
2 │
3 │ const first = record(), second = record();
  │                         ^^^^^^
  │
"#,
        );
    }

    /// Accept a call result that carries a value.
    #[test]
    fn test_accepts_value_binding() {
        let session = TestSession::dir(
            &NO_UNIT_BINDING,
            r#"
function calculate(): int32 {
    return 4;
}

const result = calculate();
"#,
        );

        session.assert_no_diagnostics();
    }
}
