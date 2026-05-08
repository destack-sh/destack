use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, SymbolForm, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_target_symbol, symbol_declaration_for, symbol_for};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer struct literal form over constructor calls.
    ///
    /// Structs are value types and read more clearly as tagged object literals.
    /// Prefer `Point { x: 1, y: 2 }` over `new Point(1, 2)`.
    #[lint(
        id = "prefer-struct-literal",
        code = "LY056",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferStructLiteral,
    "Prefer struct literal form"
}

impl LintRule for PreferStructLiteral {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferStructLiteral::meta()
    }

    /// Check module DIR nodes for struct constructor expressions.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferStructLiteralVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags struct `new` constructor usage.
struct PreferStructLiteralVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferStructLiteralVisitor<'a, 'b> {
    /// Build a visitor for struct literal style checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Return true when the constructor callee resolves to a struct symbol.
    fn is_struct_constructor(&self, callee_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let Some(target_symbol) = expression_target_symbol(self.ctx, callee_id) else {
            return false;
        };

        symbol_for(
            self.ctx.artifacts.as_ref(),
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            target_symbol,
        )
        .is_some_and(|symbol| symbol.form == SymbolForm::Struct)
    }

    /// Collect struct field names in constructor order for one constructor callee.
    fn constructor_field_names(
        &self,
        callee_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Vec<destack_core::StringId>> {
        // resolve the struct constructor symbol
        let target_symbol = expression_target_symbol(self.ctx, callee_id)?;
        let target_symbol_entry = symbol_for(
            self.ctx.artifacts.as_ref(),
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            target_symbol,
        )?;
        if target_symbol_entry.form != SymbolForm::Struct {
            return None;
        }

        // resolve the declaration for the struct symbol
        let declaration_id = symbol_declaration_for(
            self.ctx.artifacts.as_ref(),
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            target_symbol,
        )?;
        if declaration_id.local_id.ty != dir::NodeType::Declaration {
            return None;
        }

        // load the declaration module tree for cross module struct constructors
        let module_dir = self.ctx.declared_dir(declaration_id.module_id)?;
        let declaration = module_dir
            .tree
            .get(declaration_id.into_local_typed::<dir::Declaration>());
        let dir::Declaration::Struct(declaration) = declaration else {
            return None;
        };

        // collect named fields in declaration order
        let mut field_names = Vec::new();
        for member_id in &declaration.members {
            let member = module_dir.tree.get(*member_id);
            let dir::Member::Field { key, .. } = member else {
                continue;
            };
            let field_name = match key {
                dir::Key::Name(name) => name.string(),
                dir::Key::Private(_) | dir::Key::Expression(_) => return None,
            };

            field_names.push(field_name);
        }

        Some(field_names)
    }

    /// Build a safe fix from `new Struct(...)` to `Struct { ... }`.
    fn struct_literal_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        callee_id: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<LintFix> {
        // skip generic constructor calls until we support generic argument rendering
        if !generic_arguments.is_empty() {
            return None;
        }

        // require a field mapping in declaration order
        let field_names = self.constructor_field_names(callee_id)?;
        if field_names.len() != arguments.len() {
            return None;
        }

        // build field initializers from positional constructor arguments
        let mut field_initializers = Vec::new();
        for (field_name, argument_id) in field_names.into_iter().zip(arguments.iter()) {
            let argument = self.ctx.tree.get(*argument_id);
            let dir::Argument::Positional { value, .. } = argument else {
                return None;
            };

            let field_name_text = self.ctx.strings.get(field_name);
            let value_span = self.ctx.get_span(*value);
            let value_text = self.ctx.get_span_text(value_span);
            field_initializers.push(format!("{}: {value_text}", field_name_text.as_ref()));
        }

        // build the tagged struct literal replacement
        let callee_span = self.ctx.get_span(callee_id);
        let callee_text = self.ctx.get_span_text(callee_span);
        let replacement = if field_initializers.is_empty() {
            format!("{callee_text} {{}}")
        } else {
            format!("{callee_text} {{ {} }}", field_initializers.join(", "))
        };

        // replace the full constructor expression
        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();
        Some(LintFix::safe("Rewrite constructor to struct literal").with_edits(edits))
    }

    /// Check one `new` expression for struct constructor style.
    fn check_new_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        callee_id: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // only lint real struct constructors
        if !self.is_struct_constructor(callee_id) {
            return;
        }

        // honor configured severity at the node
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build the base diagnostic
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            PREFER_STRUCT_LITERAL.id,
            PREFER_STRUCT_LITERAL.code,
            PREFER_STRUCT_LITERAL.category,
            severity,
            "prefer struct literal form over struct constructor call",
            span,
        )
        .label("replace this constructor call with a tagged struct literal");

        // attach fix when argument to field mapping is unambiguous
        if let Some(fix) =
            self.struct_literal_fix(expression_id, callee_id, generic_arguments, arguments)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }
}

impl NodeVisitor for PreferStructLiteralVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check struct constructor calls
        if let dir::Expression::New {
            left,
            generic_arguments,
            arguments,
        } = expression
        {
            self.check_new_expression(id, *left, generic_arguments.as_slice(), arguments);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Report and fix local struct constructor calls.
    #[test]
    fn test_fix_local_struct_constructor_call() {
        let test = TestProgram::for_rule_without_prelude(PreferStructLiteral);
        let result = test.lint_dir(
            "prefer_struct_literal/test_fix_local_struct_constructor_call.ds",
            r#"
struct Point {
    x: int32
    y: int32
}

const point = new Point(1, 2);
"#,
        );
        test.result(result)
            .assert_lint("prefer-struct-literal")
            .assert_has_fix("prefer-struct-literal")
            .assert_safe_fixed(
                r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
"#,
            );
    }

    /// Report and fix cross module struct constructor calls.
    #[test]
    fn test_fix_cross_module_struct_constructor_call() {
        let test = TestProgram::for_rule_without_prelude(PreferStructLiteral);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "prefer_struct_literal/point.ds" => r#"
export struct Point {
    x: int32
    y: int32
}
"#,
                "prefer_struct_literal/consumer.ds" => r#"
import { Point } from "./point.ds";

const point = new Point(1, 2);
"#,
            },
            "prefer_struct_literal/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("prefer-struct-literal")
            .assert_has_fix("prefer-struct-literal")
            .assert_safe_fixed(
                r#"
import { Point } from "./point.ds";

const point = Point { x: 1, y: 2 };
"#,
            );
    }

    /// Do not report class constructor calls.
    #[test]
    fn test_allows_class_constructor_call() {
        let test = TestProgram::for_rule_without_prelude(PreferStructLiteral);
        let result = test.lint_dir(
            "prefer_struct_literal/test_allows_class_constructor_call.ds",
            r#"
class Point {
    x: int32;
    y: int32;
}

const point = new Point(1, 2);
"#,
        );
        test.result(result).assert_no_lint("prefer-struct-literal");
    }

    /// Report but avoid fixing generic constructor calls.
    #[test]
    fn test_no_fix_for_generic_struct_constructor_call() {
        let test = TestProgram::for_rule_without_prelude(PreferStructLiteral);
        let result = test.lint_dir(
            "prefer_struct_literal/test_no_fix_for_generic_struct_constructor_call.ds",
            r#"
struct Box<T> {
    value: T
}

const value = new Box<int32>(1);
"#,
        );
        test.result(result)
            .assert_lint("prefer-struct-literal")
            .assert_has_no_fix("prefer-struct-literal");
    }
}
