use crate::LintMeta;
use destack_workspace::{FilenameCase, LintSeverity};

use crate::{LintAstContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Enforce a specific case style for filenames.
    ///
    /// Consistent filename casing improves project organization.
    /// Configure via `filename_case` option.
    #[lint(
        id = "filename-case",
        code = "LY012",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub FilenameCaseRule,
    "Enforce filename case style"
}

impl LintRule for FilenameCaseRule {
    fn meta(&self) -> &'static LintMeta {
        FilenameCaseRule::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();
        let expected_case = ctx.options.style.filename_case;

        // keep filesystem paths only
        let Some(path) = &ctx.module.path else {
            return;
        };

        // keep one report anchor
        let root_expression_id = match ctx.roots.first() {
            Some(root_expression_id) => *root_expression_id,
            None => return,
        };
        let severity = ctx.get_effective_severity(meta, root_expression_id);
        if !severity.is_enabled() {
            return;
        }

        let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) else {
            return;
        };
        let base_name = file_stem.split('.').next().unwrap_or(file_stem);
        let normalized_name = base_name.trim_start_matches('_');
        if normalized_name.is_empty() {
            return;
        }

        // skip index files
        if normalized_name == "index" || normalized_name == "mod" {
            return;
        }

        // check if filename matches expected case
        if !matches_case(normalized_name, expected_case) {
            let expected = case_name(expected_case);
            ctx.report(
                LintReport::new(
                    FILENAME_CASE_RULE.id,
                    FILENAME_CASE_RULE.code,
                    FILENAME_CASE_RULE.category,
                    severity,
                    format!("filename `{base_name}` should be {expected}"),
                    ctx.tree.get_span(root_expression_id),
                )
                .label(format!("rename to {expected}")),
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
        let test = TestProgram::for_rule_without_prelude(FilenameCaseRule);
        let result = test.lint_ast("my-component.ds", "const x = 1");
        test.result(result).assert_no_lint("filename-case");
    }

    #[test]
    fn test_detects_snake_case_when_kebab_expected() {
        let test = TestProgram::for_rule_without_prelude(FilenameCaseRule);
        let result = test.lint_ast("my_component.ds", "const x = 1");
        test.result(result).assert_lint("filename-case");
    }

    #[test]
    fn test_detects_pascal_case_when_kebab_expected() {
        let test = TestProgram::for_rule_without_prelude(FilenameCaseRule);
        let result = test.lint_ast("MyComponent.ds", "const x = 1");
        test.result(result).assert_lint("filename-case");
    }

    #[test]
    fn test_allows_index_file() {
        let test = TestProgram::for_rule_without_prelude(FilenameCaseRule);
        let result = test.lint_ast("index.ds", "const x = 1");
        test.result(result).assert_no_lint("filename-case");
    }

    #[test]
    fn test_allows_mod_file() {
        let test = TestProgram::for_rule_without_prelude(FilenameCaseRule);
        let result = test.lint_ast("mod.ds", "const x = 1");
        test.result(result).assert_no_lint("filename-case");
    }

    #[test]
    fn test_allows_leading_underscore() {
        let test = TestProgram::for_rule_without_prelude(FilenameCaseRule);
        let result = test.lint_ast("_my-component.ds", "const x = 1");
        test.result(result).assert_no_lint("filename-case");
    }

    #[test]
    fn test_allows_multiple_extensions_when_basename_matches() {
        let test = TestProgram::for_rule_without_prelude(FilenameCaseRule);
        let result = test.lint_ast("my-component.test.ds", "const x = 1");
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
