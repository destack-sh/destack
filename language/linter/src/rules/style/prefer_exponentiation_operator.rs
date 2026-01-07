use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{expression_is_global_qualified_member, expression_target_symbol};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `**` over `Math.pow()`.
    ///
    /// The exponentiation operator `**` is more concise and readable than
    /// `Math.pow()`. It also handles BigInt values correctly.
    #[lint(
        id = "prefer-exponentiation-operator",
        code = "LY043",
        category = Style,
        level = Dir,
        requires_all = [RequireLibSymbol("Math", &[])],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferExponentiationOperator,
    "Prefer ** over Math.pow()"
}

impl LintRule for PreferExponentiationOperator {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferExponentiationOperator::meta()
    }

    /// Check module DIR nodes for Math.pow usage.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = ExponentiationVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags Math.pow usage.
struct ExponentiationVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The Math symbol for this module.
    math_symbol: dir::GlobalSymbolId,
    /// The Math member name.
    math_name: StringId,
    /// The pow member name.
    pow_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ExponentiationVisitor<'a, 'b> {
    /// Build a visitor for prefer-exponentiation-operator checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let math_name = ctx.program.strings.intern("Math");
        let pow_name = ctx.program.strings.intern("pow");
        let math_symbol = ctx.declared_lib_symbol(math_name);
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            math_symbol,
            math_name,
            pow_name,
            global_qualifiers,
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

    /// Check a call expression for Math.pow usage.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        // match Math.pow calls
        if !self.is_math_pow(left) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                PREFER_EXPONENTIATION_OPERATOR.id,
                PREFER_EXPONENTIATION_OPERATOR.code,
                PREFER_EXPONENTIATION_OPERATOR.category,
                severity,
                "prefer ** operator over Math.pow()",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use base ** exponent instead"),
        );
    }

    /// Return true when the expression is Math.pow.
    fn is_math_pow(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match member expressions
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Member { left, name, .. } = expression else {
            return false;
        };
        if *name != self.pow_name {
            return false;
        }

        self.is_math_object(*left)
    }

    /// Return true when the expression is a Math object reference.
    fn is_math_object(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match direct symbol references
        let target_symbol = expression_target_symbol(self.ctx.tree, expression_id);
        if target_symbol == Some(self.math_symbol) {
            return true;
        }

        // match global qualified references
        expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.math_name,
        )
    }
}

impl NodeVisitor for ExponentiationVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions for Math.pow
        if let dir::Expression::Call { left, .. } = expression {
            self.check_call(id, *left);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_math_pow() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "test.ds",
            r#"
let value = Math.pow(2, 3);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator");
    }

    #[test]
    fn test_flags_global_math_pow() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "test.ds",
            r#"
let value = globalThis.Math.pow(2, 3);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator");
    }

    #[test]
    fn test_allows_exponentiation_operator() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "test.ds",
            r#"
let value = 2 ** 3;
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-exponentiation-operator");
    }

    #[test]
    fn test_allows_other_math_call() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "test.ds",
            r#"
let value = Math.max(1, 2);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-exponentiation-operator");
    }
}
