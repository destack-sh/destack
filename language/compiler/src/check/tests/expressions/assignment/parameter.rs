use crate::tests::{DirRows, TestSession};

#[test]
fn test_parameter_binding_accepts_assignment() {
    let session = TestSession::single(
        r#"
function bump(value: int32): int32 {
    value = 2;
    return value;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function bump(value: int32): int32 {
    value = 2;
    return value;
}

=== checked ===
function bump(value: int32): int32 {
/// @type.symbol symbol=bump type=(int32) => int32
/// @type.symbol symbol=value source="value: int32" type=int32

    value = 2;
    /// @type.node source="value = 2" type=2
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.pattern.assign source=value kind=place place=value
    /// @type.node source=2 type=2

    return value;
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value

}
"#,
    );
}

#[test]
fn test_parameter_assignment_rejects_incompatible_value() {
    let session = TestSession::single(
        r#"
function bump(value: int32): void {
    value = "no";
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function bump(value: int32): void {
    value = "no";
}

=== checked ===
function bump(value: int32): void {
/// @type.symbol symbol=bump type=(int32) => void
/// @type.symbol symbol=value source="value: int32" type=int32

    value = "no";
    /// @type.node source="value = \"no\"" type="no"
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.pattern.assign source=value kind=place place=value
    /// @type.node source="\"no\"" type="no"

}
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"no\"' is not assignable to type 'int32'"
/// @diagnostic.label line=3 column=13 span="\"no\"" line_source="value = \"no\";"
"#,
    );
}

#[test]
fn test_parameter_binding_accepts_compound_assignment() {
    let session = TestSession::single(
        r#"
function bump(value: int32): int32 {
    value += 2;
    return value;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function bump(value: int32): int32 {
    value += 2;
    return value;
}

=== checked ===
function bump(value: int32): int32 {
/// @type.symbol symbol=bump type=(int32) => int32
/// @type.symbol symbol=value source="value: int32" type=int32

    value += 2;
    /// @type.node source="value += 2" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.call source="value += 2" parameters=() return=int32 kind=builtin builtin=binary.add
    /// @resolution.pattern.assign source=value kind=place place=value
    /// @type.node source=2 type=2

    return value;
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value

}
"#,
    );
}
