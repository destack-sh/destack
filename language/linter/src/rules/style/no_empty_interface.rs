use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

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
        level = Ast,
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
    fn meta(&self) -> &'static crate::LintMeta {
        NoEmptyInterface::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let Declaration::Interface {
                descriptor,
                generics,
                members,
                heritage,
                ..
            } = declaration
            else {
                continue;
            };

            let Some(interface_name_id) = descriptor.name.map(|name| name.string()) else {
                continue;
            };

            let extends_count = heritage
                .extends_types
                .as_ref()
                .map(|v| v.len())
                .unwrap_or(0);

            if !members.is_empty() {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            if extends_count == 0 {
                ctx.report(
                    LintDiagnostic::new(
                        NO_EMPTY_INTERFACE.id,
                        NO_EMPTY_INTERFACE.code,
                        NO_EMPTY_INTERFACE.category,
                        severity,
                        "empty interface declaration",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("use `type X = {}` or add members"),
                );
                continue;
            }

            if extends_count != 1 || ctx.options.allow_single_extends_empty_interface {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                NO_EMPTY_INTERFACE.id,
                NO_EMPTY_INTERFACE.code,
                NO_EMPTY_INTERFACE.category,
                severity,
                "interface extends single type without adding members",
                ctx.module.file_id,
                span,
            )
            .with_label("use `type X = Parent` or `newtype X = Parent` instead");

            if ctx.compute_fixes
                && !interface_has_class_merge(ctx, node_id, interface_name_id)
                && let Some(fix) = no_empty_interface_single_extends_fix(
                    ctx,
                    node_id,
                    interface_name_id,
                    generics,
                    heritage,
                )
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when the interface name is merged with a class declaration.
fn interface_has_class_merge(
    ctx: &LintModuleAstContext<'_>,
    interface_id: ast::LocalNodeId<ast::Declaration>,
    interface_name_id: ast::StringId,
) -> bool {
    for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
        if declaration_id == interface_id {
            continue;
        }

        let declaration = ctx.tree.get(declaration_id);
        let Declaration::Class { descriptor, .. } = declaration else {
            continue;
        };

        let Some(class_name_id) = descriptor.name.map(|name| name.string()) else {
            continue;
        };

        if class_name_id == interface_name_id {
            return true;
        }
    }

    false
}

/// Build a type alias fix for one empty single-extends interface.
fn no_empty_interface_single_extends_fix(
    ctx: &LintModuleAstContext<'_>,
    interface_id: ast::LocalNodeId<ast::Declaration>,
    interface_name_id: ast::StringId,
    generics: &ast::Generics,
    heritage: &ast::Heritage,
) -> Option<LintFix> {
    if generics.where_clauses.is_some() {
        return None;
    }

    let parent_id = *heritage.extends_types.as_ref()?.first()?;
    let parent_text = ctx.get_span_text(ctx.tree.get_span(parent_id));
    let interface_name = ctx.strings.get(interface_name_id);
    let generic_text = generic_parameters_text(ctx, generics.static_parameters.as_deref());

    let replacement = format!(
        "type {}{generic_text} = {parent_text}",
        interface_name.as_ref()
    );
    let edits = ctx
        .edit_builder()
        .replace(ctx.tree.get_span(interface_id), replacement)
        .into_edits();
    Some(LintFix::safe("Convert to type alias").with_edits(edits))
}

/// Build source text for generic parameter declarations.
fn generic_parameters_text(
    ctx: &LintModuleAstContext<'_>,
    parameters: Option<&[ast::LocalNodeId<ast::Parameter>]>,
) -> String {
    let Some(parameters) = parameters else {
        return String::new();
    };
    if parameters.is_empty() {
        return String::new();
    }

    let parameter_text = parameters
        .iter()
        .map(|parameter_id| {
            ctx.get_span_text(ctx.tree.get_span(*parameter_id))
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
        let result = test.lint_ast(
            "no_empty_interface/test_detects_empty_interface.ts",
            r#"
interface Empty {}
"#,
        );
        test.result(result).assert_lint("no-empty-interface");
    }

    #[test]
    fn test_detects_single_extends() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_ast(
            "no_empty_interface/test_detects_single_extends.ts",
            r#"
interface Child extends Parent {}
"#,
        );
        test.result(result).assert_lint("no-empty-interface");
    }

    #[test]
    fn test_allows_interface_with_members() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_ast(
            "no_empty_interface/test_allows_interface_with_members.ts",
            r#"
interface Foo {
    bar(): void;
}
"#,
        );
        test.result(result).assert_no_lint("no-empty-interface");
    }

    #[test]
    fn test_allows_multiple_extends() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_ast(
            "no_empty_interface/test_allows_multiple_extends.ts",
            r#"
interface Combined extends A, B {}
"#,
        );
        test.result(result).assert_no_lint("no-empty-interface");
    }

    #[test]
    fn test_allows_extends_with_members() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_ast(
            "no_empty_interface/test_allows_extends_with_members.ts",
            r#"
interface Child extends Parent {
    extra(): void;
}
"#,
        );
        test.result(result).assert_no_lint("no-empty-interface");
    }

    #[test]
    fn test_no_fix_for_empty_interface() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_ast(
            "no_empty_interface/test_no_fix_for_empty_interface.ts",
            r#"
interface Empty {}
"#,
        );
        test.result(result)
            .assert_lint("no-empty-interface")
            .assert_has_no_fix("no-empty-interface");
    }

    #[test]
    fn test_fix_single_extends() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_ast(
            "no_empty_interface/test_fix_single_extends.ts",
            r#"
interface Child extends Parent {}
"#,
        );
        test.result(result)
            .assert_lint("no-empty-interface")
            .assert_safe_fixed(
                r#"
type Child = Parent;
"#,
            );
    }

    #[test]
    fn test_allows_single_extends_when_option_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface)
            .with_options(|options| options.allow_single_extends_empty_interface = true);
        let result = test.lint_ast(
            "no_empty_interface/test_allows_single_extends_when_option_enabled.ts",
            r#"
interface Child extends Parent {}
"#,
        );
        test.result(result).assert_no_lint("no-empty-interface");
    }

    #[test]
    fn test_no_fix_for_single_extends_with_merged_class() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_ast(
            "no_empty_interface/test_no_fix_for_single_extends_with_merged_class.ts",
            r#"
interface Child extends Parent {}
class Child {}
"#,
        );
        test.result(result)
            .assert_lint("no-empty-interface")
            .assert_has_no_fix("no-empty-interface");
    }

    #[test]
    fn test_fix_single_extends_with_generics() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyInterface);
        let result = test.lint_ast(
            "no_empty_interface/test_fix_single_extends_with_generics.ts",
            r#"
interface Box<T> extends ReadonlyBox<T> {}
"#,
        );
        test.result(result)
            .assert_lint("no-empty-interface")
            .assert_safe_fixed(
                r#"
type Box<T> = ReadonlyBox<T>;
"#,
            );
    }
}
