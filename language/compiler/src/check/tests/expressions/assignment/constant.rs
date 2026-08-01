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
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value = 2;
/// @type.node source="value = 2" type=2
/// @type.node source=value type=int32
/// @resolution.pattern.assign source=value kind=place
/// @resolution.assignment source=value write=binding(value) type=int32
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=5 constraints=0 obligations=2 solutions=0 bounds=0 decisions=3
"#,
        r#"
/// @diagnostic.error id=cannot-assign-immutable-binding message="cannot assign to immutable binding 'value'"
/// @diagnostic.label line=3 column=1 span="value" line_source="value = 2;"
/// @diagnostic.related line=2 column=7 span="value" line_source="const value: int32 = 1;" message="declared here"
/// @diagnostic.help message="declare 'value' with 'let' to allow reassignment"
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
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value += 2;
/// @type.node source="value += 2" type=int32
/// @type.node source=value type=int32
/// @resolution.operator source="value += 2" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 2 as int32 families=(integer)]
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.assignment source=value read=binding(value) write=binding(value) type=int32
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=8 constraints=0 obligations=2 solutions=0 bounds=0 decisions=4
"#,
        r#"
/// @diagnostic.error id=cannot-assign-immutable-binding message="cannot assign to immutable binding 'value'"
/// @diagnostic.label line=3 column=1 span="value" line_source="value += 2;"
/// @diagnostic.related line=2 column=7 span="value" line_source="const value: int32 = 1;" message="declared here"
/// @diagnostic.help message="declare 'value' with 'let' to allow reassignment"
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
/// @resolution.pattern source=state kind=binding target=state
/// @type.node source={ count: 0 } type={ count: int32 }
/// @type.node source=0 type=0

state.count = 1;
/// @type.node source="state.count = 1" type=1
/// @type.node source=state type={ count: int32 }
/// @type.node source=state.count type=int32
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=state root=state
/// @resolution.pattern.assign source=state.count kind=place
/// @resolution.assignment source=state.count write="receiver={ count: int32 }, target=field(receiver={ count: int32 }, target=count, type=int32), type=int32" type=int32
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=9 constraints=0 obligations=2 solutions=0 bounds=0 decisions=4
"#,
    );
}
