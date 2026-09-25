use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require lowercase kebab-case source filenames.
    pub FILENAME_CASE {
        id: "filename-case",
        summary: "Require lowercase kebab-case source filenames",
        explanation: r#"
Inconsistent filename casing makes module paths harder to predict across filesystems.
Instead, you SHOULD use lowercase ASCII kebab-case for every source filename component.

Dot-separated roles such as `.test` and `.d` are checked independently.
"#,
        example: {
            reported: ("UserService.tspp", "export const value = 1;"),
            accepted: ("user-service.tspp", "export const value = 1;"),
        },
        provenance: [Unicorn("filename-case")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report source files whose names do not use lowercase kebab-case.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect each source file contributing to this module
    for file in module.files {
        let Some(stem) = file.name.strip_suffix(".tspp") else {
            continue;
        };
        if stem.split('.').all(is_kebab_component) {
            continue;
        }

        // report the physical file as a whole
        let message = format!("filename `{}` must use lowercase kebab-case", file.name);
        output.report(lint.diagnostic(message, file.id));
    }

    Ok(output)
}

/// Return whether one dot-separated filename component uses lowercase kebab-case.
fn is_kebab_component(component: &str) -> bool {
    let mut characters = component.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }

    // require lowercase words separated by single interior hyphens
    let mut previous_was_hyphen = false;
    for character in characters {
        let is_hyphen = character == '-';
        let is_valid = character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || is_hyphen && !previous_was_hyphen;
        if !is_valid {
            return false;
        }
        previous_was_hyphen = is_hyphen;
    }

    !previous_was_hyphen
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report uppercase and underscore filename words.
    #[test]
    fn test_reports_noncanonical_filename() {
        let session = TestSession::dir_path(
            &FILENAME_CASE,
            "User_service.tspp",
            "export const value = 1;",
        );

        session.assert_diagnostics(
            r#"
warning[filename-case]: filename `User_service.tspp` must use lowercase kebab-case
  ──▶ User_service.tspp
"#,
        );
    }

    /// Accept declaration roles written as separate kebab-case components.
    #[test]
    fn test_accepts_declaration_filename() {
        let session = TestSession::dir_path(
            &FILENAME_CASE,
            "user-service.d.tspp",
            "export declare const value: int32;",
        );

        session.assert_no_diagnostics();
    }
}
