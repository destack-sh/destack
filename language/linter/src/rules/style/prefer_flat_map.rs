use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    const_i64, expression_method_call, is_array_type, member_receiver_text,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest `.flatMap()` over `.map().flat()`.
    ///
    /// `flatMap` performs map and flat in one step, which is more concise
    /// and avoids creating an intermediate array.
    #[lint(
        id = "prefer-flat-map",
        code = "LY038",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
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
    /// The string id for the flatMap method name.
    flat_map_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferFlatMapVisitor<'a, 'b> {
    /// Build a visitor for prefer-flat-map checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let map_name = ctx.repository.strings.intern("map");
        let flat_name = ctx.repository.strings.intern("flat");
        let flat_map_name = ctx.repository.strings.intern("flatMap");

        Self {
            ctx,
            meta,
            array_symbol,
            map_name,
            flat_name,
            flat_map_name,
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
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // skip static call arguments until we support rendering them
        if !generic_arguments.is_empty() {
            return;
        }

        // match outer .flat() call
        let Some(flat_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };
        if flat_call.method_name != self.flat_name {
            return;
        }

        // check flat() member static arguments
        let flat_member = self.ctx.tree.get(flat_call.callee_id);
        let dir::Expression::Member {
            generic_arguments, ..
        } = flat_member
        else {
            return;
        };
        if !generic_arguments.is_empty() {
            return;
        }

        // check flat() arguments: must be no args or literal 1
        if !self.is_valid_flat_depth(arguments) {
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
        if !map_call.generic_arguments.is_empty() {
            return;
        }
        if map_call.arguments.is_empty() {
            return;
        }

        // keep React children mapping shape unchanged
        if self.is_ignored_map_receiver(map_call.receiver_id) {
            return;
        }

        // check map() member static arguments
        let map_member_id = map_call.callee_id;
        let map_member = self.ctx.tree.get(map_member_id);
        let dir::Expression::Member {
            generic_arguments, ..
        } = map_member
        else {
            return;
        };
        if !generic_arguments.is_empty() {
            return;
        }

        // verify the base is an array type
        if !self.is_array_receiver(map_call.receiver_id) {
            return;
        }

        // report the match and attach fix when safe
        self.report(expression_id, map_member_id, map_call.arguments);
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

    /// Return true when the receiver is one ignored React children helper.
    fn is_ignored_map_receiver(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression_span = self.ctx.get_span(expression_id);
        let expression_text = self.ctx.get_span_text(expression_span);
        let normalized_text: String = expression_text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();

        normalized_text == "Children" || normalized_text == "React.Children"
    }

    /// Report a prefer-flat-map match.
    fn report(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        map_member_id: dir::LocalNodeId<dir::Expression>,
        map_arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            PREFER_FLAT_MAP.id,
            PREFER_FLAT_MAP.code,
            PREFER_FLAT_MAP.category,
            severity,
            "prefer flatMap() over map().flat()",
            self.ctx.module.file_id,
            span,
        )
        .with_label("use array.flatMap(...) instead");
        if let Some(fix) = self.flat_map_fix(expression_id, map_member_id, map_arguments) {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix from map().flat() to flatMap().
    fn flat_map_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        map_member_id: dir::LocalNodeId<dir::Expression>,
        map_arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<LintFix> {
        // require at least one argument
        let first_argument_id = *map_arguments.first()?;
        let last_argument_id = *map_arguments.last()?;

        // derive receiver text from the map member expression
        let map_member_expression = self.ctx.tree.get(map_member_id);
        let dir::Expression::Member { left, name, .. } = map_member_expression else {
            return None;
        };
        let map_member_span = self.ctx.get_span(map_member_id);
        let map_member_text = self.ctx.get_span_text(map_member_span);
        let receiver_text =
            member_receiver_text(self.ctx, *left, map_member_text, (*name)?, false)?;

        // preserve original map argument source range
        let first_span = self.ctx.get_span(first_argument_id);
        let last_span = self.ctx.get_span(last_argument_id);
        let arguments_span = Span::new(first_span.file, first_span.start, last_span.end);
        let arguments_text = self.ctx.get_span_text(arguments_span);

        let flat_map_name = self.ctx.repository.strings.get(self.flat_map_name);
        let replacement = format!(
            "{receiver_text}.{}({arguments_text})",
            flat_map_name.as_ref()
        );

        // replace the full map().flat() expression
        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace map().flat() with flatMap()").with_edits(edits))
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
        if let dir::Expression::Call {
            generic_arguments,
            arguments,
            ..
        } = expression
        {
            self.check_call(id, generic_arguments.as_slice(), arguments);
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
            "prefer_flat_map/test_flags_map_flat.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.map(x => x).flat();
"#,
        );
        test.result(result).assert_lint("prefer-flat-map");
    }

    /// Fix map().flat() into flatMap().
    #[test]
    fn test_fix_map_flat() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "prefer_flat_map/test_fix_map_flat.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.map(x => x).flat();
"#,
        );
        test.result(result)
            .assert_lint("prefer-flat-map")
            .assert_has_fix("prefer-flat-map")
            .assert_safe_fixed(
                r#"
let items = [[1, 2], [3, 4]];
let flat = items.flatMap((x) => x);
"#,
            );
    }

    /// Flag map().flat(1) pattern.
    #[test]
    fn test_flags_map_flat_one() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "prefer_flat_map/test_flags_map_flat_one.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.map(x => x).flat(1);
"#,
        );
        test.result(result).assert_lint("prefer-flat-map");
    }

    /// Fix map().flat(1) into flatMap().
    #[test]
    fn test_fix_map_flat_one() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "prefer_flat_map/test_fix_map_flat_one.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.map(x => x).flat(1);
"#,
        );
        test.result(result)
            .assert_lint("prefer-flat-map")
            .assert_has_fix("prefer-flat-map")
            .assert_safe_fixed(
                r#"
let items = [[1, 2], [3, 4]];
let flat = items.flatMap((x) => x);
"#,
            );
    }

    /// Allow map().flat(2) since flatMap only flattens one level.
    #[test]
    fn test_allows_map_flat_two() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "prefer_flat_map/test_allows_map_flat_two.ds",
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
            "prefer_flat_map/test_allows_flat_alone.ds",
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
            "prefer_flat_map/test_allows_non_adjacent.ds",
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
            "prefer_flat_map/test_allows_map_flat_zero.ds",
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
            "prefer_flat_map/test_allows_flatmap_directly.ds",
            r#"
let items = [[1, 2], [3, 4]];
let flat = items.flatMap(x => x);
"#,
        );
        test.result(result).assert_no_lint("prefer-flat-map");
    }

    /// Allow Children.map(...).flat() helper usage.
    #[test]
    fn test_allows_children_map_flat() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "prefer_flat_map/test_allows_children_map_flat.ds",
            r#"
let flat = Children.map(children, fn).flat();
"#,
        );
        test.result(result).assert_no_lint("prefer-flat-map");
    }

    /// Allow React.Children.map(...).flat() helper usage.
    #[test]
    fn test_allows_react_children_map_flat() {
        let test = TestProgram::for_rule_with_prelude(PreferFlatMap);
        let result = test.lint_dir(
            "prefer_flat_map/test_allows_react_children_map_flat.ds",
            r#"
let flat = React.Children.map(children, fn).flat();
"#,
        );
        test.result(result).assert_no_lint("prefer-flat-map");
    }
}
