use destack_ast::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_method_call, expression_target_symbol, has_useful_to_string_type,
};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `.toString()` on objects without useful representation.
    ///
    /// Calling `.toString()` on plain objects or types without a custom
    /// `toString()` method returns `"[object Object]"`, which is rarely
    /// useful. Consider implementing a custom `toString()` method or using
    /// a different approach like `JSON.stringify()`.
    #[lint(
        id = "no-base-to-string",
        code = "LC006",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoBaseToString,
    "Disallow toString on objects without useful representation"
}

impl LintRule for NoBaseToString {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoBaseToString::meta()
    }

    /// Check module DIR nodes for base toString calls.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata and well known names
        let meta = self.meta();
        let to_string_name = ctx.string_id("toString");
        let to_locale_string_name = ctx.string_id("toLocaleString");
        let join_name = ctx.string_id("join");
        let string_symbol = ctx.get_well_known_symbol(WellKnownSymbol::String);

        // walk module expressions
        let mut visitor = BaseToStringVisitor::new(
            ctx,
            meta,
            to_string_name,
            to_locale_string_name,
            join_name,
            string_symbol,
        );
        visitor.run();
    }
}

/// Visitor that flags `.toString()` on objects without useful representation.
struct BaseToStringVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The interned "toString" name.
    to_string_name: StringId,
    /// The interned "toLocaleString" name.
    to_locale_string_name: StringId,
    /// The interned "join" name.
    join_name: StringId,
    /// The optional well known String symbol.
    string_symbol: Option<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> BaseToStringVisitor<'a, 'b> {
    /// Build a visitor for base toString checks.
    fn new(
        ctx: &'a mut LintModuleDirContext<'b>,
        meta: &'a LintMeta,
        to_string_name: StringId,
        to_locale_string_name: StringId,
        join_name: StringId,
        string_symbol: Option<dir::GlobalSymbolId>,
    ) -> Self {
        Self {
            ctx,
            meta,
            to_string_name,
            to_locale_string_name,
            join_name,
            string_symbol,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a method call for base toString usage.
    fn check_to_string_like(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // match method call pattern
        let Some(method_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };

        // check if this is toString() or toLocaleString()
        if method_call.method_name != self.to_string_name
            && method_call.method_name != self.to_locale_string_name
        {
            return;
        }

        // get receiver type
        let Some(type_id) = self.ctx.expression_type_id(method_call.receiver_id) else {
            return;
        };

        // check if the receiver has useful toString
        if has_useful_to_string_type(self.ctx.types, type_id) {
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
                NO_BASE_TO_STRING.id,
                NO_BASE_TO_STRING.code,
                NO_BASE_TO_STRING.category,
                severity,
                "toString() may produce '[object Object]'",
                span,
            )
            .label("this type has no useful toString representation"),
        );
    }

    /// Check global String(value) calls for base object stringification.
    fn check_string_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // resolve required String symbol
        let Some(string_symbol) = self.string_symbol else {
            return;
        };

        // match call expression
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            left, arguments, ..
        } = expression
        else {
            return;
        };

        // ensure this is a call to global String
        let Some(target_symbol) = expression_target_symbol(self.ctx.tree, *left) else {
            return;
        };
        if target_symbol != string_symbol {
            return;
        }

        // resolve first argument type
        let Some(argument_id) = arguments.first() else {
            return;
        };
        let argument = self.ctx.tree.get(*argument_id);
        let argument_value = argument.value();
        let Some(type_id) = self.ctx.expression_type_id(argument_value) else {
            return;
        };

        // ignore useful stringification targets
        if has_useful_to_string_type(self.ctx.types, type_id) {
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
                NO_BASE_TO_STRING.id,
                NO_BASE_TO_STRING.code,
                NO_BASE_TO_STRING.category,
                severity,
                "String() may produce '[object Object]'",
                span,
            )
            .label("this value has no useful toString representation"),
        );
    }

    /// Check join() calls for array element stringification.
    fn check_join_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // match method call pattern
        let Some(method_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };
        if method_call.method_name != self.join_name {
            return;
        }

        // resolve receiver type
        let Some(type_id) = self.ctx.expression_type_id(method_call.receiver_id) else {
            return;
        };
        if has_useful_to_string_type(self.ctx.types, type_id) {
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
                NO_BASE_TO_STRING.id,
                NO_BASE_TO_STRING.code,
                NO_BASE_TO_STRING.category,
                severity,
                "join() may stringify elements as '[object Object]'",
                span,
            )
            .label("array elements should have useful toString representations"),
        );
    }

    /// Check template interpolations for base object stringification.
    fn check_template_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        template: &dir::TemplateLiteral,
    ) {
        let dir::TemplateLiteral::InterpolatedString { arguments, .. } = template else {
            return;
        };

        // inspect candidate nodes
        for argument_id in arguments {
            let argument = self.ctx.tree.get(*argument_id);
            let value_expression_id = argument.value();
            let Some(type_id) = self.ctx.expression_type_id(value_expression_id) else {
                continue;
            };
            if has_useful_to_string_type(self.ctx.types, type_id) {
                continue;
            }

            // honor per node severity
            let severity = self.ctx.get_effective_severity(self.meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // report the diagnostic
            let span = self.ctx.get_span(value_expression_id);
            self.ctx.report(
                LintReport::new(
                    NO_BASE_TO_STRING.id,
                    NO_BASE_TO_STRING.code,
                    NO_BASE_TO_STRING.category,
                    severity,
                    "template interpolation may produce '[object Object]'",
                    span,
                )
                .label("this value has no useful toString representation"),
            );
        }
    }
}

impl NodeVisitor for BaseToStringVisitor<'_, '_> {
    /// Return visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Visit an expression node.
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_to_string_like(id);
            self.check_string_call(id);
            self.check_join_call(id);
        }

        // check template interpolations
        if let dir::Expression::TemplateExpression { value } = expression {
            self.check_template_expression(id, value);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag toString on plain object.
    #[test]
    fn test_flags_object_to_string() {
        let test = TestProgram::for_rule_without_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_flags_object_to_string.ds",
            r#"
let obj = { x: 1, y: 2 };
let str = obj.toString();
"#,
        );
        test.result(result).assert_lint("no-base-to-string");
    }

    /// Allow toString on string.
    #[test]
    fn test_allows_string_to_string() {
        let test = TestProgram::for_rule_without_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_allows_string_to_string.ds",
            r#"
let str = "hello";
let result = str.toString();
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }

    /// Allow toString on number.
    #[test]
    fn test_allows_number_to_string() {
        let test = TestProgram::for_rule_without_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_allows_number_to_string.ds",
            r#"
let num: int32 = 42;
let str = num.toString();
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }

    /// Allow toString on array.
    #[test]
    fn test_allows_array_to_string() {
        let test = TestProgram::for_rule_without_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_allows_array_to_string.ds",
            r#"
let arr = [1, 2, 3];
let str = arr.toString();
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }

    /// Flag toLocaleString on plain object.
    #[test]
    fn test_flags_object_to_locale_string() {
        let test = TestProgram::for_rule_without_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_flags_object_to_locale_string.ds",
            r#"
let obj = { x: 1, y: 2 };
let str = obj.toLocaleString();
"#,
        );
        test.result(result).assert_lint("no-base-to-string");
    }

    /// Allow toLocaleString on string.
    #[test]
    fn test_allows_string_to_locale_string() {
        let test = TestProgram::for_rule_without_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_allows_string_to_locale_string.ds",
            r#"
let str = "hello";
let result = str.toLocaleString();
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }

    /// Flag String conversion for plain object values.
    #[test]
    fn test_flags_string_call_on_object() {
        let test = TestProgram::for_rule_with_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_flags_string_call_on_object.ds",
            r#"
let obj = { x: 1, y: 2 };
let str = String(obj);
"#,
        );
        test.result(result).assert_lint("no-base-to-string");
    }

    /// Allow String conversion for primitives.
    #[test]
    fn test_allows_string_call_on_number() {
        let test = TestProgram::for_rule_with_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_allows_string_call_on_number.ds",
            r#"
let value: int32 = 42;
let str = String(value);
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }

    /// Allow shadowed String function calls.
    #[test]
    fn test_allows_shadowed_string_call() {
        let test = TestProgram::for_rule_with_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_allows_shadowed_string_call.ds",
            r#"
let String = (value: { x: int32 }): string => {
    return "value";
};

let value = { x: 1 };
let str = String(value);
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }

    /// Allow Error stringification.
    #[test]
    fn test_allows_error_to_string() {
        let test = TestProgram::for_rule_with_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_allows_error_to_string.ds",
            r#"
let error = new Error("failed");
let text = error.toString();
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }

    /// Allow String conversion for Error values.
    #[test]
    fn test_allows_string_call_on_error() {
        let test = TestProgram::for_rule_with_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_allows_string_call_on_error.ds",
            r#"
let error = new Error("failed");
let text = String(error);
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }

    /// Flag template interpolation for plain object values.
    #[test]
    fn test_flags_template_interpolation_object() {
        let test = TestProgram::for_rule_without_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_flags_template_interpolation_object.ds",
            r#"
let obj = { x: 1, y: 2 };
let text = `${obj}`;
"#,
        );
        test.result(result).assert_lint("no-base-to-string");
    }

    /// Allow template interpolation for string values.
    #[test]
    fn test_allows_template_interpolation_string() {
        let test = TestProgram::for_rule_without_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_allows_template_interpolation_string.ds",
            r#"
let value = "hello";
let text = `${value}`;
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }

    /// Flag join() on object arrays.
    #[test]
    fn test_flags_join_on_object_array() {
        let test = TestProgram::for_rule_without_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_flags_join_on_object_array.ds",
            r#"
let values = [{ x: 1 }, { x: 2 }];
let text = values.join(",");
"#,
        );
        test.result(result).assert_lint("no-base-to-string");
    }

    /// Allow join() on string arrays.
    #[test]
    fn test_allows_join_on_string_array() {
        let test = TestProgram::for_rule_without_prelude(NoBaseToString);
        let result = test.lint_dir(
            "no_base_to_string/test_allows_join_on_string_array.ds",
            r#"
let values = ["a", "b"];
let text = values.join(",");
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }
}
