use destack_ast::{self as ast};
use destack_dir as dir;
use destack_source::{NodeSpanRegion, NodeSpanType};
use destack_workspace::LintSeverity;

use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty interface declarations.
    ///
    /// Empty interfaces are often a sign of incomplete code. If the interface
    /// has no members and doesn't extend anything, use `type X = {}` or
    /// `type X = object` instead. If it extends a single type, use a type alias.
    #[lint(
        id = "no-empty-interface",
        code = "LY018",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoEmptyInterface,
    "Disallow empty interface declarations"
}

impl LintRule for NoEmptyInterface {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoEmptyInterface::meta()
    }

    /// Check module DIR declarations for empty interfaces.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // inspect interface declarations only
        for declaration_id in ctx.tree.iter_node_ids_of_type::<dir::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            let dir::Declaration::Interface(declaration) = declaration else {
                continue;
            };
            let Some(interface_name_id) = declaration.name.map(|name| name.string()) else {
                continue;
            };

            // keep non-empty interfaces out of this rule
            if !declaration.members.is_empty() {
                continue;
            }

            // resolve source declaration data used for fixes
            let Some(source_declaration_id) =
                ctx.source_node_id::<ast::Declaration>(declaration_id.into_any())
            else {
                continue;
            };
            let source_declaration = ctx.ast.get(source_declaration_id);
            let ast::Declaration::Interface(source_declaration) = source_declaration else {
                continue;
            };

            // honor per node severity
            let severity = ctx.get_effective_severity(meta, declaration_id);
            if !severity.is_enabled() {
                continue;
            }

            // report plain empty interfaces first
            let extends_count = declaration.extends.len();
            let span = ctx.get_span(declaration_id);
            if extends_count == 0 {
                ctx.report(
                    LintReport::new(
                        NO_EMPTY_INTERFACE.id,
                        NO_EMPTY_INTERFACE.code,
                        NO_EMPTY_INTERFACE.category,
                        severity,
                        "empty interface declaration",
                        span,
                    )
                    .label("use `type X = {}` or add members"),
                );
                continue;
            }

            // keep multi-extends and configured single-extends interfaces
            if extends_count != 1 || ctx.options.style.allow_single_extends_empty_interface {
                continue;
            }

            // report single-extends interfaces and attach one safe fix when merging allows it
            let mut diagnostic = LintReport::new(
                NO_EMPTY_INTERFACE.id,
                NO_EMPTY_INTERFACE.code,
                NO_EMPTY_INTERFACE.category,
                severity,
                "interface extends single type without adding members",
                span,
            )
            .label("use `type X = Parent` or `newtype X = Parent` instead");
            if ctx.include_fixes
                && let Some(fix) = no_empty_interface_single_extends_fix(
                    ctx,
                    source_declaration_id,
                    interface_name_id,
                    source_declaration,
                )
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a type alias fix for one empty single-extends interface.
fn no_empty_interface_single_extends_fix(
    ctx: &LintModuleDirContext<'_>,
    source_declaration_id: ast::LocalNodeId<ast::Declaration>,
    interface_name_id: ast::StringId,
    declaration: &ast::InterfaceDeclaration,
) -> Option<LintFix> {
    // keep interfaces with where clauses out of the automatic rewrite
    if !declaration.where_clauses.is_empty() {
        return None;
    }

    // build the replacement alias from the source declaration text
    let parent = declaration.extends.first()?;
    let parent_span = ctx
        .ast
        .get_side_span(
            parent.expression,
            NodeSpanType::Region(NodeSpanRegion::Type),
        )
        .unwrap_or_else(|| ctx.ast.get_span(parent.expression));
    let parent_text = ctx.get_span_text(parent_span);
    let interface_name = ctx.strings.get(interface_name_id);
    let generic_text = generic_parameters_text(ctx, &declaration.generic_parameters);
    let replacement = format!("type {}{generic_text} = {parent_text}", interface_name);
    let edits = ctx
        .edit_builder()
        .replace(ctx.ast.get_span(source_declaration_id), replacement)
        .into_edits();

    Some(LintFix::safe("Convert to type alias").with_edits(edits))
}

/// Build source text for generic parameter declarations.
fn generic_parameters_text(
    ctx: &LintModuleDirContext<'_>,
    parameters: &[ast::LocalNodeId<ast::GenericParameter>],
) -> String {
    if parameters.is_empty() {
        return String::new();
    }

    // keep the exact source text for each generic parameter
    let parameter_text = parameters
        .iter()
        .map(|parameter_id| {
            ctx.get_span_text(ctx.ast.get_span(*parameter_id))
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("<{parameter_text}>")
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_interface() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_dir(
            "no_empty_interface/test_detects_empty_interface.ts",
            r#"
interface Empty {}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-empty-interface");
    }

    #[test]
    fn test_detects_single_extends() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_dir(
            "no_empty_interface/test_detects_single_extends.ts",
            r#"
interface Parent {
    value: number;
}

interface Child extends Parent {}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-empty-interface");
    }

    #[test]
    fn test_allows_interface_with_members() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_dir(
            "no_empty_interface/test_allows_interface_with_members.ts",
            r#"
interface Foo {
    bar(): void;
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-empty-interface");
    }

    #[test]
    fn test_allows_multiple_extends() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_dir(
            "no_empty_interface/test_allows_multiple_extends.ts",
            r#"
interface A {
    a: number;
}

interface B {
    b: number;
}

interface Combined extends A, B {}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-empty-interface");
    }

    #[test]
    fn test_allows_extends_with_members() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_dir(
            "no_empty_interface/test_allows_extends_with_members.ts",
            r#"
interface Parent {
    value: number;
}

interface Child extends Parent {
    extra(): void;
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-empty-interface");
    }

    #[test]
    fn test_no_fix_for_empty_interface() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_dir(
            "no_empty_interface/test_no_fix_for_empty_interface.ts",
            r#"
interface Empty {}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-empty-interface")
            .assert_has_no_fix("no-empty-interface");
    }

    #[test]
    fn test_fix_single_extends() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_dir(
            "no_empty_interface/test_fix_single_extends.ts",
            r#"
interface Parent {
    value: number;
}

interface Child extends Parent {}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-empty-interface")
            .assert_safe_fixed(
                r#"
interface Parent {
    value: number;
}

type Child = Parent;
"#,
            );
    }

    #[test]
    fn test_allows_single_extends_when_option_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface)
            .with_options(|options| options.style.allow_single_extends_empty_interface = true);
        let result = test.lint_dir(
            "no_empty_interface/test_allows_single_extends_when_option_enabled.ts",
            r#"
interface Parent {
    value: number;
}

interface Child extends Parent {}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-empty-interface");
    }

    #[test]
    fn test_no_fix_for_single_extends_with_merged_class() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_dir(
            "no_empty_interface/test_no_fix_for_single_extends_with_merged_class.ts",
            r#"
interface Parent {
    value: number;
}

interface Child extends Parent {}
class Child {}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-empty-interface")
            .assert_has_no_fix("no-empty-interface");
    }

    #[test]
    fn test_fix_single_extends_with_generics() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_dir(
            "no_empty_interface/test_fix_single_extends_with_generics.ts",
            r#"
interface Parent<T> {
    value: T;
}

interface Box<T> extends Parent<T> {}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-empty-interface")
            .assert_safe_fixed(
                r#"
interface Parent<T> {
    value: T;
}

type Box<T> = Parent<T>;
"#,
            );
    }
}
