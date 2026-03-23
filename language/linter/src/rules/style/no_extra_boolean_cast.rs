use destack_core::StringId;
use destack_dir::{
    self as dir, NodeVisitor, NodeVisitorOptions, UnaryOperator, WellKnownSymbol, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_is_global_qualified_member, expression_target_symbol, expression_type_map,
    expression_unwrap_parenthesized, is_strict_boolean_type,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow redundant boolean casts.
    ///
    /// Casting values already typed as boolean adds noise and does not
    /// change behavior.
    #[lint(
        id = "no-extra-boolean-cast",
        code = "LY067",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Boolean)],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoExtraBooleanCast,
    "Disallow redundant boolean casts"
}

impl LintRule for NoExtraBooleanCast {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoExtraBooleanCast::meta()
    }

    /// Check module DIR nodes for redundant boolean casts.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoExtraBooleanCastVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags redundant boolean casts.
struct NoExtraBooleanCastVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Boolean symbol for this module.
    boolean_symbol: dir::GlobalSymbolId,
    /// The Boolean member name.
    boolean_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoExtraBooleanCastVisitor<'a, 'b> {
    /// Build a visitor for no-extra-boolean-cast checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let boolean_symbol = ctx.well_known_symbol(WellKnownSymbol::Boolean);
        let boolean_name = ctx.program.strings.intern("Boolean");
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            boolean_symbol,
            boolean_name,
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

    /// Report one redundant boolean cast diagnostic.
    fn report(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        replacement_id: dir::LocalNodeId<dir::Expression>,
        message: &'static str,
        label: &'static str,
    ) {
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build a safe source replacement from the original expression
        let span = self.ctx.get_span(expression_id);
        let replacement_span = self.ctx.get_span(replacement_id);
        let replacement_text = self.ctx.get_span_text(replacement_span).to_string();
        let edits = self
            .ctx
            .edit_builder()
            .replace(span, replacement_text)
            .into_edits();
        let fix = LintFix::safe("Remove redundant boolean cast").with_edits(edits);

        // report the redundant cast with an auto fix
        self.ctx.report(
            LintDiagnostic::new(
                NO_EXTRA_BOOLEAN_CAST.id,
                NO_EXTRA_BOOLEAN_CAST.code,
                NO_EXTRA_BOOLEAN_CAST.category,
                severity,
                message,
                self.ctx.module.file_id,
                span,
            )
            .with_label(label)
            .with_fix(fix),
        );
    }

    /// Return true when the expression resolves to the built in Boolean constructor.
    fn is_boolean_reference(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        if let Some(symbol) = expression_target_symbol(self.ctx.tree, expression_id)
            && symbol == self.boolean_symbol
        {
            return true;
        }

        expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.boolean_name,
        )
    }

    /// Return true when the expression type is strictly boolean.
    fn expression_is_strict_boolean(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        expression_type_map(
            &self.ctx.program,
            &self.ctx.artifacts,
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.tree,
            self.ctx.symbols,
            self.ctx.types,
            expression_id,
            is_strict_boolean_type,
        )
        .unwrap_or(false)
    }

    /// Check `Boolean(value)` calls for redundant casts.
    fn check_boolean_constructor_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        callee_id: dir::LocalNodeId<dir::Expression>,
        dynamic_arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        if !self.is_boolean_reference(callee_id) {
            return;
        }

        if dynamic_arguments.len() != 1 {
            return;
        }

        let argument = self.ctx.tree.get(dynamic_arguments[0]);
        let argument_value = argument.value();
        if !self.expression_is_strict_boolean(argument_value) {
            return;
        }

        self.report(
            expression_id,
            argument_value,
            "redundant Boolean() cast",
            "this value is already boolean",
        );
    }

    /// Check `!!value` expressions for redundant casts.
    fn check_double_negation(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        let inner_expression_id = expression_unwrap_parenthesized(self.ctx.tree, right);
        let inner_expression = self.ctx.tree.get(inner_expression_id);
        let dir::Expression::Unary {
            operator: UnaryOperator::Not,
            right: inner_right,
        } = inner_expression
        else {
            return;
        };

        if !self.expression_is_strict_boolean(*inner_right) {
            return;
        }

        self.report(
            expression_id,
            *inner_right,
            "redundant double negation",
            "this value is already boolean",
        );
    }
}

impl NodeVisitor for NoExtraBooleanCastVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check Boolean(value) casts
        if let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        {
            self.check_boolean_constructor_call(id, *left, dynamic_arguments);
        }

        // check !!value casts
        if let dir::Expression::Unary {
            operator: UnaryOperator::Not,
            right,
        } = expression
        {
            self.check_double_negation(id, *right);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Report Boolean(value) when value is already boolean typed.
    #[test]
    fn test_flags_boolean_constructor_on_boolean_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_flags_boolean_constructor_on_boolean_value.ds",
            r#"
let flag: boolean = true;
let value = Boolean(flag);
"#,
        );
        test.result(result).assert_lint("no-extra-boolean-cast");
    }

    /// Safely remove Boolean() casts on boolean values.
    #[test]
    fn test_fix_boolean_constructor_on_boolean_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_fix_boolean_constructor_on_boolean_value.ds",
            r#"
let flag: boolean = true;
let value = Boolean(flag);
"#,
        );
        test.result(result)
            .assert_lint("no-extra-boolean-cast")
            .assert_has_fix("no-extra-boolean-cast")
            .assert_safe_fixed(
                r#"
let flag: boolean = true;
let value = flag;
"#,
            );
    }

    /// Report global Boolean(value) when value is already boolean typed.
    #[test]
    fn test_flags_global_boolean_constructor_on_boolean_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_flags_global_boolean_constructor_on_boolean_value.ds",
            r#"
let flag: boolean = true;
let value = globalThis.Boolean(flag);
"#,
        );
        test.result(result).assert_lint("no-extra-boolean-cast");
    }

    /// Allow Boolean(value) when the value is not already boolean typed.
    #[test]
    fn test_allows_boolean_constructor_on_non_boolean_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_allows_boolean_constructor_on_non_boolean_value.ds",
            r#"
let value = Boolean("name");
"#,
        );
        test.result(result).assert_no_lint("no-extra-boolean-cast");
    }

    /// Report `!!value` when value is already boolean typed.
    #[test]
    fn test_flags_double_negation_on_boolean_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_flags_double_negation_on_boolean_value.ds",
            r#"
let flag: boolean = true;
let value = !!flag;
"#,
        );
        test.result(result).assert_lint("no-extra-boolean-cast");
    }

    /// Safely remove double negation on boolean values.
    #[test]
    fn test_fix_double_negation_on_boolean_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_fix_double_negation_on_boolean_value.ds",
            r#"
let flag: boolean = true;
let value = !!flag;
"#,
        );
        test.result(result)
            .assert_lint("no-extra-boolean-cast")
            .assert_has_fix("no-extra-boolean-cast")
            .assert_safe_fixed(
                r#"
let flag: boolean = true;
let value = flag;
"#,
            );
    }

    /// Allow `!!value` when value is not already boolean typed.
    #[test]
    fn test_allows_double_negation_on_non_boolean_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_allows_double_negation_on_non_boolean_value.ds",
            r#"
let value = !!"name";
"#,
        );
        test.result(result).assert_no_lint("no-extra-boolean-cast");
    }

    /// Allow single negation on boolean values.
    #[test]
    fn test_allows_single_negation() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_allows_single_negation.ds",
            r#"
let flag: boolean = true;
let value = !flag;
"#,
        );
        test.result(result).assert_no_lint("no-extra-boolean-cast");
    }

    /// Allow local Boolean symbols that shadow the global constructor.
    #[test]
    fn test_allows_shadowed_boolean_symbol() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_allows_shadowed_boolean_symbol.ds",
            r#"
function Boolean(value: boolean): boolean {
    return value;
}

let flag: boolean = true;
let result = Boolean(flag);
"#,
        );
        test.result(result).assert_no_lint("no-extra-boolean-cast");
    }

    /// Report Boolean(value) for imported boolean values.
    #[test]
    fn test_flags_boolean_constructor_on_cross_module_boolean_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_extra_boolean_cast/source.ds" => r#"
export const flag: boolean = true;
"#,
                "no_extra_boolean_cast/consumer.ds" => r#"
import { flag } from "./source.ds";
let value = Boolean(flag);
"#,
            },
            "no_extra_boolean_cast/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-extra-boolean-cast");
    }

    /// Allow Boolean(value) when the value can be non boolean.
    #[test]
    fn test_allows_boolean_constructor_on_union_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_allows_boolean_constructor_on_union_value.ds",
            r#"
let value: boolean | string = "x";
let casted = Boolean(value);
"#,
        );
        test.result(result).assert_no_lint("no-extra-boolean-cast");
    }

    /// Report `!!value` for imported boolean values.
    #[test]
    fn test_flags_double_negation_on_cross_module_boolean_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_extra_boolean_cast/source_not.ds" => r#"
export const flag: boolean = true;
"#,
                "no_extra_boolean_cast/consumer_not.ds" => r#"
import { flag } from "./source_not.ds";
let value = !!flag;
"#,
            },
            "no_extra_boolean_cast/consumer_not.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-extra-boolean-cast");
    }

    /// Report redundant casts for boolean literals.
    #[test]
    fn test_flags_boolean_constructor_on_boolean_literal() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_flags_boolean_constructor_on_boolean_literal.ds",
            r#"
let value = Boolean(true);
"#,
        );
        test.result(result).assert_lint("no-extra-boolean-cast");
    }

    /// Allow boolean constructor casts for unknown values.
    #[test]
    fn test_allows_boolean_constructor_on_unknown_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_allows_boolean_constructor_on_unknown_value.ds",
            r#"
let value: unknown = true;
let casted = Boolean(value);
"#,
        );
        test.result(result).assert_no_lint("no-extra-boolean-cast");
    }

    /// Report parenthesized double negation on booleans.
    #[test]
    fn test_flags_parenthesized_double_negation_on_boolean_value() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_flags_parenthesized_double_negation_on_boolean_value.ds",
            r#"
let flag: boolean = true;
let value = !(!flag);
"#,
        );
        test.result(result).assert_lint("no-extra-boolean-cast");
    }
}
