use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_attribute_map;
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow usage of APIs marked as `@deprecated`.
    ///
    /// Deprecated symbols remain available but should be migrated to
    /// supported alternatives.
    #[lint(
        id = "no-deprecated",
        code = "LC011",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoDeprecated,
    "Disallow use of @deprecated APIs"
}

impl LintRule for NoDeprecated {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoDeprecated::meta()
    }

    /// Check module DIR nodes for deprecated API usage.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = DeprecatedUsageVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor for deprecated usage checks.
struct DeprecatedUsageVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> DeprecatedUsageVisitor<'a, 'b> {
    /// Build a visitor for deprecated usage checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one expression for deprecated symbol usage.
    fn check_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if !is_usage_expression(expression) {
            return;
        }
        if should_skip_expression(self.ctx.tree, expression_id) {
            return;
        }

        // require optional structure
        let Some(deprecated_message) = self.deprecated_message_for_expression(expression_id) else {
            return;
        };

        // resolve effective lint severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // resolve diagnostic span
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            NO_DEPRECATED.id,
            NO_DEPRECATED.code,
            NO_DEPRECATED.category,
            severity,
            "deprecated API usage",
            span,
        );

        // enforce this lint guard
        if let Some(message) = deprecated_message {
            diagnostic = diagnostic.label(format!("deprecated: {message}"));
        } else {
            diagnostic = diagnostic.label("this API is marked as deprecated");
        }

        self.ctx.report(diagnostic);
    }

    /// Resolve one deprecated message for an expression when available.
    fn deprecated_message_for_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Option<String>> {
        expression_attribute_map(
            self.ctx.artifacts.as_ref(),
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            self.ctx.types,
            expression_id,
            |attributes| attributes.deprecated_message(),
        )
        .map(|message_id| message_id.map(|message_id| self.ctx.strings.get(message_id).to_string()))
    }
}

impl NodeVisitor for DeprecatedUsageVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        self.check_expression(id, expression);
        walk_expression(self, tree, id, expression);
    }
}

/// Return true when an expression represents a user-visible usage site.
fn is_usage_expression(expression: &dir::Expression) -> bool {
    matches!(
        expression,
        dir::Expression::Path { .. }
            | dir::Expression::Member { .. }
            | dir::Expression::Call { .. }
            | dir::Expression::New { .. }
    )
}

/// Return true when this expression should be skipped to avoid duplicate reports.
fn should_skip_expression(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(parent) = tree.get_parent(expression_id.id) else {
        return false;
    };

    // ignore annotation references
    if parent.ty == dir::NodeType::Decorator {
        return true;
    }

    // keep only call/new as the primary report node for callees
    if parent.ty == dir::NodeType::Expression {
        let parent_id = parent.into_typed::<dir::Expression>();
        let parent_expression = tree.get(parent_id);
        match parent_expression {
            dir::Expression::Call { left, .. } | dir::Expression::New { left, .. } => {
                return *left == expression_id;
            }
            _ => {}
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LintLevel;
    use crate::linter::TestProgram;

    /// Flag calls to deprecated functions.
    #[test]
    fn test_flags_deprecated_function_call() {
        let test = TestProgram::for_rule_with_prelude(NoDeprecated);
        let result = test.lint_dir(
            "no_deprecated/test_flags_deprecated_function_call.ds",
            r#"
@deprecated("use next")
function legacy(): int32 {
    return 1
}

legacy()
"#,
        );
        test.result(result).assert_lint("no-deprecated");
    }

    /// Flag references to deprecated methods.
    #[test]
    fn test_flags_deprecated_method_reference() {
        let test = TestProgram::for_rule_with_prelude(NoDeprecated);
        let result = test.lint_dir(
            "no_deprecated/test_flags_deprecated_method_reference.ds",
            r#"
class Service {
    @deprecated("use run")
    execute(): int32 {
        return 1
    }
}

let service = new Service();
let callback = service.execute;
"#,
        );
        test.result(result).assert_lint("no-deprecated");
    }

    /// Allow non deprecated API usage.
    #[test]
    fn test_allows_non_deprecated_usage() {
        let test = TestProgram::for_rule_with_prelude(NoDeprecated);
        let result = test.lint_dir(
            "no_deprecated/test_allows_non_deprecated_usage.ds",
            r#"
function current(): int32 {
    return 1;
}

current();
"#,
        );
        test.result(result).assert_no_lint("no-deprecated");
    }

    /// Flag deprecated imports across modules.
    #[test]
    fn test_flags_deprecated_import_usage() {
        let test = TestProgram::for_rule_with_prelude(NoDeprecated);
        let old_module = test.add_module(
            "no_deprecated/legacy.ds",
            r#"
@deprecated("use stable()")
export function legacy(): int32 {
    return 1;
}
"#,
        );
        let user_module = test.add_module(
            "no_deprecated/user.ds",
            r#"
import { legacy } from "./legacy.ds"

legacy();
"#,
        );

        test.import_module(old_module);
        test.import_module(user_module);
        test.enqueue_profile_resolution_once();
        test.analyze_module(old_module);
        test.analyze_module(user_module);
        test.compile();

        let result = test.lint_module(user_module, LintLevel::Dir);
        test.result(result).assert_lint("no-deprecated");
    }

    /// Flag aliased imports of deprecated symbols.
    #[test]
    fn test_flags_aliased_deprecated_import_usage() {
        let test = TestProgram::for_rule_with_prelude(NoDeprecated);
        let legacy_module = test.add_module(
            "no_deprecated/aliased_legacy.ds",
            r#"
@deprecated("use stable()")
export function legacy(): int32 {
    return 1;
}
"#,
        );
        let user_module = test.add_module(
            "no_deprecated/aliased_user.ds",
            r#"
import { legacy as renamedLegacy } from "./aliased_legacy.ds"

renamedLegacy();
"#,
        );

        test.import_module(legacy_module);
        test.import_module(user_module);
        test.enqueue_profile_resolution_once();
        test.analyze_module(legacy_module);
        test.analyze_module(user_module);
        test.compile();

        let result = test.lint_module(user_module, LintLevel::Dir);
        test.result(result).assert_lint("no-deprecated");
    }

    /// Flag namespace imports that reference deprecated symbols.
    #[test]
    fn test_flags_namespace_deprecated_import_usage() {
        let test = TestProgram::for_rule_with_prelude(NoDeprecated);
        let legacy_module = test.add_module(
            "no_deprecated/namespace_legacy.ds",
            r#"
@deprecated("use stable()")
export function legacy(): int32 {
    return 1;
}
"#,
        );
        let user_module = test.add_module(
            "no_deprecated/namespace_user.ds",
            r#"
import * as legacyModule from "./namespace_legacy.ds"

legacyModule.legacy();
"#,
        );

        test.import_module(legacy_module);
        test.import_module(user_module);
        test.enqueue_profile_resolution_once();
        test.analyze_module(legacy_module);
        test.analyze_module(user_module);
        test.compile();

        let result = test.lint_module(user_module, LintLevel::Dir);
        test.result(result).assert_lint("no-deprecated");
    }

    /// Flag deprecated symbols that are re-exported through another module.
    #[test]
    fn test_flags_reexported_deprecated_import_usage() {
        let test = TestProgram::for_rule_with_prelude(NoDeprecated);
        let legacy_module = test.add_module(
            "no_deprecated/reexported_legacy.ds",
            r#"
@deprecated("use stable()")
export function legacy(): int32 {
    return 1;
}
"#,
        );
        let bridge_module = test.add_module(
            "no_deprecated/reexport_bridge.ds",
            r#"
export { legacy } from "./reexported_legacy.ds"
"#,
        );
        let user_module = test.add_module(
            "no_deprecated/reexport_user.ds",
            r#"
import { legacy } from "./reexport_bridge.ds"

legacy();
"#,
        );

        test.import_module(legacy_module);
        test.import_module(bridge_module);
        test.import_module(user_module);
        test.enqueue_profile_resolution_once();
        test.analyze_module(legacy_module);
        test.analyze_module(bridge_module);
        test.analyze_module(user_module);
        test.compile();

        let result = test.lint_module(user_module, LintLevel::Dir);
        test.result(result).assert_lint("no-deprecated");
    }
}
