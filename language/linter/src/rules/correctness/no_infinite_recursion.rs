use std::collections::{HashMap, HashSet};

use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{find_cycle_path, strongly_connected_components};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow functions that recurse without conditional guards.
    ///
    /// Unconditional self recursion or unconditional mutual recursion cycles
    /// lead to non-terminating call paths and eventual stack overflows.
    #[lint(
        id = "no-infinite-recursion",
        code = "LC019",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoInfiniteRecursion,
    "Disallow infinite recursion"
}

impl LintRule for NoInfiniteRecursion {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoInfiniteRecursion::meta()
    }

    /// Check module DIR nodes for unconditional recursion cycles.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // collect all function declarations with bodies in this module
        let function_infos = collect_function_infos(ctx);
        if function_infos.is_empty() {
            return;
        }

        // index functions by symbol for graph construction and reporting
        let function_symbols = function_infos
            .iter()
            .map(|function| function.symbol)
            .collect::<HashSet<_>>();
        let function_by_symbol = function_infos
            .iter()
            .map(|function| (function.symbol, function))
            .collect::<HashMap<_, _>>();

        // build one call graph from unconditional call edges only
        let mut adjacency = HashMap::new();
        for function in &function_infos {
            let profile = analyze_function_calls(ctx, function.body_id);
            let mut neighbors = Vec::new();

            // keep the current conservative guard behavior:
            // if a function contains conditionals, skip recursion reporting for it
            if !profile.has_conditional {
                for called_symbol in profile.called_symbols {
                    if function_symbols.contains(&called_symbol) {
                        neighbors.push(called_symbol);
                    }
                }
            }

            neighbors.sort_unstable();
            neighbors.dedup();
            adjacency.insert(function.symbol, neighbors);
        }

        // report one diagnostic for each function in each recursive SCC
        let components = strongly_connected_components(&adjacency);
        for component in components {
            let Some(cycle_path) = find_cycle_path(&adjacency, &component) else {
                continue;
            };

            // resolve cycle label
            let cycle_label = format_cycle_path(&cycle_path, &function_by_symbol);
            let is_self_cycle = component.len() == 1;

            // inspect candidate nodes
            for symbol in component {
                let Some(function) = function_by_symbol.get(&symbol).copied() else {
                    continue;
                };

                // resolve effective lint severity
                let severity = ctx.get_effective_severity(meta, function.decl_id);
                if !severity.is_enabled() {
                    continue;
                }

                // resolve message
                let message = if is_self_cycle {
                    "function unconditionally calls itself"
                } else {
                    "functions form an unconditional recursion cycle"
                };
                let diagnostic = LintReport::new(
                    NO_INFINITE_RECURSION.id,
                    NO_INFINITE_RECURSION.code,
                    NO_INFINITE_RECURSION.category,
                    severity,
                    message,
                    ctx.get_span(function.decl_id),
                )
                .label(format!("recursive cycle path: {cycle_label}"));

                ctx.report(diagnostic);
            }
        }
    }
}

/// Function metadata for recursion checks.
struct FunctionInfo {
    /// The function declaration node.
    decl_id: dir::LocalNodeId<dir::Declaration>,
    /// The function symbol.
    symbol: dir::GlobalSymbolId,
    /// The function body expression.
    body_id: dir::LocalNodeId<dir::Expression>,
    /// Display name used in cycle labels.
    display_name: String,
}

/// Collected call profile for one function body.
#[derive(Default)]
struct FunctionCallProfile {
    /// Whether the body contains conditional control flow.
    has_conditional: bool,
    /// Called function symbols observed in this body.
    called_symbols: HashSet<dir::GlobalSymbolId>,
}

/// Visitor that collects function call symbols and conditional markers.
struct FunctionCallCollector<'a> {
    /// The module being scanned.
    module_id: destack_source::ModuleId,
    /// The type table carrying semantic resolutions.
    types: &'a dir::TypeTable,
    /// Whether conditionals were seen while traversing this body.
    has_conditional: bool,
    /// Called function symbols in this body.
    called_symbols: HashSet<dir::GlobalSymbolId>,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl FunctionCallCollector<'_> {
    /// Walk one function body and collect call profile data.
    fn run(&mut self, tree: &dir::Tree, body_id: dir::LocalNodeId<dir::Expression>) {
        let body = tree.get(body_id);
        self.visit_expression(tree, body_id, body);
    }

    /// Finish collection and return profile data.
    fn finish(self) -> FunctionCallProfile {
        FunctionCallProfile {
            has_conditional: self.has_conditional,
            called_symbols: self.called_symbols,
        }
    }
}

impl NodeVisitor for FunctionCallCollector<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // detect conditional control flow in this function body
        if matches!(
            expression,
            dir::Expression::If { .. } | dir::Expression::Match { .. }
        ) {
            self.has_conditional = true;
        }

        // collect call targets for call graph edges
        if let dir::Expression::Call { left, .. } = expression
            && let Some(target_symbol) = self
                .types
                .symbol_resolution(left.into_global_any(self.module_id))
        {
            self.called_symbols.insert(target_symbol);
        }

        walk_expression(self, tree, id, expression);
    }

    fn visit_declaration(
        &mut self,
        _tree: &dir::Tree,
        _id: dir::LocalNodeId<dir::Declaration>,
        _declaration: &dir::Declaration,
    ) {
        // skip nested function declarations
    }
}

/// Collect all function declarations with bodies in the current module.
fn collect_function_infos(ctx: &LintModuleContext<'_>) -> Vec<FunctionInfo> {
    let mut functions = Vec::new();

    // inspect candidate nodes
    for (decl_id, declaration) in ctx.dir.iter_nodes_of_type::<dir::Declaration>() {
        let dir::Declaration::Function(declaration) = declaration else {
            continue;
        };
        let Some(body_id) = declaration.body else {
            continue;
        };

        // build a readable function name for diagnostics
        let display_name = declaration
            .name
            .map(|name| ctx.strings.get(name.string()).to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());
        let Some(symbol) = ctx.symbol_for_node(decl_id) else {
            continue;
        };
        functions.push(FunctionInfo {
            decl_id,
            symbol,
            body_id,
            display_name,
        });
    }

    functions
}

/// Analyze one function body and return call profile data.
fn analyze_function_calls(
    ctx: &LintModuleContext<'_>,
    body_id: dir::LocalNodeId<dir::Expression>,
) -> FunctionCallProfile {
    let mut collector = FunctionCallCollector {
        module_id: ctx.module_id(),
        types: ctx.types,
        has_conditional: false,
        called_symbols: HashSet::new(),
        options: NodeVisitorOptions::default(),
    };
    collector.run(ctx.dir.tree(), body_id);
    collector.finish()
}

/// Format one cycle path for diagnostics.
fn format_cycle_path(
    cycle_path: &[dir::GlobalSymbolId],
    function_by_symbol: &HashMap<dir::GlobalSymbolId, &FunctionInfo>,
) -> String {
    let mut names = Vec::with_capacity(cycle_path.len());

    // map each symbol to a display name
    for symbol in cycle_path {
        let name = function_by_symbol
            .get(symbol)
            .map(|function| function.display_name.clone())
            .unwrap_or_else(|| "<function>".to_string());
        names.push(name);
    }

    names.join(" -> ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag function that unconditionally calls itself.
    #[test]
    fn test_flags_direct_recursion() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_flags_direct_recursion.ds",
            r#"
function infinite() {
    infinite();
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-infinite-recursion");
    }

    /// Flag function that unconditionally calls itself with arguments.
    #[test]
    fn test_flags_recursion_with_args() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_flags_recursion_with_args.ds",
            r#"
function process(x: number) {
    process(x + 1);
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-infinite-recursion");
    }

    /// Flag unconditional mutual recursion.
    #[test]
    fn test_flags_mutual_recursion_cycle() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_flags_mutual_recursion_cycle.ds",
            r#"
function first() {
    second();
}

function second() {
    first();
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint_count("no-infinite-recursion", 2);
    }

    /// Allow recursion guarded by if statement.
    #[test]
    fn test_allows_conditional_recursion() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_allows_conditional_recursion.ds",
            r#"
function factorial(n: number): number {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-infinite-recursion");
    }

    /// Allow recursion guarded by match.
    #[test]
    fn test_allows_match_recursion() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_allows_match_recursion.ds",
            r#"
function count(n: number): number {
    match (n) {
        0 => 0
        _ => 1 + count(n - 1)
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-infinite-recursion");
    }

    /// Allow conditional mutual recursion.
    #[test]
    fn test_allows_conditional_mutual_recursion() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_allows_conditional_mutual_recursion.ds",
            r#"
function first(value: number) {
    if (value > 0) {
        second(value - 1);
    }
}

function second(value: number) {
    first(value);
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-infinite-recursion");
    }

    /// Allow calling a different function.
    #[test]
    fn test_allows_different_function() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_allows_different_function.ds",
            r#"
function helper(x: number): number {
    return x * 2;
}

function process(x: number): number {
    return helper(x);
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-infinite-recursion");
    }

    /// Allow non-recursive functions.
    #[test]
    fn test_allows_non_recursive() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_allows_non_recursive.ds",
            r#"
function add(a: number, b: number): number {
    return a + b;
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-infinite-recursion");
    }
}
