use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_binding_rejects_assignment() {
    let session = TestSession::single(
        r#"
const value: int32 = 1;
value = 2;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: int32 = 1;
value = 2;

=== dir ===
const value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value = 2;
/// @type.node source="value = 2" type=2
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=int32
/// @type.node source=2 type=2
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: int32 = 1;
value += 2;

=== dir ===
const value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value += 2;
/// @type.node source="value += 2" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.operator source="value += 2" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 2 as int32 families=(integer)]
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.assignment source=value read=binding(value) write=binding(value) type=int32
/// @resolution.access source=value root=value
/// @type.node source=2 type=2
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const state: { count: int32 } = { count: 0 };
state.count = 1;

=== dir ===
const state: { count: int32 } = { count: 0 };
/// @type.symbol symbol=state source=state type={ count: int32 }
/// @resolution.pattern source=state kind=binding target=state
/// @type.symbol symbol=count source="count: int32" type=int32
/// @type.node source={ count: 0 } type={ count: int32 }
/// @type.node source=0 type=0

state.count = 1;
/// @type.node source="state.count = 1" type=1
/// @type.node source=state type={ count: int32 }
/// @type.node source=state.count type=int32
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state root=state
/// @resolution.pattern.assign source=state.count kind=place
/// @resolution.place source=state.count placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=state.count root=state keys=[count]
/// @resolution.assignment source=state.count write="receiver={ count: int32 }, target=field(receiver={ count: int32 }, target=count, type=int32), type=int32" type=int32
/// @type.node source=1 type=1
"#,
    );
}
