use crate::tests::{DirRows, TestSession};

/// Require one nominal enum member per runtime value.
#[test]
fn test_rejects_duplicate_enum_variant_values() {
    let session = TestSession::single(
        r#"
enum Status {
    Ready = 1,
    Active = 1,
    Waiting,
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Status {
    Ready = 1,
    Active = 1,
    Waiting,
}

=== dir ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Ready source="Ready = 1" key=Ready value=1
/// @definition.variant symbol=Status.Waiting source=Waiting key=Waiting value=2

    Ready = 1,
    /// @type.symbol symbol=Status.Ready source="Ready = 1" type=Status.Ready
    /// @type.node source=1 type=1

    Active = 1,
    /// @type.symbol symbol=Status.Active source="Active = 1" type=<error>
    /// @type.node source=1 type=1

    Waiting,
    /// @type.symbol symbol=Status.Waiting source=Waiting type=Status.Waiting

}
"#,
        r#"
/// @diagnostic.error id=duplicate-enum-variant-value message="enum variant value '1' is already declared"
/// @diagnostic.label line=4 column=5 span="Active" line_source="Active = 1,"
/// @diagnostic.related line=3 column=5 span="Ready" line_source="Ready = 1," message="first declared here"
"#,
    );
}

/// Reject enum values outside the integer and string domains.
#[test]
fn test_rejects_invalid_enum_variant_type() {
    let session = TestSession::single(
        r#"
enum Status {
    Ready = true,
    Waiting,
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Status {
    Ready = true,
    Waiting,
}

=== dir ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status

    Ready = true,
    /// @type.symbol symbol=Status.Ready source="Ready = true" type=<error>
    /// @type.node source=true type=true

    Waiting,
    /// @type.symbol symbol=Status.Waiting source=Waiting type=<error>

}
"#,
        r#"
/// @diagnostic.error id=invalid-enum-variant-type message="enum variant value must be an integer or string constant, received 'true'"
/// @diagnostic.label line=3 column=13 span="true" line_source="Ready = true,"
"#,
    );
}

/// Reject enum declarations that mix integer and string values.
#[test]
fn test_rejects_mixed_enum_variant_domains() {
    let session = TestSession::single(
        r#"
enum Status {
    Ready = 1,
    Failed = "failed",
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Status {
    Ready = 1,
    Failed = "failed",
}

=== dir ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Ready source="Ready = 1" key=Ready value=1

    Ready = 1,
    /// @type.symbol symbol=Status.Ready source="Ready = 1" type=Status.Ready
    /// @type.node source=1 type=1

    Failed = "failed",
    /// @type.symbol symbol=Status.Failed source="Failed = \"failed\"" type=<error>
    /// @type.node source="\"failed\"" type="failed"

}
"#,
        r#"
/// @diagnostic.error id=mixed-enum-variant-domain message="enum variants must all use the same scalar domain"
/// @diagnostic.label line=4 column=5 span="Failed" line_source="Failed = \"failed\","
"#,
    );
}

/// Require explicit values after a string enum variant.
#[test]
fn test_requires_explicit_string_enum_variant_values() {
    let session = TestSession::single(
        r#"
enum Status {
    Ready = "ready",
    Failed,
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Status {
    Ready = "ready",
    Failed,
}

=== dir ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status backing=string
/// @definition.variant symbol=Status.Ready source="Ready = \"ready\"" key=Ready value="\"ready\""

    Ready = "ready",
    /// @type.symbol symbol=Status.Ready source="Ready = \"ready\"" type=Status.Ready
    /// @type.node source="\"ready\"" type="ready"

    Failed,
    /// @type.symbol symbol=Status.Failed source=Failed type=<error>

}
"#,
        r#"
/// @diagnostic.error id=implicit-string-enum-variant message="string backed enum variants require explicit values"
/// @diagnostic.label line=4 column=5 span="Failed" line_source="Failed,"
"#,
    );
}

/// Reject implicit integer enum values beyond the int64 domain.
#[test]
fn test_rejects_enum_variant_value_overflow() {
    let session = TestSession::single(
        r#"
enum Status {
    Ready = 9223372036854775807,
    Failed,
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Status {
    Ready = 9223372036854775807,
    Failed,
}

=== dir ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Ready source="Ready = 9223372036854775807" key=Ready value=9223372036854775807

    Ready = 9223372036854775807,
    /// @type.symbol symbol=Status.Ready source="Ready = 9223372036854775807" type=Status.Ready
    /// @type.node source=9223372036854775807 type=9223372036854775807

    Failed,
    /// @type.symbol symbol=Status.Failed source=Failed type=<error>

}
"#,
        r#"
/// @diagnostic.error id=enum-variant-value-overflow message="implicit enum variant value overflows int64"
/// @diagnostic.label line=4 column=5 span="Failed" line_source="Failed,"
"#,
    );
}
