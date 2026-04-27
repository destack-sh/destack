use std::collections::{HashMap, HashSet};

use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{is_simple_identifier, rename_local_symbol_fix};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow parameters that are only used in recursive self calls.
    ///
    /// Parameters that exist only to feed recursion often indicate accidental
    /// API shape and can usually be converted into local temporaries.
    #[lint(
        id = "no-unused-except-recursion",
        code = "LU033",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Experimental,
        declarations = Exclude
    )]
    pub NoUnusedExceptRecursion,
    "Disallow parameters used only for recursive self calls"
}

impl LintRule for NoUnusedExceptRecursion {
    fn meta(&self) -> &'static LintMeta {
        NoUnusedExceptRecursion::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // inspect function declarations
        for declaration_id in ctx.tree.iter_node_ids_of_type::<dir::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            let dir::Declaration::Function(declaration) = declaration else {
                continue;
            };
            let Some(body_expression) = declaration.body else {
                continue;
            };

            report_recursive_only_parameters(
                ctx,
                meta,
                declaration.symbol,
                &declaration.signature,
                body_expression,
            );
        }

        // inspect class and extension methods
        for member_id in ctx.tree.iter_node_ids_of_type::<dir::Member>() {
            let member = ctx.tree.get(member_id);
            let dir::Member::Method {
                symbol,
                signature,
                body: Some(body_expression),
                ..
            } = member
            else {
                continue;
            };

            report_recursive_only_parameters(ctx, meta, *symbol, signature, *body_expression);
        }
    }
}

/// Report parameters used only for recursive self calls in one callable body.
fn report_recursive_only_parameters(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &LintMeta,
    function_symbol: dir::LocalSymbolId,
    signature: &dir::FunctionSignature,
    body_expression_id: dir::LocalNodeId<dir::Expression>,
) {
    let mut parameter_by_symbol = HashMap::new();

    // collect parameters for this callable
    for parameter_id in &signature.parameters {
        let parameter = ctx.tree.get(*parameter_id);
        parameter_by_symbol.insert(parameter.symbol(), *parameter_id);
    }

    // skip callables without parameters
    if parameter_by_symbol.is_empty() {
        return;
    }

    // collect recursive and non recursive parameter usages
    let mut visitor = RecursiveParameterUseVisitor::new(
        ctx.tree,
        ctx.types,
        ctx.module_id(),
        function_symbol.into_global(ctx.module_id()),
        parameter_by_symbol.keys().copied().collect(),
    );
    let body_expression = ctx.tree.get(body_expression_id);
    visitor.visit_expression(ctx.tree, body_expression_id, body_expression);

    // report symbols used only in recursive arguments
    for (parameter_symbol, parameter_id) in parameter_by_symbol {
        // keep symbols observed in recursive arguments
        if !visitor.recursive_used_symbols.contains(&parameter_symbol) {
            continue;
        }

        // skip symbols also used outside recursive arguments
        if visitor
            .non_recursive_used_symbols
            .contains(&parameter_symbol)
        {
            continue;
        }

        let severity = ctx.get_effective_severity(meta, parameter_id);
        if !severity.is_enabled() {
            continue;
        }

        // report recursion only parameter
        let span = ctx.get_span(parameter_id);
        let mut diagnostic = LintDiagnostic::new(
            NO_UNUSED_EXCEPT_RECURSION.id,
            NO_UNUSED_EXCEPT_RECURSION.code,
            NO_UNUSED_EXCEPT_RECURSION.category,
            severity,
            "parameter used only for recursion",
            ctx.module.file_id,
            span,
        )
        .with_label("this parameter is only forwarded into recursive self calls");
        if ctx.include_fixes
            && let Some(replacement_name) =
                recursion_parameter_replacement_name(ctx, parameter_symbol)
            && let Some(fix) = rename_local_symbol_fix(
                ctx,
                parameter_symbol,
                &replacement_name,
                &format!("Rename recursion-only parameter to `{replacement_name}`"),
            )
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        ctx.report(diagnostic);
    }
}

/// Build one underscore-prefixed replacement name for a recursion-only parameter.
fn recursion_parameter_replacement_name(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::LocalSymbolId,
) -> Option<String> {
    let symbol = ctx.symbols.get_symbol(symbol_id);
    let symbol_name_id = symbol.name()?;
    let symbol_name = ctx.repository.strings.get(symbol_name_id).to_string();

    let base_name = if symbol_name.starts_with('_') {
        format!("{symbol_name}Recursive")
    } else {
        format!("_{symbol_name}")
    };
    if !is_simple_identifier(&base_name) {
        return None;
    }

    let scope_id = symbol.scope.0;
    let mut candidate = base_name.clone();
    let mut suffix = 2_u32;
    loop {
        let candidate_id = ctx.repository.strings.intern(&candidate);
        if !scope_subtree_contains_name(ctx, scope_id, candidate_id) {
            return Some(candidate);
        }

        candidate = format!("{base_name}{suffix}");
        suffix += 1;
        if suffix > 1024 {
            return None;
        }
    }
}

/// Return true when a scope or one of its descendants defines a given name.
fn scope_subtree_contains_name(
    ctx: &LintModuleDirContext<'_>,
    scope_id: dir::LocalScopeId,
    name_id: dir::StringId,
) -> bool {
    let mut pending = vec![scope_id];
    let mut visited_scope_ids = HashSet::new();
    while let Some(current_scope_id) = pending.pop() {
        if !visited_scope_ids.insert(current_scope_id) {
            continue;
        }

        let scope = ctx.symbols.get_scope_by_id(current_scope_id);

        for (_, symbol_id) in ctx.symbols.active_named_symbols(scope) {
            let symbol = ctx.symbols.get_symbol(symbol_id);
            if symbol.name() == Some(name_id) {
                return true;
            }
        }

        for child_scope_id in &scope.children {
            pending.push(*child_scope_id);
        }
    }

    false
}

/// Collect parameter symbol usage while tracking recursive call argument context.
struct RecursiveParameterUseVisitor<'a> {
    /// The DIR tree.
    tree: &'a dir::Tree,
    /// The DIR type table.
    types: &'a dir::TypeTable,
    /// The current module id.
    module_id: destack_source::ModuleId,
    /// The current callable symbol.
    function_symbol: dir::GlobalSymbolId,
    /// Parameter symbols for the current callable.
    parameter_symbols: HashSet<dir::LocalSymbolId>,
    /// Symbols used in recursive call argument position.
    recursive_used_symbols: HashSet<dir::LocalSymbolId>,
    /// Symbols used outside recursive call argument position.
    non_recursive_used_symbols: HashSet<dir::LocalSymbolId>,
    /// Nested recursive argument depth.
    recursive_argument_depth: usize,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl<'a> RecursiveParameterUseVisitor<'a> {
    /// Build a visitor for one callable body.
    fn new(
        tree: &'a dir::Tree,
        types: &'a dir::TypeTable,
        module_id: destack_source::ModuleId,
        function_symbol: dir::GlobalSymbolId,
        parameter_symbols: HashSet<dir::LocalSymbolId>,
    ) -> Self {
        Self {
            tree,
            types,
            module_id,
            function_symbol,
            parameter_symbols,
            recursive_used_symbols: HashSet::new(),
            non_recursive_used_symbols: HashSet::new(),
            recursive_argument_depth: 0,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Record one reference usage when it points at a tracked parameter symbol.
    fn record_parameter_usage(&mut self, expression: &dir::Expression) {
        let Some(target_symbol) = expression.target_symbol() else {
            return;
        };

        // keep local parameters only
        if target_symbol.module_id != self.module_id {
            return;
        }
        if !self.parameter_symbols.contains(&target_symbol.local_id) {
            return;
        }

        // split usage by recursive argument context
        if self.recursive_argument_depth > 0 {
            self.recursive_used_symbols.insert(target_symbol.local_id);
            return;
        }

        self.non_recursive_used_symbols
            .insert(target_symbol.local_id);
    }

    /// Return true when this expression resolves to the current callable symbol.
    fn expression_is_recursive_callee(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression = self.tree.get(expression_id);

        // fast path: direct target symbol match
        if expression.target_symbol() == Some(self.function_symbol) {
            return true;
        }

        // fallback: resolution candidate match
        let global_expression_id = expression_id.into_global_any(self.module_id);
        let Some(resolution_id) = self.types.get_resolution_for_node(global_expression_id) else {
            return false;
        };
        let resolution = self.types.get_resolution(resolution_id);
        match resolution {
            dir::Resolution::Static { candidate, .. } => {
                candidate.target_symbol == self.function_symbol
            }
            dir::Resolution::Dynamic { candidates, .. } => candidates
                .iter()
                .any(|candidate| candidate.target_symbol == self.function_symbol),
            dir::Resolution::Unresolved { .. } | dir::Resolution::Builtin { .. } => false,
        }
    }
}

impl NodeVisitor for RecursiveParameterUseVisitor<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // record parameter references at this expression
        self.record_parameter_usage(expression);

        // call arguments need recursive-context tracking
        if let dir::Expression::Call {
            left,
            generic_arguments,
            arguments,
        } = expression
        {
            // visit callee first
            let left_expression = tree.get(*left);
            self.visit_expression(tree, *left, left_expression);

            let is_recursive_call = self.expression_is_recursive_callee(*left);

            // visit static arguments in recursive context when needed
            for argument_id in generic_arguments {
                let argument = tree.get(*argument_id);
                let argument_expression_id = match argument {
                    dir::GenericArgument::Type { .. } => continue,
                    dir::GenericArgument::Value { value } => *value,
                    dir::GenericArgument::Error => continue,
                };
                if is_recursive_call {
                    self.recursive_argument_depth += 1;
                }
                let argument_expression = tree.get(argument_expression_id);
                self.visit_expression(tree, argument_expression_id, argument_expression);
                if is_recursive_call {
                    self.recursive_argument_depth -= 1;
                }
            }

            // visit arguments in recursive context when needed
            for argument_id in arguments {
                let argument = tree.get(*argument_id);
                if is_recursive_call {
                    self.recursive_argument_depth += 1;
                }
                let argument_expression_id = argument.value();
                let argument_expression = tree.get(argument_expression_id);
                self.visit_expression(tree, argument_expression_id, argument_expression);
                if is_recursive_call {
                    self.recursive_argument_depth -= 1;
                }
            }

            return;
        }

        // recurse through non call expressions
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag a parameter used only in direct recursive self calls.
    #[test]
    fn test_flags_parameter_used_only_for_recursion() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_flags_parameter_used_only_for_recursion.ds",
            r#"
function recurse(value: int32): int32 {
    return recurse(value);
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-except-recursion")
            .assert_lint_count("no-unused-except-recursion", 1);
    }

    /// Allow parameters that are used outside recursive argument positions.
    #[test]
    fn test_allows_parameter_used_outside_recursion() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_allows_parameter_used_outside_recursion.ds",
            r#"
function recurse(value: int32): int32 {
    const next = value + 1;
    return recurse(next);
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unused-except-recursion");
    }

    /// Ignore non-recursive functions.
    #[test]
    fn test_ignores_non_recursive_function() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_ignores_non_recursive_function.ds",
            r#"
function value(input: int32): int32 {
    return input;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unused-except-recursion");
    }

    /// Flag method parameters used only in method self recursion.
    #[test]
    fn test_flags_method_parameter_used_only_for_recursion() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_flags_method_parameter_used_only_for_recursion.ds",
            r#"
class Counter {
    run(value: int32): int32 {
        return this.run(value);
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-except-recursion");
    }

    /// Report only the parameters that are recursion-only in multi-parameter functions.
    #[test]
    fn test_reports_only_recursion_only_parameters() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_reports_only_recursion_only_parameters.ds",
            r#"
function recurse(value: int32, limit: int32): int32 {
    if (limit <= 0) {
        return 0;
    }

    return recurse(value, limit - 1);
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-except-recursion")
            .assert_lint_count("no-unused-except-recursion", 1);
    }

    /// Allow parameters passed to non-recursive calls.
    #[test]
    fn test_allows_parameter_used_in_non_recursive_call() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_allows_parameter_used_in_non_recursive_call.ds",
            r#"
function helper(value: int32): int32 {
    return value;
}

function recurse(value: int32): int32 {
    const forwarded = helper(value);
    return recurse(forwarded);
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unused-except-recursion");
    }

    /// Allow parameters referenced by nested callbacks.
    #[test]
    fn test_allows_parameter_used_in_nested_callback() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_allows_parameter_used_in_nested_callback.ds",
            r#"
function recurse(value: int32): int32 {
    const read = () => value;
    return recurse(value + read());
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unused-except-recursion");
    }

    /// Flag multiple recursion-only parameters in one recursive function.
    #[test]
    fn test_flags_multiple_recursion_only_parameters() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_flags_multiple_recursion_only_parameters.ds",
            r#"
function recurse(left: int32, right: int32): int32 {
    return recurse(left + 1, right + 1);
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-except-recursion")
            .assert_lint_count("no-unused-except-recursion", 2);
    }

    /// Suggest renaming recursion-only parameters to underscore-prefixed names.
    #[test]
    fn test_fix_renames_recursion_only_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_fix_renames_recursion_only_parameter.ds",
            r#"
function recurse(value: int32): int32 {
    return recurse(value);
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-except-recursion")
            .assert_has_fix("no-unused-except-recursion")
            .assert_unsafe_fixed(
                r#"
function recurse(_value: int32): int32 {
    return recurse(_value);
}
"#,
            );
    }

    /// Use recursive suffix for leading-underscore parameters.
    #[test]
    fn test_fix_uses_recursive_suffix_for_underscored_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_fix_uses_recursive_suffix_for_underscored_parameter.ds",
            r#"
function recurse(_value: int32): int32 {
    return recurse(_value);
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-except-recursion")
            .assert_has_fix("no-unused-except-recursion")
            .assert_unsafe_fixed(
                r#"
function recurse(_valueRecursive: int32): int32 {
    return recurse(_valueRecursive);
}
"#,
            );
    }

    /// Pick a stable suffix when the preferred replacement already exists.
    #[test]
    fn test_fix_uses_suffix_when_recursive_replacement_exists() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedExceptRecursion);
        let result = test.lint_dir(
            "no_unused_except_recursion/test_fix_uses_suffix_when_recursive_replacement_exists.ds",
            r#"
function recurse(_value: int32): int32 {
    let _valueRecursive = 1;
    return recurse(_value);
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-except-recursion")
            .assert_has_fix("no-unused-except-recursion")
            .assert_unsafe_fixed(
                r#"
function recurse(_valueRecursive2: int32): int32 {
    let _valueRecursive = 1;
    return recurse(_valueRecursive2);
}
"#,
            );
    }
}
