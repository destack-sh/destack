use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer dot notation for statically named member access.
    pub DOT_NOTATION {
        id: "dot-notation",
        summary: "Prefer dot notation for statically named member access",
        explanation: r#"
A string-literal key is static, so bracket notation presents it as computed member access.
Instead, you SHOULD use dot notation for that member access.
"#,
        example: {
            reported: r#"
function name(user: { name: string }): string {
    return user["name"];
}
"#,
            accepted: r#"
function name(user: { name: string }): string {
    return user.name;
}
"#,
        },
        provenance: [Eslint("dot-notation"), TypeScriptEslint("dot-notation")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report bracket access that checking resolved as a named member.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect static string subscripts
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Index {
            left,
            index: Some(index),
            is_optional,
            ..
        } = node
        else {
            continue;
        };

        // require a static identifier key
        let Some(dir::StaticKey::Name(key)) = view.get(*index).static_key() else {
            continue;
        };
        let name = module.dir.strings.get(key);
        if !dir::is_identifier(name) {
            continue;
        }

        // require named-member lookup on every selected arm
        let Some(resolution) = module.subscript_decision(expression)? else {
            continue;
        };
        let is_member = resolution.arms().iter().all(|subscript| {
            let dir::SubscriptTarget::Member(access) = &subscript.target else {
                return false;
            };

            is_named_member(&access.target)
        });
        if !is_member {
            continue;
        }

        // retain the receiver and replace the bracket suffix
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("static member uses bracket notation", span);
        if let Some(suggestion) = suggestion(module, lint, expression, *left, name, *is_optional)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one member target comes from named member lookup.
fn is_named_member(target: &dir::MemberTarget) -> bool {
    match target {
        dir::MemberTarget::Index(_) => false,
        dir::MemberTarget::OverloadSet(targets) | dir::MemberTarget::Intersection(targets) => {
            !targets.is_empty() && targets.iter().all(is_named_member)
        }
        dir::MemberTarget::Projection { .. }
        | dir::MemberTarget::Field(_)
        | dir::MemberTarget::Call(_)
        | dir::MemberTarget::Symbol(_) => true,
    }
}

/// Build the exact dot access replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
    name: &str,
    is_optional: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let extent = module
        .source_parentheses(receiver.into_any())
        .map_or(extent, |parentheses| extent.merge(parentheses));
    let receiver_span = module.source_extent(receiver.into_any())?;
    if module.has_unretained_comment(extent, &[receiver_span])? {
        return Ok(None);
    }

    // preserve the exact receiver expression
    let operator = if is_optional { "?." } else { "." };
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Postfix)?;
    let replacement = format!("{receiver}{operator}{name}");
    let patch = Patch::replace(extent, replacement);

    let suggestion = lint.fix("use dot notation", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Preserve optional member access when replacing bracket notation.
    #[test]
    fn test_replaces_optional_bracket_notation() {
        let session = TestSession::dir(
            &DOT_NOTATION,
            r#"
function name(user: { name: string } | null): string | undefined {
    return user?.["name"];
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[dot-notation]: static member uses bracket notation
 ──▶ main.tspp:2:12
  │
1 │ function name(user: { name: string } | null): string | undefined {
2 │     return user?.["name"];
  │            ^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use dot notation
--- a/main.tspp
+++ b/main.tspp

    1│ function name(user: { name: string } | null): string | undefined {
-   2│     return user?.["name"];
+   2│     return user?.name;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function name(user: { name: string } | null): string | undefined {
    return user?.name;
}
"#,
        );
    }

    /// Accept a computed key selected through an index signature.
    #[test]
    fn test_accepts_computed_index_signature() {
        let session = TestSession::dir(
            &DOT_NOTATION,
            r#"
function read(values: { [key: string]: int32 }): int32 | undefined {
    return values["item"];
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a property name that is not an identifier.
    #[test]
    fn test_accepts_non_identifier_property() {
        let session = TestSession::dir(
            &DOT_NOTATION,
            r#"
function read(values: { "item-name": int32 }): int32 {
    return values["item-name"];
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve comments inside bracket access by omitting the fix.
    #[test]
    fn test_reports_commented_bracket_access_without_fix() {
        let session = TestSession::dir(
            &DOT_NOTATION,
            r#"
function name(user: { name: string }): string {
    return user[/* retain */ "name"];
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[dot-notation]: static member uses bracket notation
 ──▶ main.tspp:2:12
  │
1 │ function name(user: { name: string }): string {
2 │     return user[/* retain */ "name"];
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Preserve grouping around a low-precedence receiver.
    #[test]
    fn test_preserves_grouped_receiver() {
        let session = TestSession::dir(
            &DOT_NOTATION,
            r#"
function name(isPrimary: boolean): string {
    return (isPrimary ? { name: "first" } : { name: "second" })["name"];
}
"#,
        );

        session.assert_fixes(
            r#"
function name(isPrimary: boolean): string {
    return (isPrimary ? { name: "first" } : { name: "second" }).name;
}
"#,
        );
    }
}
