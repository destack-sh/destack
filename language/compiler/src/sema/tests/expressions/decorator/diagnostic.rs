use crate::tests::{DirRows, TestSession};

/// Suppress one matching compiler warning.
#[test]
fn test_allow_suppresses_compiler_warning() {
    let session = TestSession::single(
        r#"
@allow("constant-condition")
if (true) {}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@allow("constant-condition")
if (true) {
}

=== dir ===
@allow("constant-condition")
/// @decorator.node source="@allow(\"constant-condition\")" owner="if (true) {}" expression=allow target=decorator.allow type=allow kind=newtype parameters=(DiagnosticId) arguments=(provided("constant-condition") as DiagnosticId) newtype=allow backing=(DiagnosticId,) value="allow(\"constant-condition\")"
/// @type.node source=allow type=allow
/// @resolution.name source=allow target=allow
/// @type.node source="\"constant-condition\"" type="constant-condition"

if (true) {}
/// @type.node source="if (true) {}" type=void
/// @type.node source=true type=true
"#,
    );
}

/// Promote one matching compiler warning to an error.
#[test]
fn test_deny_promotes_compiler_warning() {
    let session = TestSession::single(
        r#"
@deny("constant-condition")
if (true) {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
@deny("constant-condition")
if (true) {
}

=== dir ===
@deny("constant-condition")
/// @decorator.node source="@deny(\"constant-condition\")" owner="if (true) {}" expression=deny target=decorator.deny type=deny kind=newtype parameters=(DiagnosticId) arguments=(provided("constant-condition") as DiagnosticId) newtype=deny backing=(DiagnosticId,) value="deny(\"constant-condition\")"
/// @type.node source=deny type=deny
/// @resolution.name source=deny target=deny
/// @type.node source="\"constant-condition\"" type="constant-condition"

if (true) {}
/// @type.node source="if (true) {}" type=void
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error id=constant-condition message="condition is always true"
/// @diagnostic.label line=3 column=5 span="true" line_source="if (true) {}"
"#,
    );
}

/// Suppress one compiler warning that satisfies an expectation.
#[test]
fn test_expect_accepts_compiler_warning() {
    let session = TestSession::single(
        r#"
@expect("constant-condition")
if (true) {}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@expect("constant-condition")
if (true) {
}

=== dir ===
@expect("constant-condition")
/// @decorator.node source="@expect(\"constant-condition\")" owner="if (true) {}" expression=expect target=decorator.expect type=expect kind=newtype parameters=(DiagnosticId) arguments=(provided("constant-condition") as DiagnosticId) newtype=expect backing=(DiagnosticId,) value="expect(\"constant-condition\")"
/// @type.node source=expect type=expect
/// @resolution.name source=expect target=expect
/// @type.node source="\"constant-condition\"" type="constant-condition"

if (true) {}
/// @type.node source="if (true) {}" type=void
/// @type.node source=true type=true
"#,
    );
}

/// Reject one compiler warning expectation that did not occur.
#[test]
fn test_expect_requires_compiler_warning() {
    let session = TestSession::single(
        r#"
@expect("constant-condition", { reason: "intentional assertion" })
const value = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
@expect("constant-condition", { reason: "intentional assertion" } as DiagnosticControlOptions)
const value: 1 = 1;

=== dir ===
@expect("constant-condition", { reason: "intentional assertion" })
/// @decorator.node source="@expect(\"constant-condition\", { reason: \"intentional assertion\" })" owner="const value = 1" expression=expect target=decorator.expect type=expect kind=newtype parameters=(DiagnosticId, DiagnosticControlOptions) arguments=(provided("constant-condition") as DiagnosticId, provided({ reason: "intentional assertion" }) as DiagnosticControlOptions) newtype=expect backing=(DiagnosticId, DiagnosticControlOptions) value="expect(\"constant-condition\", { reason: \"intentional assertion\" })"
/// @type.node source=expect type=expect
/// @resolution.name source=expect target=expect
/// @type.node source="\"constant-condition\"" type="constant-condition"
/// @type.node source={ reason: "intentional assertion" } type={ reason: string; if?: never; otherwise?: never }
/// @type.node source="\"intentional assertion\"" type="intentional assertion"

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=unmet-diagnostic-expectation message="expected diagnostic 'constant-condition' did not occur"
/// @diagnostic.label line=2 column=9 span="\"constant-condition\"" line_source="@expect(\"constant-condition\", { reason: \"intentional assertion\" })"
/// @diagnostic.note message="intentional assertion"
"#,
    );
}

/// Apply the alternate diagnostic level when a static condition is false.
#[test]
fn test_diagnostic_control_applies_otherwise_level() {
    let session = TestSession::single(
        r#"
@deny("constant-condition", { if: false, otherwise: "allow" })
if (true) {}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@deny("constant-condition", {
    if: false,
    otherwise: "allow" as "allow" | "warn" | "deny" | "forbid" | undefined,
} as DiagnosticControlOptions)
if (true) {
}

=== dir ===
@deny("constant-condition", { if: false, otherwise: "allow" })
/// @decorator.node source="@deny(\"constant-condition\", { if: false, otherwise: \"allow\" })" owner="if (true) {}" expression=deny target=decorator.deny type=deny kind=newtype parameters=(DiagnosticId, DiagnosticControlOptions) arguments=(provided("constant-condition") as DiagnosticId, provided({ if: false, otherwise: "allow" }) as DiagnosticControlOptions) newtype=deny backing=(DiagnosticId, DiagnosticControlOptions) value="deny(\"constant-condition\", { if: false; otherwise: \"allow\" })"
/// @type.node source=deny type=deny
/// @resolution.name source=deny target=deny
/// @type.node source="\"constant-condition\"" type="constant-condition"
/// @type.node source={ if: false, otherwise: "allow" } type={ reason?: string; if: boolean; otherwise?: "allow" | "warn" | "deny" | "forbid" }
/// @type.node source=false type=false
/// @type.node source="\"allow\"" type="allow"

if (true) {}
/// @type.node source="if (true) {}" type=void
/// @type.node source=true type=true
"#,
    );
}

/// Preserve an enclosing forbid over an inner diagnostic control.
#[test]
fn test_forbid_controls_compiler_warning() {
    let session = TestSession::single(
        r#"
@forbid("constant-condition")
@allow("constant-condition")
if (true) {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
@forbid("constant-condition")
@allow("constant-condition")
if (true) {
}

=== dir ===
@forbid("constant-condition")
/// @decorator.node source="@forbid(\"constant-condition\")" owner="if (true) {}" expression=forbid target=decorator.forbid type=forbid kind=newtype parameters=(DiagnosticId) arguments=(provided("constant-condition") as DiagnosticId) newtype=forbid backing=(DiagnosticId,) value="forbid(\"constant-condition\")"
/// @type.node source=forbid type=forbid
/// @resolution.name source=forbid target=forbid
/// @type.node source="\"constant-condition\"" type="constant-condition"

@allow("constant-condition")
/// @decorator.node source="@allow(\"constant-condition\")" owner="if (true) {}" expression=allow target=decorator.allow type=allow kind=newtype parameters=(DiagnosticId) arguments=(provided("constant-condition") as DiagnosticId) newtype=allow backing=(DiagnosticId,) value="allow(\"constant-condition\")"
/// @type.node source=allow type=allow
/// @resolution.name source=allow target=allow
/// @type.node source="\"constant-condition\"" type="constant-condition"

if (true) {}
/// @type.node source="if (true) {}" type=void
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error id=forbidden-diagnostic-override message="diagnostic 'constant-condition' is forbidden by an enclosing control"
/// @diagnostic.label line=3 column=8 span="\"constant-condition\"" line_source="@allow(\"constant-condition\")"
/// @diagnostic.related line=2 column=9 span="\"constant-condition\"" line_source="@forbid(\"constant-condition\")" message="forbidden here"
/// @diagnostic.error id=constant-condition message="condition is always true"
/// @diagnostic.label line=4 column=5 span="true" line_source="if (true) {}"
"#,
    );
}

/// Reject a diagnostic control whose id is not registered.
#[test]
fn test_rejects_unknown_diagnostic_id() {
    let session = TestSession::single(
        r#"
@allow("not-a-diagnostic")
const value = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
@allow("not-a-diagnostic")
const value: 1 = 1;

=== dir ===
@allow("not-a-diagnostic")
/// @decorator.node source="@allow(\"not-a-diagnostic\")" owner="const value = 1" expression=allow target=decorator.allow type=allow kind=newtype parameters=(DiagnosticId) arguments=(provided("not-a-diagnostic") as DiagnosticId) newtype=allow backing=(DiagnosticId,) value="allow(\"not-a-diagnostic\")"
/// @type.node source=allow type=allow
/// @resolution.name source=allow target=allow
/// @type.node source="\"not-a-diagnostic\"" type="not-a-diagnostic"

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=unknown-diagnostic message="unknown diagnostic 'not-a-diagnostic'"
/// @diagnostic.label line=2 column=8 span="\"not-a-diagnostic\"" line_source="@allow(\"not-a-diagnostic\")"
"#,
    );
}

/// Reject a diagnostic control whose registered diagnostic is not controllable.
#[test]
fn test_rejects_uncontrollable_diagnostic_id() {
    let session = TestSession::single(
        r#"
@allow("not-assignable")
const value = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
@allow("not-assignable")
const value: 1 = 1;

=== dir ===
@allow("not-assignable")
/// @decorator.node source="@allow(\"not-assignable\")" owner="const value = 1" expression=allow target=decorator.allow type=allow kind=newtype parameters=(DiagnosticId) arguments=(provided("not-assignable") as DiagnosticId) newtype=allow backing=(DiagnosticId,) value="allow(\"not-assignable\")"
/// @type.node source=allow type=allow
/// @resolution.name source=allow target=allow
/// @type.node source="\"not-assignable\"" type="not-assignable"

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=uncontrollable-diagnostic message="diagnostic 'not-assignable' cannot be controlled"
/// @diagnostic.label line=2 column=8 span="\"not-assignable\"" line_source="@allow(\"not-assignable\")"
"#,
    );
}
