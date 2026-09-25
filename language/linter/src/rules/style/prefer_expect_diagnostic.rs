use tspp_dir as dir;
use tspp_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer diagnostic expectations for local suppressions.
    pub PREFER_EXPECT_DIAGNOSTIC {
        id: "prefer-expect-diagnostic",
        summary: "Prefer diagnostic expectations for local suppressions",
        explanation: r#"
A local `@allow` remains valid after the suppressed diagnostic disappears.
Instead, you SHOULD use `@expect` for an intentional diagnostic so its disappearance is reported.
"#,
        example: {
            reported: r#"
@allow("constant-condition", { reason: "required sentinel branch" })
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
            accepted: r#"
@expect("constant-condition", { reason: "required sentinel branch" })
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        },
        provenance: [TypeScriptEslint("prefer-ts-expect-error")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report canonical allow decorators whose diagnostics should be expected.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect applications of the canonical allow decorator
    for (_, application) in module.decorators.iter_applications() {
        if application.resolution.target.language_item() != Some(dir::LanguageItem::Allow) {
            continue;
        }

        // replace only the decorator function name
        let span = module.main_span(application.expression.local_id.into_any())?;
        let patch = Patch::replace(span, "expect");
        let suggestion = lint.suggestion("expect the diagnostic", patch)?;
        let diagnostic = lint
            .diagnostic("diagnostic is suppressed without an expectation", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;
    /// Retain conditional diagnostic control options.
    #[test]
    fn test_retains_conditional_options() {
        let session = TestSession::dir(
            &PREFER_EXPECT_DIAGNOSTIC,
            r#"
@allow("constant-condition", {
    reason: "required sentinel branch",
    if: true,
    otherwise: "deny",
})
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        );

        session.assert_suggestions(
            r#"
@expect("constant-condition", {
    reason: "required sentinel branch",
    if: true,
    otherwise: "deny",
})
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        );
    }

    /// Accept an existing diagnostic expectation.
    #[test]
    fn test_accepts_expect() {
        let session = TestSession::dir(
            &PREFER_EXPECT_DIAGNOSTIC,
            r#"
@expect("constant-condition", { reason: "required sentinel branch" })
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined decorator named allow.
    #[test]
    fn test_accepts_user_decorator() {
        let session = TestSession::dir(
            &PREFER_EXPECT_DIAGNOSTIC,
            r#"
newtype allow = (string,);

@allow("custom")
function ready(): boolean {
    return true;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
