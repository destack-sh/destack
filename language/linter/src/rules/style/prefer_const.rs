use std::collections::HashSet;

use destack_dir::{
    self as dir, GlobalSymbolId, LocalNodeId, LocalSymbolId, Mutability, NodeVisitor,
    NodeVisitorOptions, walk_expression,
};
use destack_source::ModuleId;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    collect_pattern_value_binding_symbols, expression_assignment_target, expression_target_symbol,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Require `const` declarations for never reassigned variables.
    ///
    /// Using `const` for variables that are never reassigned helps clarify
    /// intent and can catch accidental reassignments.
    #[lint(
        id = "prefer-const",
        code = "LY035",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferConst,
    "Require const for never-reassigned variables"
}

impl LintRule for PreferConst {
    fn meta(&self) -> &'static LintMeta {
        PreferConst::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // collect mutable declaration symbols by declaration site
        let mut collector = LetDeclarationCollector::new(ctx);
        collector.run();
        let let_declarations = collector.let_declarations;

        if let_declarations.is_empty() {
            return;
        }

        // collect reassignments for the declared symbols
        let mut scanner = ReassignmentScanner::new(ctx, &let_declarations);
        scanner.run();
        let assigned_symbols = scanner.assigned_symbols;

        // report declarations when all bound symbols stay immutable
        for declaration in let_declarations {
            if declaration
                .symbols
                .iter()
                .any(|symbol_id| assigned_symbols.contains(symbol_id))
            {
                continue;
            }

            // honor per node severity
            let severity = ctx.get_effective_severity(meta, declaration.expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = ctx.get_span(declaration.expression_id);
            let mut diagnostic = LintDiagnostic::new(
                PREFER_CONST.id,
                PREFER_CONST.code,
                PREFER_CONST.category,
                severity,
                "use const instead of let",
                ctx.module.file_id,
                span,
            )
            .with_label("this variable is never reassigned");

            // rewrite mutable declarations to const when possible
            if ctx.include_fixes
                && let Some(fix) = build_prefer_const_fix(ctx, declaration.expression_id)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// One mutable declaration and its bound symbols.
struct LetDeclaration {
    /// The declaration expression id.
    expression_id: LocalNodeId<dir::Expression>,
    /// All value symbols declared in the expression.
    symbols: Vec<GlobalSymbolId>,
}

/// Build a safe rewrite from `let` or `var` to `const`.
fn build_prefer_const_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let span = ctx.get_span(expression_id);
    let expression_text = ctx.get_span_text(span);
    let replacement = replace_binding_keyword_with_const(expression_text)?;
    let edits = ctx.edit_builder().replace(span, replacement).into_edits();
    Some(LintFix::safe("Replace with const declaration").with_edits(edits))
}

/// Replace the leading mutable declaration keyword with `const`.
fn replace_binding_keyword_with_const(expression_text: &str) -> Option<String> {
    if let Some(rest) = expression_text.strip_prefix("let") {
        return Some(format!("const{rest}"));
    }

    if let Some(rest) = expression_text.strip_prefix("var") {
        return Some(format!("const{rest}"));
    }

    None
}

/// Collector for let declarations.
struct LetDeclarationCollector<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The module ID for creating GlobalSymbolIds.
    module_id: ModuleId,
    /// Collected let declarations with all bound symbols.
    let_declarations: Vec<LetDeclaration>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> LetDeclarationCollector<'a, 'b> {
    /// Build a collector for let declarations.
    fn new(ctx: &'a mut LintModuleDirContext<'b>) -> Self {
        let module_id = ctx.module_id();

        Self {
            ctx,
            module_id,
            let_declarations: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Convert a local symbol ID to a global symbol ID.
    fn to_global(&self, local_id: LocalSymbolId) -> GlobalSymbolId {
        GlobalSymbolId::new(self.module_id, local_id)
    }
}

impl NodeVisitor for LetDeclarationCollector<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for mutable let declarations
        if let dir::Expression::Let {
            mutability: Mutability::Mutable,
            declarators,
            ..
        } = expression
        {
            // collect value symbols from all declarator patterns
            let mut declaration_symbols = HashSet::new();
            for declarator_id in declarators {
                let declarator = tree.get(*declarator_id);
                collect_pattern_value_binding_symbols(
                    tree,
                    self.ctx.symbols,
                    declarator.pattern,
                    &mut declaration_symbols,
                );
            }

            if !declaration_symbols.is_empty() {
                let symbols = declaration_symbols
                    .into_iter()
                    .map(|local_symbol| self.to_global(local_symbol))
                    .collect();
                self.let_declarations.push(LetDeclaration {
                    expression_id: id,
                    symbols,
                });
            }
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Scanner for reassignments to specific symbols.
struct ReassignmentScanner<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// Symbols to look for reassignments to.
    target_symbols: HashSet<GlobalSymbolId>,
    /// Symbols that have been reassigned.
    assigned_symbols: HashSet<GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ReassignmentScanner<'a, 'b> {
    /// Build a scanner for reassignments to specific symbols.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, let_declarations: &[LetDeclaration]) -> Self {
        let target_symbols = let_declarations
            .iter()
            .flat_map(|declaration| declaration.symbols.iter().copied())
            .collect();

        Self {
            ctx,
            target_symbols,
            assigned_symbols: HashSet::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check if the assignment target is a tracked symbol.
    fn check_assignment_target(&mut self, target_id: LocalNodeId<dir::Expression>) {
        if let Some(symbol) = expression_target_symbol(self.ctx.tree, target_id)
            && self.target_symbols.contains(&symbol)
        {
            self.assigned_symbols.insert(symbol);
        }
    }
}

impl NodeVisitor for ReassignmentScanner<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // record assignment targets for tracked symbols
        if let Some(target_id) = expression_assignment_target(expression) {
            self.check_assignment_target(target_id);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag let that is never reassigned.
    #[test]
    fn test_flags_never_reassigned_let() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_flags_never_reassigned_let.ds",
            r#"
let x = 1;
let y = x + 1;
"#,
        );
        test.result(result).assert_lint_count("prefer-const", 2);
    }

    /// Safely rewrite `let` bindings that are never reassigned.
    #[test]
    fn test_fix_rewrites_let_to_const() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_fix_rewrites_let_to_const.ds",
            r#"
let value = 1;
"#,
        );
        test.result(result)
            .assert_lint("prefer-const")
            .assert_has_fix("prefer-const")
            .assert_safe_fixed(
                r#"
const value = 1;
"#,
            );
    }

    /// Safely rewrite legacy mutable declarations that are never reassigned.
    #[test]
    fn test_mutation_fix_rewrites_var_to_const() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_mutation_fix_rewrites_var_to_const.ds",
            r#"
var value = 1;
"#,
        );
        test.result(result)
            .assert_lint("prefer-const")
            .assert_has_fix("prefer-const")
            .assert_safe_fixed(
                r#"
const value = 1;
"#,
            );
    }

    /// Allow let that is reassigned.
    #[test]
    fn test_allows_reassigned_let() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_reassigned_let.ds",
            r#"
let x = 1;
x = 2;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow let with compound assignment.
    #[test]
    fn test_allows_compound_assignment() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_compound_assignment.ds",
            r#"
let x = 1;
x += 1;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow let with increment.
    #[test]
    fn test_allows_increment() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_increment.ds",
            r#"
let x = 1;
x++;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow let with pre increment.
    #[test]
    fn test_allows_pre_increment() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_pre_increment.ds",
            r#"
let x = 1;
++x;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow const declarations.
    #[test]
    fn test_allows_const() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_const.ds",
            r#"
const x = 1;
const y = x + 1;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Mixed let and const.
    #[test]
    fn test_mixed_declarations() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_mixed_declarations.ds",
            r#"
let x = 1;
let y = 2;
y = 3;
const z = x + y;
"#,
        );
        // only x should be flagged (y is reassigned, z is already const)
        test.result(result).assert_lint_count("prefer-const", 1);
    }

    /// Allow let with decrement.
    #[test]
    fn test_allows_decrement() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_decrement.ds",
            r#"
let x = 10;
x--;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow let reassigned in nested scope.
    #[test]
    fn test_allows_reassigned_in_nested_scope() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_reassigned_in_nested_scope.ds",
            r#"
let x = 1;
if (true) {
    x = 2;
}
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Flag let in function that is never reassigned.
    #[test]
    fn test_flags_let_in_function() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_flags_let_in_function.ds",
            r#"
function foo(): number {
    let x = 1;
    return x;
}
"#,
        );
        test.result(result).assert_lint("prefer-const");
    }

    /// Allow let reassigned with other compound operators.
    #[test]
    fn test_allows_other_compound_operators() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_other_compound_operators.ds",
            r#"
let a = 10;
let b = 20;
let c = 5;
a -= 1;
b *= 2;
c /= 1;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Avoid mixed declaration fixes when one declarator is reassigned.
    #[test]
    fn test_allows_mixed_mutability_in_one_declaration() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_mixed_mutability_in_one_declaration.ds",
            r#"
let a = 1, b = 2;
b = 3;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Flag destructured let declarations that are never reassigned.
    #[test]
    fn test_flags_destructured_let() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_flags_destructured_let.ds",
            r#"
let { a, b } = value;
"#,
        );
        test.result(result)
            .assert_lint("prefer-const")
            .assert_has_fix("prefer-const")
            .assert_safe_fixed(
                r#"
const { a, b } = value;
"#,
            );
    }
}
