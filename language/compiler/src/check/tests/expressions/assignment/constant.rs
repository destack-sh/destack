use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_binding_rejects_assignment() {
    let session = TestSession::single(
        r#"
const value: int32 = 1;
value = 2;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: int32 = 1;
value = 2;

=== checked ===
const value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=1 type=1

value = 2;
/// @type.node source="value = 2" type=2
/// @type.node source=value type=int32
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=int32
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=4 constraints=2 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error code=EC212 message="cannot assign to immutable binding 'value'"
/// @diagnostic.label line=3 column=1 span="value" line_source="value = 2;"
"#,
    );
}

#[test]
fn test_const_binding_rejects_compound_assignment() {
    let session = TestSession::single(
        r#"
const value: int32 = 1;
value += 2;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: int32 = 1;
value += 2;

=== checked ===
const value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=1 type=1

value += 2;
/// @type.node source="value += 2" type=int32
/// @type.node source=value type=int32
/// @resolution.call source="value += 2" parameters=() return=int32 kind=builtin builtin=binary.add
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=int32
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=4 constraints=2 obligations=1 solutions=0 bounds=0 decisions=2
"#,
        r#"
/// @diagnostic.error code=EC212 message="cannot assign to immutable binding 'value'"
/// @diagnostic.label line=3 column=1 span="value" line_source="value += 2;"
"#,
    );
}

#[test]
fn test_const_binding_accepts_member_assignment() {
    let session = TestSession::single(
        r#"
const state: { count: int32 } = { count: 0 };
state.count = 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const state: { count: int32 } = { count: 0 };
state.count = 1;

=== checked ===
const state: { count: int32 } = { count: 0 };
/// @type.symbol symbol=state source=state type={ count: int32 }
/// @type.node source={ count: 0 } type={ count: 0 }
/// @type.node source=0 type=0

state.count = 1;
/// @type.node source="state.count = 1" type=1
/// @type.node source=state type={ count: int32 }
/// @type.node source=state.count type=int32
/// @resolution.name source=state target=state
/// @resolution.pattern.assign source=state.count kind=place place=field(count) type=int32
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=6 constraints=3 obligations=1 solutions=0 bounds=0 decisions=2
"#,
    );
}
