use destack_ast as ast;
use std::collections::{HashMap, HashSet};

use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_declared_or_inferred_type_id, expression_target_symbol,
    expression_unwrap_parenthesized, tuple_type_arity,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer tuple destructuring over indexed access.
    ///
    /// When accessing multiple elements of one tuple by index in consecutive
    /// declarations, prefer using tuple destructuring instead.
    #[lint(
        id = "prefer-tuple-destructure",
        code = "LY059",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferTupleDestructure,
    "Prefer tuple destructuring over indexed access"
}

impl LintRule for PreferTupleDestructure {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferTupleDestructure::meta()
    }

    /// Check module DIR declarators for repeated tuple index reads.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut access_groups = HashMap::new();

        // collect direct tuple index reads grouped by resolved tuple symbol
        for declarator_id in ctx.tree.iter_node_ids_of_type::<dir::Declarator>() {
            let Some(tuple_access) = tuple_indexed_access(ctx, declarator_id) else {
                continue;
            };
            let source_symbol = tuple_access.source_symbol;
            let source_text = tuple_access.source_text.clone();
            let tuple_arity = tuple_access.tuple_arity;

            access_groups
                .entry(source_symbol)
                .or_insert_with(|| TupleAccessGroup::new(source_text, tuple_arity))
                .accesses
                .push(tuple_access);
        }

        // report groups that read at least two distinct tuple slots
        for access_group in access_groups.into_values() {
            if !tuple_access_group_has_multiple_indices(&access_group) {
                continue;
            }

            let Some(first_access) = access_group.accesses.first() else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, first_access.declarator_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                PREFER_TUPLE_DESTRUCTURE.id,
                PREFER_TUPLE_DESTRUCTURE.code,
                PREFER_TUPLE_DESTRUCTURE.category,
                severity,
                format!(
                    "multiple indexed accesses to `{}` could use tuple destructuring",
                    access_group.source_text
                ),
                ctx.module.file_id,
                ctx.get_span(first_access.declarator_id),
            )
            .with_label("use tuple destructuring instead");

            // attach the multi declarator rewrite only when one exact source rewrite is safe
            if ctx.include_fixes
                && let Some(fix) = prefer_tuple_destructure_fix(ctx, &access_group)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// One resolved tuple index access.
#[derive(Debug, Clone)]
struct TupleAccess {
    /// The tuple symbol read by the index expression.
    source_symbol: dir::GlobalSymbolId,
    /// The exact tuple source text used in the index expression.
    source_text: String,
    /// The fixed tuple arity resolved from types.
    tuple_arity: usize,
    /// The indexed tuple slot.
    index: usize,
    /// The declarator that performs the read.
    declarator_id: dir::LocalNodeId<dir::Declarator>,
}

/// One group of tuple accesses that share the same tuple symbol.
#[derive(Debug, Clone)]
struct TupleAccessGroup {
    /// The tuple source text used in diagnostics and fixes.
    source_text: String,
    /// The fixed tuple arity.
    tuple_arity: usize,
    /// All declarator reads for this tuple symbol.
    accesses: Vec<TupleAccess>,
}

impl TupleAccessGroup {
    /// Build one empty tuple access group.
    fn new(source_text: String, tuple_arity: usize) -> Self {
        Self {
            source_text,
            tuple_arity,
            accesses: Vec::new(),
        }
    }
}

/// Return true when one tuple access group reads at least two distinct indices.
fn tuple_access_group_has_multiple_indices(access_group: &TupleAccessGroup) -> bool {
    let mut indices = HashSet::new();

    // keep only groups that read at least two distinct slots
    for access in &access_group.accesses {
        indices.insert(access.index);
        if indices.len() >= 2 {
            return true;
        }
    }

    false
}

/// Resolve one declarator as a direct tuple index read when possible.
fn tuple_indexed_access(
    ctx: &LintModuleDirContext<'_>,
    declarator_id: dir::LocalNodeId<dir::Declarator>,
) -> Option<TupleAccess> {
    let declarator = ctx.tree.get(declarator_id);
    let value_id = declarator.value?;

    // keep simple binding declarators only
    let pattern = ctx.tree.get(declarator.pattern);
    if !matches!(
        pattern,
        dir::Pattern::Binding {
            mutability: _,
            name: _,
            pattern: None,
            symbol: _,
        }
    ) {
        return None;
    }

    // keep direct tuple index expressions only
    let value_id = expression_unwrap_parenthesized(ctx.tree, value_id);
    let dir::Expression::Index { left, right } = ctx.tree.get(value_id) else {
        return None;
    };
    let index_expression_id = (*right)?;
    let left_id = expression_unwrap_parenthesized(ctx.tree, *left);

    // resolve the direct tuple reference symbol and source text
    let source_symbol = expression_target_symbol(ctx.tree, left_id)?;
    let source_text = direct_reference_text(ctx, left_id)?;

    // resolve one fixed tuple type for the indexed source
    let source_type_id =
        expression_declared_or_inferred_type_id(ctx.module_id(), ctx.tree, ctx.types, left_id)?;
    let tuple_arity = tuple_type_arity(ctx.types, source_type_id)?;
    let index = integer_literal_index(ctx.tree, index_expression_id)?;
    if index >= tuple_arity {
        return None;
    }

    Some(TupleAccess {
        source_symbol,
        source_text,
        tuple_arity,
        index,
        declarator_id,
    })
}

/// Return the exact source text for one direct reference expression.
fn direct_reference_text(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<String> {
    let expression = ctx.tree.get(expression_id);

    // keep plain symbol references only
    match expression {
        dir::Expression::LocalReference {
            path: _,
            target_symbol: _,
            generic_arguments,
        }
        | dir::Expression::ModuleReference {
            path: _,
            target_symbol: _,
            generic_arguments,
        }
        | dir::Expression::GlobalReference {
            path: _,
            target_symbol: _,
            generic_arguments,
        } if generic_arguments.is_empty() => {
            Some(ctx.get_span_text(ctx.get_span(expression_id)).to_string())
        }
        _ => None,
    }
}

/// Resolve one integer literal index as a zero based tuple slot.
fn integer_literal_index(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<usize> {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);
    let dir::Expression::ScalarLiteral {
        value: dir::ScalarLiteral::Integer(index),
    } = expression
    else {
        return None;
    };
    if *index < 0 {
        return None;
    }

    Some(*index as usize)
}

/// Build a safe tuple destructure rewrite for one multi declarator let expression.
fn prefer_tuple_destructure_fix(
    ctx: &LintModuleDirContext<'_>,
    access_group: &TupleAccessGroup,
) -> Option<LintFix> {
    // keep at least two exact accesses for a rewrite
    if access_group.accesses.len() < 2 {
        return None;
    }

    // keep one parent let expression for all declarators
    let first_declarator_id = access_group.accesses.first()?.declarator_id;
    let parent_node_id = ctx.tree.get_parent(first_declarator_id.id)?;
    if parent_node_id.ty != dir::NodeType::Expression {
        return None;
    }
    let parent_expression_id = parent_node_id.into_typed::<dir::Expression>();
    let parent_expression = ctx.tree.get(parent_expression_id);
    let dir::Expression::Let {
        export: _,
        ambient: _,
        mutability: _,
        declarators,
    } = parent_expression
    else {
        return None;
    };

    // require exact declarator coverage inside one let expression
    if declarators.len() != access_group.accesses.len() {
        return None;
    }
    for access in &access_group.accesses {
        let access_parent_node_id = ctx.tree.get_parent(access.declarator_id.id)?;
        if access_parent_node_id != parent_node_id {
            return None;
        }
    }

    // require a contiguous tuple prefix with one unique binding per index
    let mut names_by_index = vec![None; access_group.accesses.len()];
    for access in &access_group.accesses {
        if access.index >= names_by_index.len() || access.index >= access_group.tuple_arity {
            return None;
        }

        let declarator = ctx.tree.get(access.declarator_id);
        let pattern = ctx.tree.get(declarator.pattern);
        let dir::Pattern::Binding {
            name,
            pattern: None,
            mutability: _,
            symbol: _,
        } = pattern
        else {
            return None;
        };

        let slot = &mut names_by_index[access.index];
        if slot.is_some() {
            return None;
        }
        *slot = Some(ctx.repository.strings.get(*name).to_string());
    }
    if names_by_index.iter().any(Option::is_none) {
        return None;
    }

    // build one exact tuple destructuring replacement
    let names = names_by_index.into_iter().flatten().collect::<Vec<_>>();
    let source_expression_id =
        ctx.source_node_id::<ast::Expression>(parent_expression_id.into_any())?;
    let source_expression = ctx.ast.get(source_expression_id);
    let ast::Expression::Let {
        kind,
        export: _,
        ambient: _,
        mutability: _,
        declarators: _,
    } = source_expression
    else {
        return None;
    };
    let let_keyword = match kind {
        ast::LetKind::Const => "const",
        ast::LetKind::Let => "let",
        ast::LetKind::Var => "var",
    };
    let replacement = format!(
        "{let_keyword} ({}) = {}",
        names.join(", "),
        access_group.source_text
    );
    let edits = ctx
        .edit_builder()
        .replace(ctx.get_span(parent_expression_id), replacement)
        .into_edits();

    Some(LintFix::safe("Rewrite indexed tuple reads to destructuring").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_multiple_tuple_indexed_accesses() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleDestructure);
        let result = test.lint_dir(
            "prefer_tuple_destructure/test_detects_multiple_tuple_indexed_accesses.ds",
            r#"
function foo(tuple: (int32, int32)) {
    const first = tuple[0]
    const second = tuple[1]
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("prefer-tuple-destructure")
            .assert_has_no_fix("prefer-tuple-destructure");
    }

    #[test]
    fn test_allows_existing_tuple_destructure() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleDestructure);
        let result = test.lint_dir(
            "prefer_tuple_destructure/test_allows_existing_tuple_destructure.ds",
            r#"
function foo() {
    const tuple = (1, 2)
    const (first, second) = tuple
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-tuple-destructure");
    }

    #[test]
    fn test_allows_single_tuple_access() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleDestructure);
        let result = test.lint_dir(
            "prefer_tuple_destructure/test_allows_single_tuple_access.ds",
            r#"
function foo(tuple: (int32, int32)) {
    const first = tuple[0]
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-tuple-destructure");
    }

    #[test]
    fn test_allows_different_tuple_sources() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleDestructure);
        let result = test.lint_dir(
            "prefer_tuple_destructure/test_allows_different_tuple_sources.ds",
            r#"
function foo(a: (int32, int32), b: (int32, int32)) {
    const x = a[0]
    const y = b[0]
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-tuple-destructure");
    }

    #[test]
    fn test_detects_three_tuple_accesses() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleDestructure);
        let result = test.lint_dir(
            "prefer_tuple_destructure/test_detects_three_tuple_accesses.ds",
            r#"
function foo(tuple: (int32, int32, int32)) {
    const a = tuple[0]
    const b = tuple[1]
    const c = tuple[2]
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("prefer-tuple-destructure")
            .assert_has_no_fix("prefer-tuple-destructure");
    }

    #[test]
    fn test_allows_array_accesses() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleDestructure);
        let result = test.lint_dir(
            "prefer_tuple_destructure/test_allows_array_accesses.ds",
            r#"
function foo(arr: int32[]) {
    const first = arr[0]
    const second = arr[1]
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-tuple-destructure");
    }

    #[test]
    fn test_fixes_multi_declarator_tuple_reads() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleDestructure);
        let result = test.lint_dir(
            "prefer_tuple_destructure/test_fixes_multi_declarator_tuple_reads.ds",
            r#"
function foo(tuple: (int32, int32)) {
    const first = tuple[0], second = tuple[1]
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("prefer-tuple-destructure")
            .assert_safe_fixed(
                r#"
function foo(tuple: (int32, int32,)) {
    const (first, second) = tuple;
}
"#,
            );
    }

    #[test]
    fn test_allows_shadowed_same_name_sources() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleDestructure);
        let result = test.lint_dir(
            "prefer_tuple_destructure/test_allows_shadowed_same_name_sources.ds",
            r#"
function foo() {
    {
        const tuple = (1, 2)
        const first = tuple[0]
    }

    {
        const tuple = ("x", "y")
        const second = tuple[1]
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-tuple-destructure");
    }
}
