use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Map.keys or Map.values when one entry component is unused.
    pub PREFER_MAP_KEYS_VALUES {
        id: "prefer-map-keys-values",
        summary: "Prefer Map.keys or Map.values when one entry component is unused",
        explanation: r#"
Iterating `Map.entries` constructs key-value pairs even when one component is never used.
Instead, you SHOULD iterate `Map.keys` or `Map.values` directly.
"#,
        example: {
            reported: r#"
function copy(values: Map<string, int32>, output: int32[]): void {
    for (const (_, value) of values.entries()) {
        output.push(value);
    }
}
"#,
            accepted: r#"
function copy(values: Map<string, int32>, output: int32[]): void {
    for (const value of values.values()) {
        output.push(value);
    }
}
"#,
        },
        provenance: [Clippy("for_kv_map")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One map entry component retained by direct iteration.
#[derive(Debug, Clone, Copy)]
struct EntryProjection {
    /// The complete tuple pattern.
    tuple: dir::LocalNodeId<dir::Pattern>,
    /// The retained component pattern.
    retained: Option<dir::LocalNodeId<dir::Pattern>>,
    /// The entries member expression.
    callee: dir::LocalNodeId<dir::Expression>,
    /// The direct iterator method.
    method: &'static str,
}

/// Report map entry iteration that discards one component.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect synchronous for-of loops over canonical Map.entries
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(iteration) = module.for_of(expression) else {
            continue;
        };
        if iteration.asynchrony != dir::Asynchrony::Sync {
            continue;
        }
        let Some(call) = module.member_call(iteration.iterator) else {
            continue;
        };
        if call.is_optional()
            || !call.generic_arguments.is_empty()
            || !call.arguments.is_empty()
            || module.language_member(iteration.iterator)?
                != Some(dir::LanguageItem::Map.member("entries"))
        {
            continue;
        }
        let dir::ForEachBinding::Pattern { pattern, .. } = iteration.binding else {
            continue;
        };
        let dir::Pattern::Tuple { fields } = view.get(*pattern) else {
            continue;
        };
        let [key_field, value_field] = fields.as_slice() else {
            continue;
        };
        let dir::PatternField::Positional { pattern: key } = view.get(*key_field) else {
            continue;
        };
        let dir::PatternField::Positional { pattern: value } = view.get(*value_field) else {
            continue;
        };

        // choose the direct iterator when exactly one component is unused
        let is_key_unused = module
            .declared_binding_uses(key.into_any(), &occurrences)
            .is_empty();
        let is_value_unused = module
            .declared_binding_uses(value.into_any(), &occurrences)
            .is_empty();
        let projection = match (is_key_unused, is_value_unused) {
            (true, false) => EntryProjection {
                tuple: *pattern,
                retained: Some(*value),
                callee: call.callee,
                method: "values",
            },
            (false, true) => EntryProjection {
                tuple: *pattern,
                retained: Some(*key),
                callee: call.callee,
                method: "keys",
            },
            (true, true) => EntryProjection {
                tuple: *pattern,
                retained: None,
                callee: call.callee,
                method: "keys",
            },
            (false, false) => continue,
        };

        // replace pair construction with direct component iteration
        let discarded = if is_key_unused && !is_value_unused {
            *key
        } else if is_value_unused && !is_key_unused {
            *value
        } else {
            *pattern
        };
        let message = if is_key_unused && is_value_unused {
            "map entry components are unused"
        } else {
            "map entry component is unused"
        };
        let span = module.span(discarded.into_any())?;
        let mut diagnostic = lint.diagnostic(message, span);
        if let Some(fix) = fix(module, lint, projection)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one Map.entries projection with direct component iteration.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    projection: EntryProjection,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let tuple = module.source_extent(projection.tuple.into_any())?;
    let retained = projection
        .retained
        .map(|retained| module.source_extent(retained.into_any()))
        .transpose()?;
    if module.has_unretained_comment(tuple, retained.as_slice())? {
        return Ok(None);
    }

    // retain the authored component pattern and replace the iterator method
    let retained = retained
        .map(|retained| module.source(retained))
        .transpose()?
        .unwrap_or("_");
    let mut patch = FilePatch::new(tuple.file);
    patch.replace(tuple, retained);
    patch.replace(
        module.main_span(projection.callee.into_any())?,
        projection.method,
    );
    patch.sort();
    let fix = lint.fix("iterate the used map component directly", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace entry iteration whose key is discarded.
    #[test]
    fn test_replaces_discarded_key() {
        let session = TestSession::dir(
            &PREFER_MAP_KEYS_VALUES,
            r#"
function copy(values: Map<string, int32>, output: int32[]): void {
    for (const (_, value) of values.entries()) {
        output.push(value);
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function copy(values: Map<string, int32>, output: int32[]): void {
    for (const value of values.values()) {
        output.push(value);
    }
}
"#,
        );
    }

    /// Replace entry iteration whose value binding is unused.
    #[test]
    fn test_replaces_unused_value() {
        let session = TestSession::dir(
            &PREFER_MAP_KEYS_VALUES,
            r#"
function copy(values: Map<string, int32>, output: string[]): void {
    for (const (key, value) of values.entries()) {
        output.push(key);
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function copy(values: Map<string, int32>, output: string[]): void {
    for (const key of values.keys()) {
        output.push(key);
    }
}
"#,
        );
    }

    /// Replace entry iteration that uses neither component.
    #[test]
    fn test_replaces_unused_entry() {
        let session = TestSession::dir(
            &PREFER_MAP_KEYS_VALUES,
            r#"
function repeat(values: Map<string, int32>, output: int32[]): void {
    for (const (key, value) of values.entries()) {
        output.push(0);
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function repeat(values: Map<string, int32>, output: int32[]): void {
    for (const _ of values.keys()) {
        output.push(0);
    }
}
"#,
        );
    }

    /// Accept entry iteration that uses both components.
    #[test]
    fn test_accepts_complete_entry() {
        let session = TestSession::dir(
            &PREFER_MAP_KEYS_VALUES,
            r#"
function copy(values: Map<string, int32>, keys: string[], output: int32[]): void {
    for (const (key, value) of values.entries()) {
        keys.push(key);
        output.push(value);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
