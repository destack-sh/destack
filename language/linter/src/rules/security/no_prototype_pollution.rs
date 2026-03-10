use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::expression_target_symbol;
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
        let prototype_name = ctx.program.strings.intern("prototype");
        let proto_name = ctx.program.strings.intern("__proto__");
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
        if self.is_prototype_access(left) {
            self.report(expression_id, "direct prototype modification");
            return;
        }

        // check for dynamic property on prototype
        let expression = self.ctx.tree.get(left);
        if let dir::Expression::Index { left: target, .. } = expression
            && self.is_prototype_access(*target)
        {
            self.report(expression_id, "dynamic prototype property assignment");
        }
    }

    /// Check an index expression for __proto__ access.
    fn check_index(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        index: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        // check for obj["__proto__"] or obj["prototype"]
        let Some(index_id) = index else {
            return;
        };
        let index_expr = self.ctx.tree.get(index_id);
        if let dir::Expression::ScalarLiteral {
            value: dir::ScalarLiteral::String(string_id),
        } = index_expr
            && (*string_id == self.proto_name || *string_id == self.prototype_name)
        {
            self.report(expression_id, "__proto__ or prototype access via string");
        }
    }

    /// Check a member expression for prototype access.
    fn check_member(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        name: StringId,
    ) {
        // check for Object.prototype
        if name == self.prototype_name
            && let Some(symbol) = expression_target_symbol(self.ctx.tree, left)
            && symbol == self.object_symbol
        {
            self.report(expression_id, "Object.prototype access");
        }

        // check for __proto__
        if name == self.proto_name {
            self.report(expression_id, "__proto__ access");
        }
    }

    /// Return true if expression accesses a prototype.
    fn is_prototype_access(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Member { left, name, .. } = expression else {
            return false;
        };

        // check for Object.prototype
        if *name == self.prototype_name
            && let Some(symbol) = expression_target_symbol(self.ctx.tree, *left)
            && symbol == self.object_symbol
        {
            return true;
        }

        // check for __proto__
        *name == self.proto_name
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
            self.check_assign(id, *left);
        }

        // check index expressions
        if let dir::Expression::Index { right, .. } = expression {
            self.check_index(id, *right);
        }

        // check member expressions
        if let dir::Expression::Member { left, name, .. } = expression {
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
}
