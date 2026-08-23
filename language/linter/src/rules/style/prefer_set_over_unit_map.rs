use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer sets over maps whose values are unit.
    pub PREFER_SET_OVER_UNIT_MAP {
        id: "prefer-set-over-unit-map",
        summary: "Prefer sets over maps whose values are unit",
        explanation: r#"
A map with unit values expresses only whether each key is present.
Instead, you SHOULD use `Set<K>` to represent membership directly.
"#,
        example: {
            reported: r#"
const values = new Map<string, void>();
"#,
            accepted: r#"
const values = new Set<string>();
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report authored Map applications whose value type is unit.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored Map type applications
    for (node, ty) in view.iter_nodes::<dir::TypeExpression>() {
        let (dir::TypeExpression::Reference {
            generic_arguments, ..
        }
        | dir::TypeExpression::Member {
            generic_arguments, ..
        }) = ty
        else {
            continue;
        };
        let (map_name, set_name, set_item) = match module.representation_item(node.into_any())? {
            Some(dir::LanguageItem::Map) => ("Map", "Set", dir::LanguageItem::Set),
            Some(dir::LanguageItem::SortedMap) => {
                ("SortedMap", "SortedSet", dir::LanguageItem::SortedSet)
            }
            Some(dir::LanguageItem::ConcurrentMap) => (
                "ConcurrentMap",
                "ConcurrentSet",
                dir::LanguageItem::ConcurrentSet,
            ),
            _ => continue,
        };
        if module.is_within_language_item(node.into_any(), set_item)? {
            continue;
        }
        let [key, value] = generic_arguments.as_slice() else {
            continue;
        };
        let dir::GenericArgument::Type { value: key } = view.get(*key) else {
            continue;
        };
        let dir::GenericArgument::Type { value } = view.get(*value) else {
            continue;
        };
        if !module.node_type(value.into_any())?.is_unit() {
            continue;
        }

        // replace the redundant value mapping with direct membership
        let extent = module.source_extent(node.into_any())?;
        let span = module.source_extent(value.into_any())?;
        let message = format!("{map_name} value type carries no information");
        let mut diagnostic = lint.diagnostic(message, span);
        if let Some(suggestion) = suggestion(module, lint, extent, *key, set_name)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one Set type replacement from the retained key type.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: destack_source::Span,
    key: dir::LocalNodeId<dir::TypeExpression>,
    set: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(key.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain the authored key type
    let key = module.source(retained)?;
    let patch = Patch::replace(extent, format!("{set}<{key}>"));
    let suggestion = lint.suggestion(format!("use a {set}"), patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a unit-valued Map annotation and construction.
    #[test]
    fn test_replaces_unit_map() {
        let session = TestSession::dir(
            &PREFER_SET_OVER_UNIT_MAP,
            r#"
function collect(): Map<string, void> {
    return new Map<string, void>();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-set-over-unit-map]: Map value type carries no information
 ──▶ main.ds:1:33
  │
1 │ function collect(): Map<string, void> {
  │                                 ^^^^
2 │     return new Map<string, void>();
3 │ }
  │

 = suggestion: use a Set (requires review)
--- a/main.ds
+++ b/main.ds

-   1│ function collect(): Map<string, void> {
+   1│ function collect(): Set<string> {

warning[prefer-set-over-unit-map]: Map value type carries no information
 ──▶ main.ds:2:28
  │
1 │ function collect(): Map<string, void> {
2 │     return new Map<string, void>();
  │                            ^^^^
3 │ }
  │

 = suggestion: use a Set (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function collect(): Map<string, void> {
-   2│     return new Map<string, void>();
+   2│     return new Set<string>();
"#,
        );
        session.assert_suggestions(
            r#"
function collect(): Set<string> {
    return new Set<string>();
}
"#,
        );
    }

    /// Replace an empty-tuple Map value with direct membership.
    #[test]
    fn test_replaces_empty_tuple_map() {
        let session = TestSession::dir(
            &PREFER_SET_OVER_UNIT_MAP,
            r#"
declare function collect(): Map<string, ()>;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-set-over-unit-map]: Map value type carries no information
 ──▶ main.ds:1:41
  │
1 │ declare function collect(): Map<string, ()>;
  │                                         ^^
  │

 = suggestion: use a Set (requires review)
--- a/main.ds
+++ b/main.ds

-   1│ declare function collect(): Map<string, ()>;
+   1│ declare function collect(): Set<string>;
"#,
        );
        session.assert_suggestions(
            r#"
declare function collect(): Set<string>;
"#,
        );
    }

    /// Preserve ordered and concurrent collection behavior.
    #[test]
    fn test_replaces_other_unit_maps() {
        let session = TestSession::dir(
            &PREFER_SET_OVER_UNIT_MAP,
            r#"
declare function sorted(): SortedMap<string, void>;
declare function concurrent(): ConcurrentMap<string, ()>;
"#,
        );

        session.assert_suggestions(
            r#"
declare function sorted(): SortedSet<string>;
declare function concurrent(): ConcurrentSet<string>;
"#,
        );
    }

    /// Accept maps that retain a meaningful value.
    #[test]
    fn test_accepts_map_value() {
        let session = TestSession::dir(
            &PREFER_SET_OVER_UNIT_MAP,
            r#"
function collect(): Map<string, boolean> {
    return new Map<string, boolean>();
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
