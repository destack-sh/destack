use crate::tests::{DirRows, TestSession};

/// Suppress one matching compiler warning.
#[test]
fn test_allow_suppresses_compiler_warning() {
    let session = TestSession::single(
        r#"
@allow("WC402")
if (true) {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@allow("WC402")
if (true) {
}

=== checked ===
@allow("WC402")
/// @decorator.node source="@allow(\"WC402\")" owner="if (true) {}" expression=allow target=decorator.allow type=allow kind=newtype parameters=(decorator.diagnostic.DiagnosticSelector) arguments=(provided("WC402") as decorator.diagnostic.DiagnosticSelector) newtype=decorator.diagnostic.allow backing=(decorator.diagnostic.DiagnosticSelector,) value="allow(\"WC402\")"
/// @type.node source=allow type=allow
/// @resolution.name source=allow target=decorator.diagnostic.allow
/// @type.node source="\"WC402\"" type="WC402"

if (true) {}
/// @type.node source="if (true) {}" type=void
/// @type.node source=true type=true
"#,
        r#""#,
    );
}

/// Promote one matching compiler warning to an error.
#[test]
fn test_deny_promotes_compiler_warning() {
    let session = TestSession::single(
        r#"
@deny("WC402")
if (true) {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@deny("WC402")
if (true) {
}

=== checked ===
@deny("WC402")
/// @decorator.node source="@deny(\"WC402\")" owner="if (true) {}" expression=deny target=decorator.deny type=deny kind=newtype parameters=(decorator.diagnostic.DiagnosticSelector) arguments=(provided("WC402") as decorator.diagnostic.DiagnosticSelector) newtype=decorator.diagnostic.deny backing=(decorator.diagnostic.DiagnosticSelector,) value="deny(\"WC402\")"
/// @type.node source=deny type=deny
/// @resolution.name source=deny target=decorator.diagnostic.deny
/// @type.node source="\"WC402\"" type="WC402"

if (true) {}
/// @type.node source="if (true) {}" type=void
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error code=WC402 message="condition is always true"
/// @diagnostic.label line=3 column=5 span="true" line_source="if (true) {}"
"#,
    );
}

/// Suppress one compiler warning that satisfies an expectation.
#[test]
fn test_expect_accepts_compiler_warning() {
    let session = TestSession::single(
        r#"
@expect("WC402")
if (true) {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@expect("WC402")
if (true) {
}

=== checked ===
@expect("WC402")
/// @decorator.node source="@expect(\"WC402\")" owner="if (true) {}" expression=expect target=decorator.expect type=expect kind=newtype parameters=(decorator.diagnostic.DiagnosticSelector) arguments=(provided("WC402") as decorator.diagnostic.DiagnosticSelector) newtype=decorator.diagnostic.expect backing=(decorator.diagnostic.DiagnosticSelector,) value="expect(\"WC402\")"
/// @type.node source=expect type=expect
/// @resolution.name source=expect target=decorator.diagnostic.expect
/// @type.node source="\"WC402\"" type="WC402"

if (true) {}
/// @type.node source="if (true) {}" type=void
/// @type.node source=true type=true
"#,
        r#""#,
    );
}

/// Reject one compiler warning expectation that did not occur.
#[test]
fn test_expect_requires_compiler_warning() {
    let session = TestSession::single(
        r#"
@expect("WC402", { reason: "intentional assertion" })
const value = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@expect("WC402", { reason: "intentional assertion" })
const value: 1 = 1;

=== checked ===
@expect("WC402", { reason: "intentional assertion" })
/// @decorator.node source="@expect(\"WC402\", { reason: \"intentional assertion\" })" owner="const value = 1" expression=expect target=decorator.expect type=expect kind=newtype parameters=(decorator.diagnostic.DiagnosticSelector, decorator.diagnostic.DiagnosticControlOptions) arguments=(provided("WC402") as decorator.diagnostic.DiagnosticSelector, provided({ reason: "intentional assertion" }) as decorator.diagnostic.DiagnosticControlOptions) newtype=decorator.diagnostic.expect backing=(decorator.diagnostic.DiagnosticSelector, decorator.diagnostic.DiagnosticControlOptions) value="expect(\"WC402\", { reason: \"intentional assertion\" })"
/// @type.node source=expect type=expect
/// @resolution.name source=expect target=decorator.diagnostic.expect
/// @type.node source="\"WC402\"" type="WC402"
/// @type.node source={ reason: "intentional assertion" } type={ reason: "intentional assertion" }
/// @type.node source="\"intentional assertion\"" type="intentional assertion"

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error code=EC700 message="expected diagnostic 'WC402' did not occur"
/// @diagnostic.label line=2 column=9 span="\"WC402\"" line_source="@expect(\"WC402\", { reason: \"intentional assertion\" })"
/// @diagnostic.note message="intentional assertion"
"#,
    );
}

/// Apply the alternate diagnostic level when a static condition is false.
#[test]
fn test_diagnostic_control_applies_otherwise_level() {
    let session = TestSession::single(
        r#"
@deny("WC402", { if: false, otherwise: "allow" })
if (true) {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@deny("WC402", { if: false, otherwise: "allow" })
if (true) {
}

=== checked ===
@deny("WC402", { if: false, otherwise: "allow" })
/// @decorator.node source="@deny(\"WC402\", { if: false, otherwise: \"allow\" })" owner="if (true) {}" expression=deny target=decorator.deny type=deny kind=newtype parameters=(decorator.diagnostic.DiagnosticSelector, decorator.diagnostic.DiagnosticControlOptions) arguments=(provided("WC402") as decorator.diagnostic.DiagnosticSelector, provided({ if: false, otherwise: "allow" }) as decorator.diagnostic.DiagnosticControlOptions) newtype=decorator.diagnostic.deny backing=(decorator.diagnostic.DiagnosticSelector, decorator.diagnostic.DiagnosticControlOptions) value="deny(\"WC402\", { if: false; otherwise: \"allow\" })"
/// @type.node source=deny type=deny
/// @resolution.name source=deny target=decorator.diagnostic.deny
/// @type.node source="\"WC402\"" type="WC402"
/// @type.node source={ if: false, otherwise: "allow" } type={ if: false; otherwise: "allow" }
/// @type.node source=false type=false
/// @type.node source="\"allow\"" type="allow"

if (true) {}
/// @type.node source="if (true) {}" type=void
/// @type.node source=true type=true
"#,
        r#""#,
    );
}

/// Preserve an enclosing forbid over an inner diagnostic control.
#[test]
fn test_forbid_controls_compiler_warning() {
    let session = TestSession::single(
        r#"
@forbid("WC402")
@allow("WC402")
if (true) {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@forbid("WC402")
@allow("WC402")
if (true) {
}

=== checked ===
@forbid("WC402")
/// @decorator.node source="@forbid(\"WC402\")" owner="if (true) {}" expression=forbid target=decorator.forbid type=forbid kind=newtype parameters=(decorator.diagnostic.DiagnosticSelector) arguments=(provided("WC402") as decorator.diagnostic.DiagnosticSelector) newtype=decorator.diagnostic.forbid backing=(decorator.diagnostic.DiagnosticSelector,) value="forbid(\"WC402\")"
/// @type.node source=forbid type=forbid
/// @resolution.name source=forbid target=decorator.diagnostic.forbid
/// @type.node source="\"WC402\"" type="WC402"

@allow("WC402")
/// @decorator.node source="@allow(\"WC402\")" owner="if (true) {}" expression=allow target=decorator.allow type=allow kind=newtype parameters=(decorator.diagnostic.DiagnosticSelector) arguments=(provided("WC402") as decorator.diagnostic.DiagnosticSelector) newtype=decorator.diagnostic.allow backing=(decorator.diagnostic.DiagnosticSelector,) value="allow(\"WC402\")"
/// @type.node source=allow type=allow
/// @resolution.name source=allow target=decorator.diagnostic.allow
/// @type.node source="\"WC402\"" type="WC402"

if (true) {}
/// @type.node source="if (true) {}" type=void
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error code=EC701 message="diagnostic 'WC402' is forbidden by an enclosing control"
/// @diagnostic.label line=3 column=8 span="\"WC402\"" line_source="@allow(\"WC402\")"
/// @diagnostic.related line=2 column=9 span="\"WC402\"" line_source="@forbid(\"WC402\")" message="forbidden here"
/// @diagnostic.error code=WC402 message="condition is always true"
/// @diagnostic.label line=4 column=5 span="true" line_source="if (true) {}"
"#,
    );
}
