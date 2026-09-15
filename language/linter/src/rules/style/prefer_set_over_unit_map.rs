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
        provenance: [Clippy("zero_sized_map_values")],
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

    // collect authored Map type applications and constructions with their arguments
    let mut applications = Vec::new();
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
        let item = module.representation_item(node.into_any())?;
        applications.push((node.into_any(), item, generic_arguments.clone(), None));
    }
    for (node, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::New {
            left,
            generic_arguments,
            arguments,
        } = expression
        else {
            continue;
        };
        let item = module.language_item(*left)?;
        let construction = Some(arguments.len());
        applications.push((
            node.into_any(),
            item,
            generic_arguments.clone(),
            construction,
        ));
    }

    // report each map whose value type is unit
    for (node, item, generic_arguments, construction) in applications {
        let [key, value] = generic_arguments.as_slice() else {
            continue;
        };
        let (map_name, set_name, set_item) = match item {
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
        if module.is_within_language_item(node, set_item)? {
            continue;
        }
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
        let extent = module.source_extent(node)?;
        let span = module.source_extent(value.into_any())?;
        let message = format!("{map_name} value type carries no information");
        let mut diagnostic = lint.diagnostic(message, span);
        if let Some(suggestion) = suggestion(module, lint, extent, *key, set_name, construction)? {
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
    construction: Option<usize>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(key.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain the authored key type, constructing an empty set in place of an empty map
    let key = module.source(retained)?;
    let replacement = match construction {
        None => format!("{set}<{key}>"),
        Some(0) => format!("new {set}<{key}>()"),
        Some(_) => return Ok(None),
    };
    let patch = Patch::replace(extent, replacement);
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
    2│     return new Map<string, void>();

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
    3│ }
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

    /// Accept non-map references without asking for a map representation.
    #[test]
    fn test_accepts_generic_references() {
        let session = TestSession::dir(
            &PREFER_SET_OVER_UNIT_MAP,
            r#"
interface Iterable<T> {
    type Iterator = T;

    iterator(): T;
}

declare function identity<T>(value: T): T;
"#,
        );

        session.assert_no_diagnostics();
    }
}
