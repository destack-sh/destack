use destack_workspace::{FilenameCase, LintSeverity};

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce a specific case style for filenames.
    ///
    /// Consistent filename casing improves project organization.
    /// Configure via `filename_case` option (default: kebab-case).
    #[lint(
        id = "filename-case",
        code = "LY022",
        category = Style,
        level = Ast
    )]
    pub FilenameCaseRule,
    "Enforce filename case style"
}

impl LintRule for FilenameCaseRule {
    fn meta(&self) -> &'static crate::LintMeta {
        FilenameCaseRule::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let expected_case = ctx.options.filename_case;

        // get the filename without extension
        let Some(path) = &ctx.module.path else {
            return;
        };

        let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) else {
            return;
        };

        // skip index files
        if file_stem == "index" || file_stem == "mod" {
            return;
        }

        // check if filename matches expected case
        if !matches_case(file_stem, expected_case) {
            let expected = case_name(expected_case);
            ctx.report(
                LintDiagnostic::new(
                    FILENAME_CASE_RULE.id,
                    FILENAME_CASE_RULE.code,
                    FILENAME_CASE_RULE.category,
                    severity,
                    format!("filename `{file_stem}` should be {expected}"),
                    ctx.module.file_id,
                    ctx.tree.get_span(ctx.roots[0]),
                )
                .with_label(format!("rename to {expected}")),
            );
        }
    }
}

/// Check if a string matches the expected case style.
fn matches_case(s: &str, case: FilenameCase) -> bool {
    match case {
        FilenameCase::Kebab => is_kebab_case(s),
        FilenameCase::Snake => is_snake_case(s),
        FilenameCase::Camel => is_camel_case(s),
        FilenameCase::Pascal => is_pascal_case(s),
    }
}

/// Get the name of a case style.
fn case_name(case: FilenameCase) -> &'static str {
    match case {
        FilenameCase::Kebab => "kebab-case",
        FilenameCase::Snake => "snake_case",
        FilenameCase::Camel => "camelCase",
        FilenameCase::Pascal => "PascalCase",
    }
}

/// Check if string is kebab-case (lowercase with hyphens).
fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
}

/// Check if string is snake_case (lowercase with underscores).
fn is_snake_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !s.starts_with('_')
        && !s.ends_with('_')
        && !s.contains("__")
}

/// Check if string is camelCase (starts lowercase, no separators).
fn is_camel_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

/// Check if string is PascalCase (starts uppercase, no separators).
fn is_pascal_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_uppercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_kebab_case() {
        let test = TestProgram::for_rule(FilenameCaseRule);
        let result = test.lint_ast("my-component.ds", "const x = 1");
        test.result(result).assert_no_lint("filename-case");
    }

    #[test]
    fn test_detects_snake_case_when_kebab_expected() {
        let test = TestProgram::for_rule(FilenameCaseRule);
        let result = test.lint_ast("my_component.ds", "const x = 1");
        test.result(result).assert_lint("filename-case");
    }

    #[test]
    fn test_detects_pascal_case_when_kebab_expected() {
        let test = TestProgram::for_rule(FilenameCaseRule);
        let result = test.lint_ast("MyComponent.ds", "const x = 1");
        test.result(result).assert_lint("filename-case");
    }

    #[test]
    fn test_allows_index_file() {
        let test = TestProgram::for_rule(FilenameCaseRule);
        let result = test.lint_ast("index.ds", "const x = 1");
        test.result(result).assert_no_lint("filename-case");
    }

    #[test]
    fn test_allows_mod_file() {
        let test = TestProgram::for_rule(FilenameCaseRule);
        let result = test.lint_ast("mod.ds", "const x = 1");
        test.result(result).assert_no_lint("filename-case");
    }

    #[test]
    fn test_case_helpers() {
        // kebab-case
        assert!(is_kebab_case("foo-bar"));
        assert!(is_kebab_case("foo"));
        assert!(is_kebab_case("foo-bar-baz"));
        assert!(!is_kebab_case("fooBar"));
        assert!(!is_kebab_case("foo_bar"));
        assert!(!is_kebab_case("-foo"));
        assert!(!is_kebab_case("foo-"));
        assert!(!is_kebab_case("foo--bar"));

        // snake_case
        assert!(is_snake_case("foo_bar"));
        assert!(is_snake_case("foo"));
        assert!(!is_snake_case("foo-bar"));
        assert!(!is_snake_case("_foo"));

        // camelCase
        assert!(is_camel_case("fooBar"));
        assert!(is_camel_case("foo"));
        assert!(!is_camel_case("FooBar"));
        assert!(!is_camel_case("foo-bar"));

        // PascalCase
        assert!(is_pascal_case("FooBar"));
        assert!(is_pascal_case("Foo"));
        assert!(!is_pascal_case("fooBar"));
        assert!(!is_pascal_case("foo-bar"));
    }
}
