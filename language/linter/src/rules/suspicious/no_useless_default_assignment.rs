use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow defaults that cannot be selected by the input type.
    pub NO_USELESS_DEFAULT_ASSIGNMENT {
        id: "no-useless-default-assignment",
        summary: "Disallow defaults that cannot be selected by the input type",
        explanation: r#"
A destructuring default can only run when its selected value is `undefined`.
Instead, you SHOULD remove the default when the input type excludes `undefined`.
"#,
        example: {
            reported: r#"
function name(user: { name: string }): string {
    const { name = "anonymous" } = user;
    return name;
}
"#,
            accepted: r#"
function name(user: { name: string }): string {
    const { name } = user;
    return name;
}
"#,
        },
        provenance: [TypeScriptEslint("no-useless-default-assignment")],
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report default patterns whose input excludes undefined.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect binding and matching defaults
    for (pattern, node) in view.iter_nodes::<dir::Pattern>() {
        let dir::Pattern::Default {
            pattern: nested,
            value,
        } = node
        else {
            continue;
        };

        report_default(
            module,
            lint,
            pattern.into_any(),
            nested.into_any(),
            *value,
            &mut output,
        )?;
    }

    // inspect destructuring assignment defaults
    for (pattern, node) in view.iter_nodes::<dir::AssignPattern>() {
        let dir::AssignPattern::Default {
            pattern: nested,
            value,
        } = node
        else {
            continue;
        };

        report_default(
            module,
            lint,
            pattern.into_any(),
            nested.into_any(),
            *value,
            &mut output,
        )?;
    }

    Ok(output)
}

/// Report one default whose input excludes undefined.
fn report_default(
    module: &DirModule<'_>,
    lint: &Lint,
    pattern: dir::LocalNodeIdAny,
    nested: dir::LocalNodeIdAny,
    value: dir::LocalNodeId<dir::Expression>,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let input = module.node_type_id(pattern)?;
    if module.dir.type_includes_undefined(input)? {
        return Ok(());
    }

    // replace the complete default with its nested pattern
    let nested_extent = module.source_extent(nested)?;
    let value_extent = module.source_extent(value.into_any())?;
    let extent = nested_extent.merge(value_extent);
    let mut diagnostic = lint.diagnostic("default cannot be selected by this type", extent);
    if let Some(fix) = fix(module, lint, extent, nested)? {
        diagnostic = diagnostic.suggestion(fix);
    }
    output.report(diagnostic);

    Ok(())
}

/// Build a replacement that retains the nested pattern.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: Span,
    nested: dir::LocalNodeIdAny,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(nested)?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain the exact nested pattern source
    let patch = Patch::replace(extent, module.source(retained)?);
    let fix = lint.fix("remove the unreachable default", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove a default from a required object field.
    #[test]
    fn test_replaces_required_object_field_default() {
        let session = TestSession::dir(
            &NO_USELESS_DEFAULT_ASSIGNMENT,
            r#"
function name(user: { name: string }): string {
    const { name = "anonymous" } = user;
    return name;
}
"#,
        );

        session.assert_fixes(
            r#"
function name(user: { name: string }): string {
    const { name } = user;
    return name;
}
"#,
        );
    }

    /// Remove a default from a fixed-array assignment target.
    #[test]
    fn test_replaces_fixed_array_assignment_default() {
        let session = TestSession::dir(
            &NO_USELESS_DEFAULT_ASSIGNMENT,
            r#"
function read(values: [int32; 1]): int32 {
    let value: int32 = 0;
    [value = 1] = values;
    return value;
}
"#,
        );

        session.assert_fixes(
            r#"
function read(values: [int32; 1]): int32 {
    let value: int32 = 0;
    [value] = values;
    return value;
}
"#,
        );
    }

    /// Keep a default for an optional object field.
    #[test]
    fn test_accepts_optional_object_field_default() {
        let session = TestSession::dir(
            &NO_USELESS_DEFAULT_ASSIGNMENT,
            r#"
function name(user: { name?: string }): string {
    const { name = "anonymous" } = user;
    return name;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep a default for an explicitly undefined union arm.
    #[test]
    fn test_accepts_undefined_union_default() {
        let session = TestSession::dir(
            &NO_USELESS_DEFAULT_ASSIGNMENT,
            r#"
function name(user: { name: string | undefined }): string {
    const { name = "anonymous" } = user;
    return name;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a comment attached to the removed default by omitting the fix.
    #[test]
    fn test_reports_commented_default_without_fix() {
        let session = TestSession::dir(
            &NO_USELESS_DEFAULT_ASSIGNMENT,
            r#"
function name(user: { name: string }): string {
    const { name = /* retain */ "anonymous" } = user;
    return name;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-default-assignment]: default cannot be selected by this type
 ──▶ main.ds:2:13
  │
1 │ function name(user: { name: string }): string {
2 │     const { name = /* retain */ "anonymous" } = user;
  │             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │     return name;
4 │ }
  │
"#,
        );
    }
}
