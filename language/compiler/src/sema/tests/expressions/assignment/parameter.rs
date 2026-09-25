use crate::tests::{DirRows, TestSession};

#[test]
fn test_assignment_rejects_parameter_target() {
    let session = TestSession::single(
        r#"
function bump(value: int32): int32 {
    value = 2;
    return value;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function bump(value: int32): int32 {
    value = 2;
    return value;
}

=== dir ===
function bump(value: int32): int32 {
/// @type.symbol symbol=bump type=(int32) => int32
/// @type.symbol symbol=bump.value source="value: int32" type=int32

    value = 2;
    /// @type.node source="value = 2" type=2
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=bump.value
    /// @resolution.pattern.assign source=value kind=place
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=bump.value
    /// @resolution.assignment source=value write=binding(bump.value) type=int32
    /// @type.node source=2 type=2

    return value;
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=bump.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=bump.value

}
"#,
        r#"
/// @diagnostic.error id=cannot-assign-immutable-binding message="cannot assign to immutable binding 'value'"
/// @diagnostic.label line=3 column=5 span="value" line_source="value = 2;"
/// @diagnostic.related line=2 column=15 span="value" line_source="function bump(value: int32): int32 {" message="declared here"
/// @diagnostic.help message="declare 'value' with 'let' to allow reassignment"
"#,
    );
}

#[test]
fn test_compound_assignment_rejects_parameter_target() {
    let session = TestSession::single(
        r#"
function bump(value: int32): int32 {
    value += 2;
    return value;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function bump(value: int32): int32 {
    value += 2;
    return value;
}

=== dir ===
function bump(value: int32): int32 {
/// @type.symbol symbol=bump type=(int32) => int32
/// @type.symbol symbol=bump.value source="value: int32" type=int32

    value += 2;
    /// @type.node source="value += 2" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=bump.value
    /// @resolution.operator source="value += 2" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 2 as int32 families=(integer)]
    /// @resolution.pattern.assign source=value kind=place
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=value read=binding(bump.value) write=binding(bump.value) type=int32
    /// @resolution.access source=value root=bump.value
    /// @type.node source=2 type=2

    return value;
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=bump.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=bump.value

}
"#,
        r#"
/// @diagnostic.error id=cannot-assign-immutable-binding message="cannot assign to immutable binding 'value'"
/// @diagnostic.label line=3 column=5 span="value" line_source="value += 2;"
/// @diagnostic.related line=2 column=15 span="value" line_source="function bump(value: int32): int32 {" message="declared here"
/// @diagnostic.help message="declare 'value' with 'let' to allow reassignment"
"#,
    );
}

#[test]
fn test_parameter_shadowing_rebinds_value() {
    let session = TestSession::single(
        r#"
function bump(value: int32): int32 {
    let value = value + 1;
    return value;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function bump(value: int32): int32 {
    let value: int32 = value + 1;
    return value;
}

=== dir ===
function bump(value: int32): int32 {
/// @type.symbol symbol=bump type=(int32) => int32
/// @type.symbol symbol=bump.value#1 source="value: int32" type=int32

    let value = value + 1;
    /// @type.symbol symbol=bump.value#2 source=value type=int32
    /// @resolution.pattern source=value kind=binding target=bump.value#2
    /// @type.node source="value + 1" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=bump.value#1
    /// @resolution.operator source="value + 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=bump.value#1
    /// @type.node source=1 type=1

    return value;
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=bump.value#2
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=bump.value#2

}
"#,
    );
}
