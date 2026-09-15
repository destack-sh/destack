use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require object property shorthand where equivalent.
    pub OBJECT_SHORTHAND {
        id: "object-shorthand",
        summary: "Require object property shorthand where equivalent",
        explanation: r#"
`{ value: value }` repeats one name for the object key and its referenced binding and is equivalent to `{ value }`.
Instead, you SHOULD use property shorthand.
"#,
        example: {
            reported: r#"
function point(x: int32): { x: int32 } {
    return { x: x };
}
"#,
            accepted: r#"
function point(x: int32): { x: int32 } {
    return { x };
}
"#,
        },
        provenance: [Eslint("object-shorthand")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report object fields that repeat an equally named binding.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored object fields
    for (property, node) in view.iter_nodes::<dir::Property>() {
        let dir::Property::Field {
            name: dir::Name::Identifier(key),
            value,
            is_shorthand: false,
        } = node
        else {
            continue;
        };

        // require an equally named identifier value
        let dir::Expression::Identifier { name } = view.get(*value) else {
            continue;
        };
        if key != name {
            continue;
        }

        // replace the complete field only when no comment would be discarded
        let span = module.source_extent(property.into_any())?;
        let mut diagnostic = lint.diagnostic("object field repeats its binding name", span);
        if let Some(suggestion) = suggestion(module, lint, property, *value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the exact property shorthand replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    property: dir::LocalNodeId<dir::Property>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(property.into_any())?;
    let value = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[value])? {
        return Ok(None);
    }

    // retain the exact identifier spelling
    let patch = Patch::replace(extent, module.source(value)?);
    let suggestion = lint.fix("use property shorthand", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report the shorthand opportunity without deleting a comment.
    #[test]
    fn test_reports_commented_property_without_fix() {
        let session = TestSession::dir(
            &OBJECT_SHORTHAND,
            r#"
function point(x: int32): { x: int32 } {
    return { x: /* retain */ x };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[object-shorthand]: object field repeats its binding name
 ──▶ main.ds:2:14
  │
1 │ function point(x: int32): { x: int32 } {
2 │     return { x: /* retain */ x };
  │              ^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Accept a field whose value comes from a differently named binding.
    #[test]
    fn test_accepts_distinct_binding_name() {
        let session = TestSession::dir(
            &OBJECT_SHORTHAND,
            r#"
function point(value: int32): { x: int32 } {
    return { x: value };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an already shorthand field.
    #[test]
    fn test_accepts_property_shorthand() {
        let session = TestSession::dir(
            &OBJECT_SHORTHAND,
            r#"
function point(x: int32): { x: int32 } {
    return { x };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an explicitly string-named property.
    #[test]
    fn test_accepts_string_named_property() {
        let session = TestSession::dir(
            &OBJECT_SHORTHAND,
            r#"
function point(x: int32): { x: int32 } {
    return { "x": x };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
