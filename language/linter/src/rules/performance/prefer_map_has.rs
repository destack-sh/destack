use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Map.has over Map.get when only key presence is observed.
    pub PREFER_MAP_HAS {
        id: "prefer-map-has",
        summary: "Prefer Map.has over Map.get when only key presence is observed",
        explanation: r#"
Comparing `Map.get` with `undefined` retrieves a value only to determine whether its key exists.
Instead, you SHOULD call `Map.has` when the mapped value cannot itself be `undefined`.
"#,
        example: {
            reported: r#"
function contains(values: Map<string, int32>, key: string): boolean {
    return values.get(key) !== undefined;
}
"#,
            accepted: r#"
function contains(values: Map<string, int32>, key: string): boolean {
    return values.has(key);
}
"#,
        },
        provenance: [Unicorn("prefer-has-check")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical map lookups used only as presence tests.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect strict equality comparisons against undefined
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((operator, operands)) = module.builtin_binary(expression)? else {
            continue;
        };
        if !operator.is_strict_equality() {
            continue;
        }
        let [left, right] = operands;
        let (lookup, undefined) = if module.scalar_constant(left.source.local_id)?
            == Some(dir::Literal::Undefined)
        {
            (right.source.local_id, left.source.local_id)
        } else if module.scalar_constant(right.source.local_id)? == Some(dir::Literal::Undefined) {
            (left.source.local_id, right.source.local_id)
        } else {
            continue;
        };

        // require one canonical Map.get whose value excludes undefined
        let Some(call) = module.member_call(lookup) else {
            continue;
        };
        if call.is_optional()
            || module.language_member(lookup)? != Some(dir::LanguageItem::Map.member("get"))
        {
            continue;
        }
        let Some(bindings) = module.call_generic_bindings(lookup)? else {
            continue;
        };
        let [_, value, ..] = bindings else {
            return Err(ProviderError::internal(
                "Map.get call has fewer than two generic bindings",
            ));
        };
        if module.dir.type_includes_undefined(value.argument)? {
            continue;
        }

        // replace the lookup comparison with a presence query
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("map value is retrieved only to test its key", span);
        if let Some(fix) = fix(
            module,
            lint,
            expression,
            lookup,
            undefined,
            call.callee,
            operator.is_negative_equality(),
        )? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one Map.get comparison with Map.has.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    lookup: dir::LocalNodeId<dir::Expression>,
    undefined: dir::LocalNodeId<dir::Expression>,
    callee: dir::LocalNodeId<dir::Expression>,
    is_present: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let lookup = module.source_extent(lookup.into_any())?;
    let undefined = module.source_extent(undefined.into_any())?;
    if module.has_unretained_comment(extent, &[lookup, undefined])? {
        return Ok(None);
    }

    // retain the complete call and remove the surrounding comparison
    let mut patch = FilePatch::new(extent.file);
    let prefix = Span::new(extent.file, extent.start, lookup.start);
    let suffix = Span::new(extent.file, lookup.end, extent.end);
    if !prefix.is_empty() {
        patch.delete(prefix);
    }
    if !is_present {
        patch.insert(lookup.start, "!");
    }
    patch.replace(module.main_span(callee.into_any())?, "has");
    if !suffix.is_empty() {
        patch.delete(suffix);
    }
    patch.sort();
    let fix = lint.fix("query map membership directly", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a positive Map.get presence test.
    #[test]
    fn test_replaces_present_lookup() {
        let session = TestSession::dir(
            &PREFER_MAP_HAS,
            r#"
function contains(values: Map<string, int32>, key: string): boolean {
    return values.get(key) !== undefined;
}
"#,
        );

        session.assert_fixes(
            r#"
function contains(values: Map<string, int32>, key: string): boolean {
    return values.has(key);
}
"#,
        );
    }

    /// Replace an absent Map.get presence test in either operand order.
    #[test]
    fn test_replaces_absent_lookup() {
        let session = TestSession::dir(
            &PREFER_MAP_HAS,
            r#"
function missing(values: Map<string, int32>, key: string): boolean {
    return undefined === values.get(key);
}
"#,
        );

        session.assert_fixes(
            r#"
function missing(values: Map<string, int32>, key: string): boolean {
    return !values.has(key);
}
"#,
        );
    }

    /// Accept a lookup whose mapped value may itself be undefined.
    #[test]
    fn test_accepts_undefined_value() {
        let session = TestSession::dir(
            &PREFER_MAP_HAS,
            r#"
function contains(values: Map<string, int32 | undefined>, key: string): boolean {
    return values.get(key) !== undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a retrieved map value.
    #[test]
    fn test_accepts_value_lookup() {
        let session = TestSession::dir(
            &PREFER_MAP_HAS,
            r#"
function lookup(values: Map<string, int32>, key: string): int32 | undefined {
    return values.get(key);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
