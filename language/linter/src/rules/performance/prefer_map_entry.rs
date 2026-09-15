use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer one map entry operation over a guarded insertion.
    pub PREFER_MAP_ENTRY {
        id: "prefer-map-entry",
        summary: "Prefer one map entry operation over a guarded insertion",
        explanation: r#"
Checking a map before inserting a missing value performs the same key lookup twice.
Instead, you SHOULD use `getOrInsertWith` to combine the lookup and conditional insertion.
"#,
        example: {
            reported: r#"
import { Map } from "destack:collections";

function ensureValue(values: Map<string, int32>, key: string): void {
    if (!values.has(key)) {
        values.set(key, 1);
    }
}
"#,
            accepted: r#"
import { Map } from "destack:collections";

function ensureValue(values: Map<string, int32>, key: string): void {
    values.getOrInsertWith(key, () => 1);
}
"#,
        },
        provenance: [Clippy("map_entry")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One absent-key check followed by insertion into the same map.
#[derive(Debug, Clone, Copy)]
struct GuardedInsertion {
    /// The map expression.
    map: dir::LocalNodeId<dir::Expression>,
    /// The tested and inserted key.
    key: dir::LocalNodeId<dir::Expression>,
    /// The conditionally inserted value.
    value: dir::LocalNodeId<dir::Expression>,
}

/// Report guarded map insertions that repeat one key lookup.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect regular if statements without alternate branches
    for (expression, node) in module.view().iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form: dir::IfForm::If,
            condition,
            then_expression,
            else_expression: None,
        } = node
        else {
            continue;
        };
        let Some(condition) = condition.as_expression() else {
            continue;
        };
        let Some(insertion) = select_guarded_insertion(module, condition, *then_expression)? else {
            continue;
        };

        // replace the repeated lookup when moving the key remains valid
        let key_type = module.adjusted_type_id(insertion.key.into_any())?;
        if !module.satisfies_copy(key_type)? {
            continue;
        }
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("map insertion repeats the key lookup", span);
        if let Some(suggestion) = suggestion(module, lint, span, insertion)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one negated Map.has check whose branch only calls Map.set.
fn select_guarded_insertion(
    module: &DirModule<'_>,
    condition: dir::LocalNodeId<dir::Expression>,
    branch: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<GuardedInsertion>, ProviderError> {
    let Some((dir::UnaryOperator::Not, condition)) = module.builtin_unary(condition)? else {
        return Ok(None);
    };
    let condition = condition.source.local_id;
    let Some(check) = module.member_call(condition) else {
        return Ok(None);
    };
    let [key] = check.arguments else {
        return Ok(None);
    };
    let dir::Argument::Positional { value: key } = module.view().get(*key) else {
        return Ok(None);
    };
    if check.is_optional()
        || !check.generic_arguments.is_empty()
        || module.language_member(condition)? != Some(dir::LanguageItem::Map.member("has"))
    {
        return Ok(None);
    }

    // require one canonical insertion with the same stable receiver and key
    let Some(insertion) = module.sole_expression(branch) else {
        return Ok(None);
    };
    let Some(set) = module.member_call(insertion) else {
        return Ok(None);
    };
    let [set_key, value] = set.arguments else {
        return Ok(None);
    };
    let dir::Argument::Positional { value: set_key } = module.view().get(*set_key) else {
        return Ok(None);
    };
    let dir::Argument::Positional { value } = module.view().get(*value) else {
        return Ok(None);
    };
    if set.is_optional()
        || !set.generic_arguments.is_empty()
        || module.language_member(insertion)? != Some(dir::LanguageItem::Map.member("set"))
        || !module.is_discarded_expression(insertion)
        || !module.is_duplicable_expression(check.receiver)?
        || !module.is_duplicable_expression(*key)?
        || !module.is_same_computation(check.receiver, set.receiver)?
        || !module.is_same_computation(*key, *set_key)?
    {
        return Ok(None);
    }

    Ok(Some(GuardedInsertion {
        map: check.receiver,
        key: *key,
        value: *value,
    }))
}

/// Replace one guarded insertion with Map.getOrInsertWith.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: destack_source::Span,
    insertion: GuardedInsertion,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let map_extent = module.source_extent(insertion.map.into_any())?;
    let key_extent = module.source_extent(insertion.key.into_any())?;
    let value_extent = module.source_extent(insertion.value.into_any())?;
    if module.has_unretained_comment(extent, &[map_extent, key_extent, value_extent])? {
        return Ok(None);
    }

    // retain the authored map, key, and conditionally evaluated value
    let map = module.expression_source(insertion.map, dir::OperatorPrecedence::Postfix)?;
    let key = module.source(key_extent)?;
    let value = module.source(value_extent)?;
    let value = match module.view().get(insertion.value) {
        dir::Expression::ObjectExpression { .. } => format!("({value})"),
        _ => value.to_owned(),
    };
    let replacement = format!("{map}.getOrInsertWith({key}, () => {value});");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("use one map entry operation", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a guarded insertion into one map.
    #[test]
    fn test_replaces_guarded_insertion() {
        let session = TestSession::dir(
            &PREFER_MAP_ENTRY,
            r#"
import { Map } from "destack:collections";

function ensureValue(values: Map<string, int32>, key: string): void {
    if (!values.has(key)) {
        values.set(key, 1);
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Map } from "destack:collections";

function ensureValue(values: Map<string, int32>, key: string): void {
    values.getOrInsertWith(key, () => 1);
}
"#,
        );
    }

    /// Accept a branch that performs another operation.
    #[test]
    fn test_accepts_additional_branch_work() {
        let session = TestSession::dir(
            &PREFER_MAP_ENTRY,
            r#"
import { Map } from "destack:collections";

declare function inserted(): void;

function ensureValue(values: Map<string, int32>, key: string): void {
    if (!values.has(key)) {
        values.set(key, 1);
        inserted();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a guarded insertion under another key.
    #[test]
    fn test_accepts_other_key() {
        let session = TestSession::dir(
            &PREFER_MAP_ENTRY,
            r#"
import { Map } from "destack:collections";

function ensureValue(values: Map<string, int32>, checked: string, inserted: string): void {
    if (!values.has(checked)) {
        values.set(inserted, 1);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
