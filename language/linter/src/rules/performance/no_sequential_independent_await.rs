use std::collections::HashSet;

use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_source::LabeledSpan;
use destack_workspace::LintSeverity;

use crate::rules::common::{assign_pattern_target_symbol, collect_pattern_value_binding_symbols};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow sequential independent awaits in the same block.
    ///
    /// Independent async operations should run concurrently where possible.
    /// Sequential awaits can often be rewritten with `Promise.all`.
    #[lint(
        id = "no-sequential-independent-await",
        code = "LP020",
        category = Performance,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoSequentialIndependentAwait,
    "Disallow sequential independent awaits in the same block"
}

impl LintRule for NoSequentialIndependentAwait {
    fn meta(&self) -> &'static LintMeta {
        NoSequentialIndependentAwait::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // check sequential awaits in each block
        for block_id in ctx.dir.iter_node_ids_of_type::<dir::Block>() {
            let block = ctx.dir.get(block_id);
            let expression_ids = block.iter_expressions().collect::<Vec<_>>();
            report_sequential_independent_awaits(ctx, meta, expression_ids.as_slice());
        }
    }
}

/// One await statement shape extracted from a block statement.
#[derive(Debug, Clone)]
struct AwaitStatement {
    /// The containing statement expression id.
    statement_expression_id: dir::LocalNodeId<dir::Expression>,
    /// The await operand used for dependency checks.
    await_operand_expression_id: dir::LocalNodeId<dir::Expression>,
    /// Symbols bound by this await statement.
    bound_symbols: HashSet<dir::LocalSymbolId>,
}

/// Report sequential awaits in one block when the second does not depend on the first.
fn report_sequential_independent_awaits(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    expression_ids: &[dir::LocalNodeId<dir::Expression>],
) {
    let mut previous_await: Option<AwaitStatement> = None;

    for expression_id in expression_ids {
        let current_await = await_statement_from_expression(ctx, *expression_id);
        let Some(current_await) = current_await else {
            previous_await = None;
            continue;
        };

        if let Some(previous_await_statement) = previous_await.as_ref()
            && awaits_are_independent(
                ctx.dir.tree(),
                ctx.module_id(),
                ctx.resolutions,
                previous_await_statement,
                &current_await,
            )
        {
            let severity = ctx.get_effective_severity(meta, current_await.statement_expression_id);
            if severity.is_enabled() {
                let current_span = ctx.get_span(current_await.statement_expression_id);
                let previous_span = ctx.get_span(previous_await_statement.statement_expression_id);
                let diagnostic = LintReport::new(
                    NO_SEQUENTIAL_INDEPENDENT_AWAIT.id,
                    NO_SEQUENTIAL_INDEPENDENT_AWAIT.code,
                    NO_SEQUENTIAL_INDEPENDENT_AWAIT.category,
                    severity,
                    "sequential await expressions appear independent",
                    current_span,
                )
                .label("this await can likely run in parallel with the previous await")
                .secondary(LabeledSpan::new(
                    previous_span,
                    "previous await does not feed this await",
                ));
                ctx.report(diagnostic);
            }
        }

        previous_await = Some(current_await);
    }
}

/// Extract one await statement shape from one expression statement when possible.
fn await_statement_from_expression(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<AwaitStatement> {
    let statement_id = expression_id;
    let statement = ctx.dir.get(statement_id);

    // keep direct await statements
    if let Some(await_operand_expression_id) = await_operand_expression_id(statement) {
        return Some(AwaitStatement {
            statement_expression_id: expression_id,
            await_operand_expression_id,
            bound_symbols: HashSet::new(),
        });
    }

    // keep assignment await forms like: value = await fetch()
    if let dir::Expression::Assign { left, right, .. } = statement
        && let Some(await_operand_expression_id) = await_operand_expression_id(ctx.dir.get(*right))
    {
        let mut bound_symbols = HashSet::new();

        if let Some(target_symbol) = assign_pattern_target_symbol(ctx, *left)
            && target_symbol.module_id == ctx.module_id()
        {
            bound_symbols.insert(target_symbol.local_id);
        }

        return Some(AwaitStatement {
            statement_expression_id: expression_id,
            await_operand_expression_id,
            bound_symbols,
        });
    }

    // keep return await forms like: return await fetch()
    if let dir::Expression::Return { value } = statement
        && let Some(value_expression_id) = value
        && let Some(await_operand_expression_id) =
            await_operand_expression_id(ctx.dir.get(*value_expression_id))
    {
        return Some(AwaitStatement {
            statement_expression_id: expression_id,
            await_operand_expression_id,
            bound_symbols: HashSet::new(),
        });
    }

    // keep single declarator await bindings like: const x = await foo()
    let declarators = match statement {
        dir::Expression::Let { declarators, .. } | dir::Expression::Using { declarators, .. } => {
            declarators
        }
        _ => return None,
    };
    if declarators.len() != 1 {
        return None;
    }

    let declarator = ctx.dir.get(declarators[0]);
    let value_expression_id = declarator.value?;
    let await_operand_expression_id =
        await_operand_expression_id(ctx.dir.get(value_expression_id))?;

    // collect all local bindings introduced by the declaration pattern
    let mut bound_symbols = HashSet::new();
    collect_pattern_value_binding_symbols(
        ctx.dir.tree(),
        &ctx.symbols,
        declarator.pattern,
        &mut bound_symbols,
    );

    Some(AwaitStatement {
        statement_expression_id: expression_id,
        await_operand_expression_id,
        bound_symbols,
    })
}

/// Return the await operand expression id for one await expression form.
fn await_operand_expression_id(
    expression: &dir::Expression,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match expression {
        dir::Expression::Await { expression }
        | dir::Expression::AwaitMaybe { expression }
        | dir::Expression::AwaitMust { expression } => Some(*expression),
        _ => None,
    }
}

/// Return true when two await statements are independent.
fn awaits_are_independent(
    tree: &dir::Tree,
    module_id: destack_source::ModuleId,
    resolutions: &dir::ResolutionTable<'_>,
    previous: &AwaitStatement,
    current: &AwaitStatement,
) -> bool {
    // a previous await with no bound symbols cannot be referenced by the next await
    if previous.bound_symbols.is_empty() {
        return true;
    }

    // the current await depends on previous when it references any prior bound symbol
    !expression_references_any_symbol(
        tree,
        module_id,
        resolutions,
        current.await_operand_expression_id,
        &previous.bound_symbols,
    )
}

/// Return true when an expression references any symbol from one set.
fn expression_references_any_symbol(
    tree: &dir::Tree,
    module_id: destack_source::ModuleId,
    resolutions: &dir::ResolutionTable<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    target_symbols: &HashSet<dir::LocalSymbolId>,
) -> bool {
    let mut visitor = SymbolReferenceVisitor::new(module_id, resolutions, target_symbols);
    let expression = tree.get(expression_id);
    visitor.visit_expression(tree, expression_id, expression);
    visitor.is_referenced
}

/// Visitor that finds one reference to any target symbol.
struct SymbolReferenceVisitor<'a> {
    /// The current module id.
    module_id: destack_source::ModuleId,
    /// The resolution table carrying semantic targets.
    resolutions: &'a dir::ResolutionTable<'a>,
    /// Target local symbols to detect.
    target_symbols: &'a HashSet<dir::LocalSymbolId>,
    /// Whether any target symbol was referenced.
    is_referenced: bool,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl<'a> SymbolReferenceVisitor<'a> {
    /// Build a visitor for one symbol set.
    fn new(
        module_id: destack_source::ModuleId,
        resolutions: &'a dir::ResolutionTable<'a>,
        target_symbols: &'a HashSet<dir::LocalSymbolId>,
    ) -> Self {
        Self {
            module_id,
            resolutions,
            target_symbols,
            is_referenced: false,
            options: NodeVisitorOptions::default(),
        }
    }
}

impl NodeVisitor for SymbolReferenceVisitor<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // stop traversal once one reference is found
        if self.is_referenced {
            return;
        }

        // keep nested function references out of this dependency check
        if let dir::Expression::Declaration(declaration) = expression {
            let declaration = tree.get(*declaration);
            if matches!(declaration, dir::Declaration::Function(_)) {
                return;
            }
        }

        // detect symbol-backed references
        if let Some(target_symbol) = self
            .resolutions
            .symbol_resolution(id.into_global_any(self.module_id))
            && target_symbol.module_id == self.module_id
            && self.target_symbols.contains(&target_symbol.local_id)
        {
            self.is_referenced = true;
            return;
        }

        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag two independent awaited expression statements.
    #[test]
    fn test_flags_independent_sequential_await_statements() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_flags_independent_sequential_await_statements.ds",
            r#"
async function run() {
    await first();
    await second();
}
"#,
        );
        test.result(result)
            .assert_lint("no-sequential-independent-await");
    }

    /// Flag two independent awaited declarations.
    #[test]
    fn test_flags_independent_sequential_await_declarations() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_flags_independent_sequential_await_declarations.ds",
            r#"
async function run() {
    const left = await first();
    const right = await second();
    return left + right;
}
"#,
        );
        test.result(result)
            .assert_lint("no-sequential-independent-await");
    }

    /// Allow dependent sequential awaits.
    #[test]
    fn test_allows_dependent_sequential_awaits() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_allows_dependent_sequential_awaits.ds",
            r#"
async function run() {
    const token = await first();
    const value = await second(token);
    return value;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-sequential-independent-await");
    }

    /// Allow single awaits.
    #[test]
    fn test_allows_single_await() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_allows_single_await.ds",
            r#"
async function run() {
    await first();
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-sequential-independent-await");
    }

    /// Reset sequence detection across non-await statements.
    #[test]
    fn test_resets_sequence_across_non_await_statement() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_resets_sequence_across_non_await_statement.ds",
            r#"
async function run() {
    await first();
    let marker = 1;
    await second();
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-sequential-independent-await");
    }

    /// Keep nested function references out of dependency checks.
    #[test]
    fn test_ignores_nested_function_reference_when_checking_dependency() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_ignores_nested_function_reference_when_checking_dependency.ds",
            r#"
async function run() {
    const token = await first();
    await second(() => token);
}
"#,
        );
        test.result(result)
            .assert_lint("no-sequential-independent-await");
    }

    /// Allow dependent sequential awaits through destructured await bindings.
    #[test]
    fn test_allows_dependent_sequential_await_with_destructured_binding() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_allows_dependent_sequential_await_with_destructured_binding.ds",
            r#"
async function run() {
    const [token] = await first();
    const value = await second(token);
    return value;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-sequential-independent-await");
    }

    /// Allow dependent sequential awaits through using bindings.
    #[test]
    fn test_allows_dependent_sequential_await_with_using_binding() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_allows_dependent_sequential_await_with_using_binding.ds",
            r#"
async function run() {
    using token = await first();
    await second(token);
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-sequential-independent-await");
    }

    /// Flag independent sequential await assignments.
    #[test]
    fn test_flags_independent_sequential_await_assignments() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_flags_independent_sequential_await_assignments.ds",
            r#"
async function run() {
    let left;
    let right;
    left = await first();
    right = await second();
    return left + right;
}
"#,
        );
        test.result(result)
            .assert_lint("no-sequential-independent-await");
    }

    /// Allow dependent sequential await assignments.
    #[test]
    fn test_allows_dependent_sequential_await_assignments() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_allows_dependent_sequential_await_assignments.ds",
            r#"
async function run() {
    let token;
    let value;
    token = await first();
    value = await second(token);
    return value;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-sequential-independent-await");
    }

    /// Flag independent return-await after a prior await.
    #[test]
    fn test_flags_independent_return_await_after_prior_await() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_flags_independent_return_await_after_prior_await.ds",
            r#"
async function run() {
    await first();
    return await second();
}
"#,
        );
        test.result(result)
            .assert_lint("no-sequential-independent-await");
    }

    /// Allow dependent return-await after a prior bound await.
    #[test]
    fn test_allows_dependent_return_await_after_prior_await() {
        let test = TestProgram::for_rule_without_prelude(NoSequentialIndependentAwait);
        let result = test.lint_dir(
            "no_sequential_independent_await/test_allows_dependent_return_await_after_prior_await.ds",
            r#"
async function run() {
    const token = await first();
    return await second(token);
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-sequential-independent-await");
    }
}
