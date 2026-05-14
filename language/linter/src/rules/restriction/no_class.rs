use crate::LintMeta;
use destack_dir::{self as dir, Declaration};
use destack_source::FileType;
use destack_workspace::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow class declarations.
    ///
    /// In Destack, structs are preferred over classes for data-oriented design.
    /// Classes encourage inheritance patterns that can lead to complex hierarchies.
    /// Otherwise, prefer interfaces, object types, or functions instead.
    #[lint(
        id = "no-class",
        code = "LR006",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoClass,
    "Disallow class declarations"
}

/// Return the primary diagnostic label for one file type.
fn no_class_label(file_type: FileType) -> &'static str {
    match file_type {
        FileType::Destack | FileType::DestackDeclaration => {
            "prefer structs or interface-based composition"
        }
        _ => "prefer interfaces, objects, or functions over classes",
    }
}

impl LintRule for NoClass {
    fn meta(&self) -> &'static LintMeta {
        NoClass::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Declaration>() {
            let declaration = ctx.dir.get(node_id);
            if !matches!(declaration, Declaration::Class(_)) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.dir.get_span(node_id);
            let file = ctx.file.as_ref();
            ctx.report(
                LintReport::new(
                    NO_CLASS.id,
                    NO_CLASS.code,
                    NO_CLASS.category,
                    severity,
                    "class declaration is not allowed in this codebase",
                    span,
                )
                .label(no_class_label(file.ty)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_class() {
        let test = TestProgram::for_rule_without_prelude(NoClass);
        let result = test.lint(
            "no_class/test_detects_class.ts",
            r#"
class MyClass {
    foo() {}
}
"#,
        );
        test.result(result).assert_lint("no-class");
    }

    #[test]
    fn test_detects_exported_class() {
        let test = TestProgram::for_rule_without_prelude(NoClass);
        let result = test.lint(
            "no_class/test_detects_exported_class.ts",
            r#"
export class MyClass {
    foo() {}
}
"#,
        );
        test.result(result).assert_lint("no-class");
    }

    #[test]
    fn test_detects_abstract_class() {
        let test = TestProgram::for_rule_without_prelude(NoClass);
        let result = test.lint(
            "no_class/test_detects_abstract_class.ts",
            r#"
abstract class BaseClass {
    abstract foo(): void;
}
"#,
        );
        test.result(result).assert_lint("no-class");
    }

    #[test]
    fn test_emits_non_destack_guidance() {
        let test = TestProgram::for_rule_without_prelude(NoClass);
        let result = test.lint(
            "no_class/test_emits_non_destack_guidance.ts",
            r#"
class MyClass {
    foo() {}
}
"#,
        );

        let result = test.result(result);
        let diagnostic = result
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.rule_id == "no-class")
            .unwrap_or_else(|| panic!("expected no-class diagnostic"));

        assert_eq!(
            diagnostic.message(),
            "class declaration is not allowed in this codebase"
        );
        assert_eq!(
            diagnostic.label_message(),
            Some("prefer interfaces, objects, or functions over classes")
        );
    }

    #[test]
    fn test_allows_struct() {
        let test = TestProgram::for_rule_without_prelude(NoClass);
        let result = test.lint(
            "no_class/test_allows_struct.ds",
            r#"
struct MyStruct {
    x: int32;
}
"#,
        );
        test.result(result).assert_no_lint("no-class");
    }

    #[test]
    fn test_allows_interface() {
        let test = TestProgram::for_rule_without_prelude(NoClass);
        let result = test.lint(
            "no_class/test_allows_interface.ts",
            r#"
interface MyInterface {
    foo(): void;
}
"#,
        );
        test.result(result).assert_no_lint("no-class");
    }

    #[test]
    fn test_allows_function() {
        let test = TestProgram::for_rule_without_prelude(NoClass);
        let result = test.lint(
            "no_class/test_allows_function.ts",
            r#"
function myFunction() {}
"#,
        );
        test.result(result).assert_no_lint("no-class");
    }

    #[test]
    fn test_skips_declaration_file_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoClass);
        let result = test.lint(
            "no_class/test_skips_declaration_file_by_default.d.ts",
            r#"
declare class ExternalClass {
    foo(): void;
}
"#,
        );
        test.result(result).assert_no_lint("no-class");
    }

    #[test]
    fn test_includes_declaration_file_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoClass).with_options(|options| {
            options.include_declaration_files = true;
        });
        let result = test.lint(
            "no_class/test_includes_declaration_file_when_enabled.d.ts",
            r#"
declare class ExternalClass {
    foo(): void;
}
"#,
        );
        test.result(result).assert_lint("no-class");
    }

    #[test]
    fn test_includes_destack_declaration_file_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoClass).with_options(|options| {
            options.include_declaration_files = true;
        });
        let result = test.lint(
            "no_class/test_includes_destack_declaration_file_when_enabled.d.ds",
            r#"
class ExternalClass {
    foo(): void;
}
"#,
        );

        let result = test.result(result);
        let diagnostic = result
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.rule_id == "no-class")
            .unwrap_or_else(|| panic!("expected no-class diagnostic"));

        assert_eq!(
            diagnostic.label_message(),
            Some("prefer structs or interface-based composition")
        );
    }
}
