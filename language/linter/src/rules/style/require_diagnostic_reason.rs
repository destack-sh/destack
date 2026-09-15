use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require reasons for diagnostic suppressions.
    pub REQUIRE_DIAGNOSTIC_REASON {
        id: "require-diagnostic-reason",
        summary: "Require reasons for diagnostic suppressions",
        explanation: r#"
A diagnostic suppression without a reason hides why the exception is necessary.
Instead, every `@allow` and `@expect` SHOULD include a concise reason.
"#,
        example: {
            reported: r#"
@allow("constant-condition")
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
            accepted: r#"
@allow("constant-condition", { reason: "required sentinel branch" })
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        },
        provenance: [
            Clippy("allow_attributes_without_reason"),
            TypeScriptEslint("ban-ts-comment"),
        ],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report canonical allow and expect decorators without a nonempty reason.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect diagnostic controls that suppress or expect a diagnostic
    for (_, application) in module.decorators.iter_applications() {
        if !matches!(
            application.resolution.target.language_item(),
            Some(dir::LanguageItem::Allow | dir::LanguageItem::Expect)
        ) {
            continue;
        }
        let reason = diagnostic_reason(module, application)?;
        if reason.is_some_and(|reason| !module.dir.strings.get(reason).trim().is_empty()) {
            continue;
        }

        // report the complete control so its options remain visible
        let span = module.source_extent(application.source.local_id.into_any())?;
        output.report(lint.diagnostic("diagnostic suppression has no reason", span));
    }

    Ok(output)
}

/// Return one diagnostic control's final reason.
fn diagnostic_reason(
    module: &DirModule<'_>,
    application: &dir::DecoratorApplication,
) -> Result<Option<dir::StringId>, ProviderError> {
    // read the evaluated decorator arguments
    let value = module.dir.get_static(application.value)?;
    let (_, value) = value.as_newtype().ok_or_else(|| {
        ProviderError::internal("canonical diagnostic control value is not a newtype")
    })?;
    let arguments = value.as_tuple().ok_or_else(|| {
        ProviderError::internal("canonical diagnostic control backing value is not a tuple")
    })?;

    // read the optional diagnostic control options
    let options = match arguments {
        [_] => return Ok(None),
        [_, options] => options,
        _ => {
            return Err(ProviderError::internal(
                "canonical diagnostic control has an invalid argument count",
            ));
        }
    };
    let properties = options.as_object().ok_or_else(|| {
        ProviderError::internal("canonical diagnostic control options are not an object")
    })?;

    // select the last reason field in evaluated property order
    for property in properties.iter().rev() {
        let Some((name, value)) = property.as_name_field() else {
            return Err(ProviderError::internal(
                "canonical diagnostic control contains a non-name field",
            ));
        };
        if module.dir.strings.get(name) != "reason" {
            continue;
        }
        let reason = value.as_string().ok_or_else(|| {
            ProviderError::internal("canonical diagnostic control reason is not a string")
        })?;

        return Ok(Some(reason));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an allow control without options.
    #[test]
    fn test_reports_allow_without_reason() {
        let session = TestSession::dir(
            &REQUIRE_DIAGNOSTIC_REASON,
            r#"
@allow("constant-condition")
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-diagnostic-reason]: diagnostic suppression has no reason
 ──▶ main.ds:1:1
  │
1 │ @allow("constant-condition")
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ function ready(): boolean {
3 │     if (true) {
  │
"#,
        );
    }

    /// Accept a nonempty suppression reason.
    #[test]
    fn test_accepts_reason() {
        let session = TestSession::dir(
            &REQUIRE_DIAGNOSTIC_REASON,
            r#"
@allow("constant-condition", { reason: "required sentinel branch" })
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

    /// Accept a reason supplied by the final static object spread.
    #[test]
    fn test_accepts_reason_from_spread() {
        let session = TestSession::dir(
            &REQUIRE_DIAGNOSTIC_REASON,
            r#"
@allow("constant-condition", {
    reason: "",
    ...{ reason: "generated declaration requires this suppression" },
})
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

    /// Report when the final static object spread clears an earlier reason.
    #[test]
    fn test_reports_reason_overridden_by_spread() {
        let session = TestSession::dir(
            &REQUIRE_DIAGNOSTIC_REASON,
            r#"
@allow("constant-condition", { reason: "generated declaration", ...{ reason: "" } })
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-diagnostic-reason]: diagnostic suppression has no reason
 ──▶ main.ds:1:1
  │
1 │ @allow("constant-condition", { reason: "generated declaration", ...{ reason: "" } })
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ function ready(): boolean {
3 │     if (true) {
  │
"#,
        );
    }

    /// Report conditional options that omit their reason.
    #[test]
    fn test_reports_conditional_control_without_reason() {
        let session = TestSession::dir(
            &REQUIRE_DIAGNOSTIC_REASON,
            r#"
@allow("constant-condition", { if: true, otherwise: "deny" })
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-diagnostic-reason]: diagnostic suppression has no reason
 ──▶ main.ds:1:1
  │
1 │ @allow("constant-condition", { if: true, otherwise: "deny" })
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ function ready(): boolean {
3 │     if (true) {
  │
"#,
        );
    }

    /// Report an empty expectation reason.
    #[test]
    fn test_reports_empty_reason() {
        let session = TestSession::dir(
            &REQUIRE_DIAGNOSTIC_REASON,
            r#"
@expect("constant-condition", { reason: "  " })
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-diagnostic-reason]: diagnostic suppression has no reason
 ──▶ main.ds:1:1
  │
1 │ @expect("constant-condition", { reason: "  " })
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ function ready(): boolean {
3 │     if (true) {
  │
"#,
        );
    }

    /// Accept diagnostic level controls that do not suppress a diagnostic.
    #[test]
    fn test_accepts_warning_without_reason() {
        let session = TestSession::dir(
            &REQUIRE_DIAGNOSTIC_REASON,
            r#"
@warn("constant-condition")
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[constant-condition]: condition is always true
 ──▶ main.ds:3:9
  │
1 │ @warn("constant-condition")
2 │ function ready(): boolean {
3 │     if (true) {
  │         ^^^^
4 │         return true;
5 │     }
  │
"#,
        );
    }

    /// Accept user-defined decorators named allow.
    #[test]
    fn test_accepts_user_decorator() {
        let session = TestSession::dir(
            &REQUIRE_DIAGNOSTIC_REASON,
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
