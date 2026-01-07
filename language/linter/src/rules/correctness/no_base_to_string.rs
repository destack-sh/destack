use destack_ast::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_method_call;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `.toString()` on objects without useful representation.
    ///
    /// Calling `.toString()` on plain objects or types without a custom
    /// `toString()` method returns `"[object Object]"`, which is rarely
    /// useful. Consider implementing a custom `toString()` method or using
    /// a different approach like `JSON.stringify()`.
    #[lint(
        id = "no-base-to-string",
        code = "LC008",
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
        let meta = self.meta();
        let to_string_name = ctx.program.strings.intern("toString");
        let mut visitor = BaseToStringVisitor::new(ctx, meta, to_string_name);
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
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> BaseToStringVisitor<'a, 'b> {
    /// Build a visitor for base toString checks.
    fn new(
        ctx: &'a mut LintModuleDirContext<'b>,
        meta: &'a LintMeta,
        to_string_name: StringId,
    ) -> Self {
        Self {
            ctx,
            meta,
            to_string_name,
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

    /// Check if a type has a useful toString representation.
    fn has_useful_to_string(&self, type_id: dir::LocalTypeId) -> bool {
        let ty = self.ctx.types.get_type(type_id);

        match ty {
            // primitives have useful toString
            dir::Type::TypeLiteral { value } => matches!(
                value,
                dir::TypeLiteral::Primitive(dir::PrimitiveType::String)
                    | dir::TypeLiteral::Primitive(dir::PrimitiveType::Number)
                    | dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean)
                    | dir::TypeLiteral::Primitive(dir::PrimitiveType::Int(_))
                    | dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(_))
                    | dir::TypeLiteral::Primitive(dir::PrimitiveType::Bigint)
                    | dir::TypeLiteral::ScalarLiteral(_)
            ),
            // arrays and tuples have useful toString
            dir::Type::Array { .. } | dir::Type::ArraySized { .. } | dir::Type::Tuple { .. } => {
                true
            }
            // follow type wrappers
            dir::Type::Value { value } => self.has_useful_to_string(*value),
            dir::Type::Mutable { right, .. }
            | dir::Type::ValueOf { right, .. }
            | dir::Type::ReferenceOf { right, .. }
            | dir::Type::PointerOf { right, .. } => self.has_useful_to_string(*right),
            // unions are safe if all elements have useful toString
            dir::Type::Union { elements } => elements.iter().all(|e| self.has_useful_to_string(*e)),
            // intersections are safe if any element has useful toString
            dir::Type::Intersection { elements } => {
                elements.iter().any(|e| self.has_useful_to_string(*e))
            }
            // references need to check if they're strings or numbers
            dir::Type::Reference { symbol, .. } => {
                // check if it's a well-known type with useful toString
                self.is_known_to_string_type(*symbol)
            }
            // plain objects don't have useful toString
            dir::Type::Object { .. } => false,
            // functions have useful toString (shows source code)
            dir::Type::Function { .. } => true,
            // other types are assumed to not have useful toString
            _ => false,
        }
    }

    /// Check if a symbol is known to have useful toString.
    fn is_known_to_string_type(&self, symbol: dir::GlobalSymbolId) -> bool {
        // classes and built-in types typically have useful toString implementations
        // only plain objects (SymbolType::Void for anonymous objects) are problematic
        !matches!(symbol.ty(), dir::SymbolType::Void)
    }

    /// Check a method call for base toString usage.
    fn check_to_string(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // match method call pattern
        let Some(method_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };

        // check if this is toString()
        if method_call.method_name != self.to_string_name {
            return;
        }

        // get receiver type
        let Some(type_id) = self.ctx.expression_type_id(method_call.receiver_id) else {
            return;
        };

        // check if the receiver has useful toString
        if self.has_useful_to_string(type_id) {
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
                NO_BASE_TO_STRING.id,
                NO_BASE_TO_STRING.code,
                NO_BASE_TO_STRING.category,
                severity,
                "toString() may produce '[object Object]'",
                self.ctx.module.file_id,
                span,
            )
            .with_label("this type has no useful toString representation"),
        );
    }
}

impl NodeVisitor for BaseToStringVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_to_string(id);
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
            "test.ds",
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
            "test.ds",
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
            "test.ds",
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
            "test.ds",
            r#"
let arr = [1, 2, 3];
let str = arr.toString();
"#,
        );
        test.result(result).assert_no_lint("no-base-to-string");
    }
}
