use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::collect_module_symbol_usage;
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow private class members that are never used.
    ///
    /// Unused private members create maintenance overhead and can hide dead code.
    #[lint(
        id = "no-unused-private-class-members",
        code = "LC037",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoUnusedPrivateClassMembers,
    "Disallow unused private class members"
}

impl LintRule for NoUnusedPrivateClassMembers {
    fn meta(&self) -> &'static LintMeta {
        NoUnusedPrivateClassMembers::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let usage = collect_module_symbol_usage(ctx.module_id(), ctx.tree, ctx.types);

        // walk class declarations and report unused private members
        for declaration_id in ctx.tree.iter_node_ids_of_type::<dir::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            let dir::Declaration::Class { members, .. } = declaration else {
                continue;
            };

            // inspect each class member candidate
            for member_id in members {
                let member = ctx.tree.get(*member_id);

                // resolve member symbol candidates
                let Some(symbol_id) = candidate_member_symbol(member) else {
                    continue;
                };

                // skip non private members
                if !member_is_private(member) {
                    continue;
                }

                // skip referenced members
                if usage.references_local_symbol(ctx.module_id(), symbol_id) {
                    continue;
                }

                let severity = ctx.get_effective_severity(meta, *member_id);
                if !severity.is_enabled() {
                    continue;
                }

                // report one unused private member
                let span = ctx.get_span(*member_id);
                let mut diagnostic = LintDiagnostic::new(
                    NO_UNUSED_PRIVATE_CLASS_MEMBERS.id,
                    NO_UNUSED_PRIVATE_CLASS_MEMBERS.code,
                    NO_UNUSED_PRIVATE_CLASS_MEMBERS.category,
                    severity,
                    "unused private class member",
                    ctx.module.file_id,
                    span,
                )
                .with_label("this private class member is never used");

                // compute fixes only when requested by the runner
                if ctx.include_fixes
                    && let Some(fix) = unused_private_member_fix(ctx, *member_id, member)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build an unsafe fix for removable unused private members.
fn unused_private_member_fix(
    ctx: &LintModuleDirContext<'_>,
    member_id: dir::LocalNodeId<dir::Member>,
    member: &dir::Member,
) -> Option<LintFix> {
    // keep method members only to avoid dropping field initializer side effects
    let dir::Member::Method { signature, .. } = member else {
        return None;
    };

    // keep non constructor methods only
    if signature.mode == Some(dir::FunctionMode::Constructor) {
        return None;
    }

    let member_span = ctx.get_span(member_id);
    let edits = ctx.edit_builder().delete(member_span).into_edits();
    Some(LintFix::r#unsafe("Remove unused private method").with_edits(edits))
}

/// Return the symbol id for members this rule should inspect.
fn candidate_member_symbol(member: &dir::Member) -> Option<dir::LocalSymbolId> {
    match member {
        // inspect only value-space member forms
        dir::Member::Field { symbol, .. } => Some(*symbol),
        dir::Member::Method {
            signature, symbol, ..
        } => {
            // constructors are invoked by allocation and should not be linted here
            if signature.mode == Some(dir::FunctionMode::Constructor) {
                return None;
            }

            Some(*symbol)
        }
        _ => None,
    }
}

/// Return true when a class member is private by modifier or key kind.
fn member_is_private(member: &dir::Member) -> bool {
    match member {
        dir::Member::Field { modifiers, key, .. } | dir::Member::Method { modifiers, key, .. } => {
            let is_private_by_modifier = modifiers
                .as_ref()
                .and_then(|modifiers| modifiers.visibility)
                == Some(dir::Visibility::Private);
            let is_private_by_key = matches!(key, Some(dir::DynamicKey::Private(_)));
            is_private_by_modifier || is_private_by_key
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag an unused private field.
    #[test]
    fn test_flags_unused_private_field() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_flags_unused_private_field.ds",
            r#"
class Service {
    private token: int32 = 1;

    read(): int32 {
        return 1;
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-private-class-members");
    }

    /// Allow a private field used in a method body.
    #[test]
    fn test_allows_used_private_field() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_allows_used_private_field.ds",
            r#"
class Service {
    private token: int32 = 1;

    read(): int32 {
        return this.token;
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unused-private-class-members");
    }

    /// Flag an unused private method.
    #[test]
    fn test_flags_unused_private_method() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_flags_unused_private_method.ds",
            r#"
class Service {
    private helper(): int32 {
        return 1;
    }

    read(): int32 {
        return 2;
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-private-class-members");
    }

    /// Allow a private method used by another class method.
    #[test]
    fn test_allows_used_private_method() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_allows_used_private_method.ds",
            r#"
class Service {
    private helper(): int32 {
        return 1;
    }

    read(): int32 {
        return this.helper();
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unused-private-class-members");
    }

    /// Ignore non-private members.
    #[test]
    fn test_ignores_public_members() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_ignores_public_members.ds",
            r#"
class Service {
    token: int32 = 1;

    helper(): int32 {
        return 1;
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unused-private-class-members");
    }

    /// Flag a private dynamic-key field when unused.
    #[test]
    fn test_flags_unused_private_dynamic_key_field() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_flags_unused_private_dynamic_key_field.ds",
            r#"
class Service {
    #token: int32 = 1;

    read(): int32 {
        return 1;
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-private-class-members");
    }

    /// Allow a private dynamic-key field used in a method.
    #[test]
    fn test_allows_used_private_dynamic_key_field() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_allows_used_private_dynamic_key_field.ds",
            r#"
class Service {
    #token: int32 = 1;

    read(): int32 {
        return this.#token;
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unused-private-class-members");
    }

    /// Count multiple unused private members independently.
    #[test]
    fn test_reports_multiple_unused_private_members() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_reports_multiple_unused_private_members.ds",
            r#"
class Service {
    private first: int32 = 1;
    private second(): int32 {
        return 2;
    }

    read(): int32 {
        return 3;
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-private-class-members")
            .assert_lint_count("no-unused-private-class-members", 2);
    }

    /// Ignore constructors even when private.
    #[test]
    fn test_ignores_private_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_ignores_private_constructor.ds",
            r#"
class Service {
    private constructor() {}

    static create(): Service {
        return new Service();
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unused-private-class-members");
    }

    /// Unsafely remove one unused private method.
    #[test]
    fn test_fix_removes_unused_private_method() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_fix_removes_unused_private_method.ds",
            r#"
class Service {
    private helper(): int32 {
        return 1;
    }

    read(): int32 {
        return 2;
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-private-class-members")
            .assert_unsafe_fixed(
                r#"
class Service {

    read(): int32 {
        return 2;
    }
}
"#,
            );
    }

    /// Keep private field diagnostics without auto-fix.
    #[test]
    fn test_no_fix_for_unused_private_field() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedPrivateClassMembers);
        let result = test.lint_dir(
            "no_unused_private_class_members/test_no_fix_for_unused_private_field.ds",
            r#"
class Service {
    private token: int32 = 1;

    read(): int32 {
        return 2;
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-private-class-members")
            .assert_has_no_fix("no-unused-private-class-members");
    }
}
