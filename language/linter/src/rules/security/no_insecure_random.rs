use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_repository::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{expression_is_symbol, expression_static_property_access};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow insecure random number generators.
    ///
    /// `Math.random()` is not cryptographically secure.
    #[lint(
        id = "no-insecure-random",
        code = "LS004",
        category = Security,
        level = Dir,
        requires_all = [RequireLibSymbol("Math", &[])],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoInsecureRandom,
    "Disallow Math.random usage"
}

impl LintRule for NoInsecureRandom {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoInsecureRandom::meta()
    }

    /// Check module DIR nodes for Math.random usage.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // walk the module for Math.random calls
        let mut visitor = NoInsecureRandomVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags Math.random usage.
struct NoInsecureRandomVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The Math symbol for this module.
    math_symbol: dir::GlobalSymbolId,
    /// The random member name.
    random_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoInsecureRandomVisitor<'a, 'b> {
    /// Build a visitor for no-insecure-random checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let math_name = ctx.string_id("Math");
        let random_name = ctx.string_id("random");
        let math_symbol = ctx.declared_library_symbol(math_name);

        Self {
            ctx,
            meta,
            math_symbol,
            random_name,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        // capture roots and tree references
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        // walk the module expression tree
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call expression for Math.random usage.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        // match Math.random calls
        if !self.is_math_random(left) {
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
            LintReport::new(
                NO_INSECURE_RANDOM.id,
                NO_INSECURE_RANDOM.code,
                NO_INSECURE_RANDOM.category,
                severity,
                "insecure random number usage",
                span,
            )
            .label("use a cryptographically secure RNG"),
        );
    }

    /// Return true when the expression is Math.random.
    fn is_math_random(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match static property access for random
        let Some((receiver_id, property_name)) =
            expression_static_property_access(self.ctx.dir.tree(), expression_id)
        else {
            return false;
        };
        if property_name != self.random_name {
            return false;
        }

        self.is_math_object(receiver_id)
    }

    /// Return true when the expression is a Math object reference.
    fn is_math_object(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_symbol(self.ctx, expression_id, self.math_symbol)
    }
}

impl NodeVisitor for NoInsecureRandomVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions for Math.random
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

    /// Report Math.random calls.
    #[test]
    fn test_flags_math_random() {
        let test = TestProgram::for_rule_with_prelude(NoInsecureRandom);
        let result = test.lint_dir(
            "no_insecure_random/test_flags_math_random.ds",
            r#"
let value = Math.random();
"#,
        );
        test.result(result).assert_lint("no-insecure-random");
    }

    /// Report global Math.random calls.
    #[test]
    fn test_flags_global_math_random() {
        let test = TestProgram::for_rule_with_prelude(NoInsecureRandom);
        let result = test.lint_dir(
            "no_insecure_random/test_flags_global_math_random.ds",
            r#"
let value = globalThis.Math.random();
"#,
        );
        test.result(result).assert_lint("no-insecure-random");
    }

    /// Allow other Math calls.
    #[test]
    fn test_allows_other_math_call() {
        let test = TestProgram::for_rule_with_prelude(NoInsecureRandom);
        let result = test.lint_dir(
            "no_insecure_random/test_allows_other_math_call.ds",
            r#"
let value = Math.max(1, 2);
"#,
        );
        test.result(result).assert_no_lint("no-insecure-random");
    }

    /// Report computed Math.random calls.
    #[test]
    fn test_flags_computed_math_random() {
        let test = TestProgram::for_rule_with_prelude(NoInsecureRandom);
        let result = test.lint_dir(
            "no_insecure_random/test_flags_computed_math_random.ds",
            r#"
let value = Math["random"]();
"#,
        );
        test.result(result).assert_lint("no-insecure-random");
    }

    /// Report global computed Math.random calls.
    #[test]
    fn test_flags_global_computed_math_random() {
        let test = TestProgram::for_rule_with_prelude(NoInsecureRandom);
        let result = test.lint_dir(
            "no_insecure_random/test_flags_global_computed_math_random.ds",
            r#"
let value = globalThis["Math"]["random"]();
"#,
        );
        test.result(result).assert_lint("no-insecure-random");
    }
}
