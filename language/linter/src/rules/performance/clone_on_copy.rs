use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow cloning values whose type is Copy.
    pub CLONE_ON_COPY {
        id: "clone-on-copy",
        summary: "Disallow cloning values whose type is Copy",
        explanation: r#"
Calling `clone` on a Copy value implies that duplication may require explicit work.
Instead, you SHOULD use the value directly and let Copy semantics duplicate it implicitly.
"#,
        example: {
            reported: r#"
function duplicate(value: int32): int32 {
    return value.clone();
}
"#,
            accepted: r#"
function duplicate(value: int32): int32 {
    return value;
}
"#,
        },
        provenance: [Clippy("clone_on_copy")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical clone calls on Copy values.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical zero-argument Clone.clone calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if !call.arguments.is_empty()
            || module.implemented_language_member(expression)?
                != Some(dir::LanguageItem::Clone.member("clone"))
        {
            continue;
        }

        // require the cloned value itself to satisfy Copy
        let receiver_type = module.adjusted_type_id(call.receiver.into_any())?;
        if !module.satisfies_copy(receiver_type)? {
            continue;
        }

        // remove the unnecessary clone call
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("Copy value is cloned explicitly", span);
        if let Some(suggestion) = suggestion(module, lint, expression, call.receiver)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Remove one clone suffix while retaining its receiver.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver_extent = module.source_extent(receiver.into_any())?;
    if !extent.contains_span(receiver_extent) {
        return Err(ProviderError::internal(
            "clone extent does not contain its receiver",
        ));
    }
    if module.has_unretained_comment(extent, &[receiver_extent])? {
        return Ok(None);
    }

    // retain the complete receiver with the expression's required precedence
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Lowest)?;
    let patch = Patch::replace(extent, receiver);
    let suggestion = lint.fix("use the Copy value directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove clone from a generic value constrained by Copy.
    #[test]
    fn test_removes_generic_copy_clone() {
        let session = TestSession::dir(
            &CLONE_ON_COPY,
            r#"
function duplicate<T: Copy>(value: T): T {
    return value.clone();
}
"#,
        );

        session.assert_fixes(
            r#"
function duplicate<T: Copy>(value: T): T {
    return value;
}
"#,
        );
    }

    /// Accept clone on a non-Copy value.
    #[test]
    fn test_accepts_noncopy_clone() {
        let session = TestSession::dir(
            &CLONE_ON_COPY,
            r#"
import { rc } from "destack:memory";

function duplicate(value: rc.Rc<int32>): rc.Rc<int32> {
    return value.clone();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve an optional Copy receiver while removing clone.
    #[test]
    fn test_removes_optional_copy_clone() {
        let session = TestSession::dir(
            &CLONE_ON_COPY,
            r#"
function duplicate(value: int32 | undefined): int32 | undefined {
    return value?.clone();
}
"#,
        );

        session.assert_fixes(
            r#"
function duplicate(value: int32 | undefined): int32 | undefined {
    return value;
}
"#,
        );
    }

    /// Preserve comments inside a removed clone call.
    #[test]
    fn test_reports_commented_clone_without_fix() {
        let session = TestSession::dir(
            &CLONE_ON_COPY,
            r#"
function duplicate(value: int32): int32 {
    return value.clone(/* retain */);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[clone-on-copy]: Copy value is cloned explicitly
 ──▶ main.ds:2:12
  │
1 │ function duplicate(value: int32): int32 {
2 │     return value.clone(/* retain */);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
