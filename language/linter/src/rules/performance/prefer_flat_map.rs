use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer flatMap over a map followed by a one-level flatten.
    pub PREFER_FLAT_MAP {
        id: "prefer-flat-map",
        summary: "Prefer flatMap over a map followed by a one-level flatten",
        explanation: r#"
Mapping an array and then flattening one level allocates an intermediate array of mapped values.
Instead, you SHOULD call `flatMap` to map and flatten in one operation.
"#,
        example: {
            reported: r#"
function pairs(values: int32[]): int32[] {
    return values.map((value) => [value, value]).flat();
}
"#,
            accepted: r#"
function pairs(values: int32[]): int32[] {
    return values.flatMap((value) => [value, value]);
}
"#,
        },
        provenance: [
            Clippy("map_flatten"),
            Unicorn("prefer-array-flat-map"),
        ],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical Array map calls immediately flattened by one level.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical zero-argument Array.flat calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(flat) = module.member_call(expression) else {
            continue;
        };
        if !flat.arguments.is_empty()
            || module.language_member(expression)? != Some(dir::LanguageItem::Array.member("flat"))
        {
            continue;
        }

        // require the flattened receiver to be one canonical Array.map call
        let Some(map) = module.member_call(flat.receiver) else {
            continue;
        };
        if map.arguments.len() != 1
            || module.language_member(flat.receiver)?
                != Some(dir::LanguageItem::Array.member("map"))
        {
            continue;
        }

        // replace both operations with the fused Array member
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("mapped array is immediately flattened", span);
        if let Some(suggestion) = suggestion(module, lint, expression, flat.receiver, map.callee)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the equivalent one-level flatMap call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    map: dir::LocalNodeId<dir::Expression>,
    map_callee: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let map_extent = module.source_extent(map.into_any())?;
    if !extent.contains_span(map_extent) {
        return Err(ProviderError::internal(
            "flattened map extent does not contain its map call",
        ));
    }
    if module.has_unretained_comment(extent, &[map_extent])? {
        return Ok(None);
    }

    // replace the mapping member and remove the trailing flatten call
    let suffix = Span::new(extent.file, map_extent.end, extent.end);
    let mut file = FilePatch::new(extent.file);
    file.replace(module.main_span(map_callee.into_any())?, "flatMap");
    file.delete(suffix);
    file.sort();
    let suggestion = lint.fix("map and flatten in one operation", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a canonical map and one-level flatten with flatMap.
    #[test]
    fn test_replaces_map_flat() {
        let session = TestSession::dir(
            &PREFER_FLAT_MAP,
            r#"
function pairs(values: int32[]): int32[] {
    return values.map((value) => [value, value]).flat();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-flat-map]: mapped array is immediately flattened
 ──▶ main.tspp:2:12
  │
1 │ function pairs(values: int32[]): int32[] {
2 │     return values.map((value) => [value, value]).flat();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: map and flatten in one operation
--- a/main.tspp
+++ b/main.tspp

    1│ function pairs(values: int32[]): int32[] {
-   2│     return values.map((value) => [value, value]).flat();
+   2│     return values.flatMap((value) => [value, value]);
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function pairs(values: int32[]): int32[] {
    return values.flatMap((value) => [value, value]);
}
"#,
        );
    }

    /// Preserve optional chaining while fusing map and flat.
    #[test]
    fn test_replaces_optional_map_flat() {
        let session = TestSession::dir(
            &PREFER_FLAT_MAP,
            r#"
function pairs(values: int32[] | undefined): int32[] | undefined {
    return values?.map((value) => [value, value]).flat();
}
"#,
        );

        session.assert_fixes(
            r#"
function pairs(values: int32[] | undefined): int32[] | undefined {
    return values?.flatMap((value) => [value, value]);
}
"#,
        );
    }

    /// Preserve comments carried by the removed flatten call.
    #[test]
    fn test_reports_commented_flat_without_fix() {
        let session = TestSession::dir(
            &PREFER_FLAT_MAP,
            r#"
function pairs(values: int32[]): int32[] {
    return values.map((value) => [value, value]).flat(/* retain */);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-flat-map]: mapped array is immediately flattened
 ──▶ main.tspp:2:12
  │
1 │ function pairs(values: int32[]): int32[] {
2 │     return values.map((value) => [value, value]).flat(/* retain */);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Accept user-defined methods with the same names.
    #[test]
    fn test_accepts_user_methods() {
        let session = TestSession::dir(
            &PREFER_FLAT_MAP,
            r#"
class Values {
    map(mapper: (value: int32) => int32): this {
        return this;
    }

    flat(): this {
        return this;
    }
}

function transform(values: Values): Values {
    return values.map((value) => value).flat();
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
