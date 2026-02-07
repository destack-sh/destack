use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{const_i64, expression_method_call, is_array_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest `.flatMap()` over `.map().flat()`.
    ///
    /// `flatMap` performs map and flat in one step, which is more concise
    /// and avoids creating an intermediate array.
    #[lint(
        id = "prefer-flat-map",
        code = "LY086",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferFlatMap,
    "Prefer flatMap() over map().flat()"
}

impl LintRule for PreferFlatMap {
    fn meta(&self) -> &'static LintMeta {
        PreferFlatMap::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferFlatMapVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags map().flat() patterns.
struct PreferFlatMapVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The array symbol for this module profile.
    array_symbol: dir::GlobalSymbolId,
    /// The string id for the map method name.
    map_name: StringId,
    /// The string id for the flat method name.
    flat_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferFlatMapVisitor<'a, 'b> {
    /// Build a visitor for prefer-flat-map checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let map_name = ctx.program.strings.intern("map");
        let flat_name = ctx.program.strings.intern("flat");

        Self {
            ctx,
            meta,
            array_symbol,
            map_name,
            flat_name,
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

    /// Check if this is a map().flat() pattern.
    fn check_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // match outer .flat() call
        let Some(flat_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };
        if flat_call.method_name != self.flat_name {
            return;
        }

        // check flat() arguments: must be no args or literal 1
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            dynamic_arguments, ..
        } = expression
        else {
            return;
        };
        if !self.is_valid_flat_depth(dynamic_arguments.as_slice()) {
            return;
        }

        // match inner .map() call as receiver of flat
        let Some(map_call) = expression_method_call(self.ctx.tree, flat_call.receiver_id) else {
            return;
        };
        if map_call.method_name != self.map_name {
            return;
        }

        // check that map() has at least one argument
        let map_expression = self.ctx.tree.get(flat_call.receiver_id);
        let dir::Expression::Call {
            dynamic_arguments: map_args,
            ..
        } = map_expression
        else {
            return;
        };
        if map_args.is_empty() {
            return;
        }

        // verify the base is an array type
        if !self.is_array_receiver(map_call.receiver_id) {
            return;
        }

        // report the match
        self.report(expression_id);
    }

    /// Check if flat() depth argument is valid (none or literal 1).
    fn is_valid_flat_depth(&mut self, arguments: &[dir::LocalNodeId<dir::Argument>]) -> bool {
        // no arguments is valid
        if arguments.is_empty() {
            return true;
        }

        // more than one argument is invalid
        if arguments.len() > 1 {
            return false;
        }

        // get the expression from the argument
        let argument = self.ctx.tree.get(arguments[0]);
        let expression_id = argument.value();

        // single argument must be literal 1
        let Some(const_value) = self.ctx.const_value(expression_id) else {
            return false;
        };
        let Some(depth) = const_i64(&const_value) else {
            return false;
        };

        depth == 1
    }

    /// Return true when the receiver expression is an array type.
    fn is_array_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_array_type(self.ctx.types, type_id, Some(self.array_symbol))
    }

    /// Report a prefer-flat-map match.
    fn report(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                PREFER_FLAT_MAP.id,
                PREFER_FLAT_MAP.code,
                PREFER_FLAT_MAP.category,
                severity,
                "prefer flatMap() over map().flat()",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use array.flatMap(...) instead"),
        );
    }
}

impl NodeVisitor for PreferFlatMapVisitor<'_, '_> {
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
            self.check_call(id);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag map().flat() pattern.
    #[test]
    fn test_flags_map_flat() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.map(x => x).flat();
"#,
        );
        test.result(result).assert_lint("prefer-flat-map");
    }

    /// Flag map().flat(1) pattern.
    #[test]
    fn test_flags_map_flat_one() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.map(x => x).flat(1);
"#,
        );
        test.result(result).assert_lint("prefer-flat-map");
    }

    /// Allow map().flat(2) since flatMap only flattens one level.
    #[test]
    fn test_allows_map_flat_two() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [[[1, 2]], [[3, 4]]];
let flat = items.map(x => x).flat(2);
"#,
        );
        test.result(result).assert_no_lint("prefer-flat-map");
    }

    /// Allow flat() without preceding map().
    #[test]
    fn test_allows_flat_alone() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.flat();
"#,
        );
        test.result(result).assert_no_lint("prefer-flat-map");
    }

    /// Allow non-adjacent map and flat in chain.
    #[test]
    fn test_allows_non_adjacent() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.map(x => x).slice().flat();
"#,
        );
        test.result(result).assert_no_lint("prefer-flat-map");
    }

    /// Allow map().flat(0) since it's a no-op flatten.
    #[test]
    fn test_allows_map_flat_zero() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.map(x => x).flat(0);
"#,
        );
        test.result(result).assert_no_lint("prefer-flat-map");
    }

    /// Allow flatMap() directly.
    #[test]
    fn test_allows_flatmap_directly() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.flatMap(x => x);
"#,
        );
        test.result(result).assert_no_lint("prefer-flat-map");
    }
}
