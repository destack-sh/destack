use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{assign_pattern_target_expression, expression_target_symbol};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow patterns that may pollute Object.prototype.
    ///
    /// Prototype pollution can lead to security vulnerabilities by allowing
    /// attackers to modify object prototypes and affect all objects.
    #[lint(
        id = "no-prototype-pollution",
        code = "LS006",
        category = Security,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Object)],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoPrototypePollution,
    "Disallow prototype pollution patterns"
}

impl LintRule for NoPrototypePollution {
    fn meta(&self) -> &'static LintMeta {
        NoPrototypePollution::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoPrototypePollutionVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags prototype pollution patterns.
struct NoPrototypePollutionVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The Object well-known symbol.
    object_symbol: dir::GlobalSymbolId,
    /// The `prototype` property name.
    prototype_name: StringId,
    /// The `__proto__` property name.
    proto_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoPrototypePollutionVisitor<'a, 'b> {
    /// Build a new visitor.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let prototype_name = ctx.repository.strings.intern("prototype");
        let proto_name = ctx.repository.strings.intern("__proto__");
        let object_symbol = ctx.well_known_symbol(WellKnownSymbol::Object);

        Self {
            ctx,
            meta,
            object_symbol,
            prototype_name,
            proto_name,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module expression roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check an assignment for prototype pollution.
    fn check_assign(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        // check for direct prototype assignment
        if self.is_prototype_assignment_target(left) {
            self.report(expression_id, "direct prototype modification");
        }
    }

    /// Check an index expression for __proto__ access.
    fn check_index(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        index: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        // check for obj["__proto__"]
        let Some(index_id) = index else {
            return;
        };
        let index_expr = self.ctx.tree.get(index_id);
        if let dir::Expression::ScalarLiteral {
            value: dir::ScalarLiteral::String(string_id),
        } = index_expr
            && *string_id == self.proto_name
        {
            self.report(expression_id, "__proto__ access via string");
        }
    }

    /// Check a member expression for prototype access.
    fn check_member(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        _left: dir::LocalNodeId<dir::Expression>,
        name: StringId,
    ) {
        // check for __proto__
        if name == self.proto_name {
            self.report(expression_id, "__proto__ access");
        }
    }

    /// Return true if expression accesses a prototype.
    fn is_prototype_access(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression = self.ctx.tree.get(expression_id);
        if let dir::Expression::Member { left, name, .. } = expression {
            // check for Object.prototype
            if *name == Some(self.prototype_name)
                && let Some(symbol) = expression_target_symbol(self.ctx.tree, *left)
                && symbol == self.object_symbol
            {
                return true;
            }

            // check for __proto__
            return *name == Some(self.proto_name);
        }

        if let dir::Expression::Index { left, right } = expression {
            let Some(right) = right else {
                return false;
            };
            let right_expression = self.ctx.tree.get(*right);
            let dir::Expression::ScalarLiteral {
                value: dir::ScalarLiteral::String(string_id),
            } = right_expression
            else {
                return false;
            };

            if *string_id == self.proto_name {
                return true;
            }

            if *string_id != self.prototype_name {
                return false;
            }

            return expression_target_symbol(self.ctx.tree, *left)
                .is_some_and(|symbol| symbol == self.object_symbol);
        }

        false
    }

    /// Return true when one assignment target writes through a prototype path.
    fn is_prototype_assignment_target(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        if self.is_prototype_access(expression_id) {
            return true;
        }

        let expression = self.ctx.tree.get(expression_id);
        if let dir::Expression::Member { left, .. } = expression {
            return self.is_prototype_access(*left);
        }

        if let dir::Expression::Index { left, .. } = expression {
            return self.is_prototype_access(*left);
        }

        false
    }

    /// Report a prototype pollution diagnostic.
    fn report(&mut self, expression_id: dir::LocalNodeId<dir::Expression>, context: &str) {
        // check effective severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_PROTOTYPE_POLLUTION.id,
                NO_PROTOTYPE_POLLUTION.code,
                NO_PROTOTYPE_POLLUTION.category,
                severity,
                format!("potential prototype pollution via {context}"),
                self.ctx.module.file_id,
                span,
            )
            .with_label("modifying prototypes can lead to security vulnerabilities"),
        );
    }
}

impl NodeVisitor for NoPrototypePollutionVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check assignments
        if let dir::Expression::Assign { left, .. } = expression {
            let Some(left_expression_id) = assign_pattern_target_expression(self.ctx.tree, *left)
            else {
                return;
            };
            self.check_assign(id, left_expression_id);
        }

        // check index expressions
        if let dir::Expression::Index { right, .. } = expression {
            self.check_index(id, *right);
        }

        // check member expressions
        if let dir::Expression::Member {
            left,
            name: Some(name),
            ..
        } = expression
        {
            self.check_member(id, *left, *name);
        }

        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag Object.prototype modification.
    #[test]
    fn test_flags_object_prototype_assignment() {
        let test = TestProgram::for_rule_with_prelude(NoPrototypePollution);
        let result = test.lint_dir(
            "no_prototype_pollution/test_flags_object_prototype_assignment.ds",
            r#"
Object.prototype.foo = "bar";
"#,
        );
        test.result(result).assert_lint("no-prototype-pollution");
    }

    /// Flag __proto__ access.
    #[test]
    fn test_flags_proto_access() {
        let test = TestProgram::for_rule_with_prelude(NoPrototypePollution);
        let result = test.lint_dir(
            "no_prototype_pollution/test_flags_proto_access.ds",
            r#"
let obj = {};
obj.__proto__.foo = "bar";
"#,
        );
        test.result(result).assert_lint("no-prototype-pollution");
    }

    /// Flag string index __proto__ access.
    #[test]
    fn test_flags_string_proto_index() {
        let test = TestProgram::for_rule_with_prelude(NoPrototypePollution);
        let result = test.lint_dir(
            "no_prototype_pollution/test_flags_string_proto_index.ds",
            r#"
let obj = {};
let x = obj["__proto__"];
"#,
        );
        test.result(result).assert_lint("no-prototype-pollution");
    }

    /// Allow normal property access.
    #[test]
    fn test_allows_normal_property_access() {
        let test = TestProgram::for_rule_with_prelude(NoPrototypePollution);
        let result = test.lint_dir(
            "no_prototype_pollution/test_allows_normal_property_access.ds",
            r#"
let obj = {};
obj.foo = "bar";
"#,
        );
        test.result(result).assert_no_lint("no-prototype-pollution");
    }

    /// Allow normal index access.
    #[test]
    fn test_allows_normal_index_access() {
        let test = TestProgram::for_rule_with_prelude(NoPrototypePollution);
        let result = test.lint_dir(
            "no_prototype_pollution/test_allows_normal_index_access.ds",
            r#"
let obj = {};
let x = obj["foo"];
"#,
        );
        test.result(result).assert_no_lint("no-prototype-pollution");
    }

    /// Allow ordinary string index access to a property named prototype.
    #[test]
    fn test_allows_non_object_prototype_index_access() {
        let test = TestProgram::for_rule_with_prelude(NoPrototypePollution);
        let result = test.lint_dir(
            "no_prototype_pollution/test_allows_non_object_prototype_index_access.ds",
            r#"
let obj = {};
let x = obj["prototype"];
"#,
        );
        test.result(result).assert_no_lint("no-prototype-pollution");
    }

    /// Allow safe Object.prototype helper access patterns.
    #[test]
    fn test_allows_safe_object_prototype_read() {
        let test = TestProgram::for_rule_with_prelude(NoPrototypePollution);
        let result = test.lint_dir(
            "no_prototype_pollution/test_allows_safe_object_prototype_read.ds",
            r#"
let obj = {};
let key = "name";
let has = Object.prototype.hasOwnProperty.call(obj, key);
"#,
        );
        test.result(result).assert_no_lint("no-prototype-pollution");
    }

    /// Flag direct Object["prototype"] assignment.
    #[test]
    fn test_flags_object_prototype_index_assignment() {
        let test = TestProgram::for_rule_with_prelude(NoPrototypePollution);
        let result = test.lint_dir(
            "no_prototype_pollution/test_flags_object_prototype_index_assignment.ds",
            r#"
Object["prototype"] = {};
"#,
        );
        test.result(result).assert_lint("no-prototype-pollution");
    }

    /// Allow string index reads of Object prototype.
    #[test]
    fn test_allows_object_prototype_index_access() {
        let test = TestProgram::for_rule_with_prelude(NoPrototypePollution);
        let result = test.lint_dir(
            "no_prototype_pollution/test_allows_object_prototype_index_access.ds",
            r#"
let x = Object["prototype"];
"#,
        );
        test.result(result).assert_no_lint("no-prototype-pollution");
    }
}
