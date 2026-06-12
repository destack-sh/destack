use crate::tests::{DirRows, TestSession};

#[test]
fn test_initializer_rejects_incompatible_value() {
    let session = TestSession::single(
        r#"
const value: int32 = "text";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: int32 = "text";

=== checked ===
const value: int32 = "text";
/// @type.symbol symbol=value source=value type=int32
/// @type.node source="\"text\"" type="text"

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"text\"' is not assignable to type 'int32'"
/// @diagnostic.label line=2 column=22 source="const value: int32 = \"text\";"
"#,
    );
}

#[test]
fn test_readonly_member_rejects_assignment() {
    let session = TestSession::single(
        r#"
const state: { readonly count: int32 } = { count: 0 };
state.count = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const state: { readonly count: int32 } = { count: 0 };
state.count = 1;

=== checked ===
const state: { readonly count: int32 } = { count: 0 };
/// @type.symbol symbol=state source=state type={ readonly count: int32 }
/// @type.node source="{ count: 0 }" type=Managed<{ count: 0 }>
/// @type.node source=0 type=0

state.count = 1;
/// @type.node source="state.count = 1" type=1
/// @type.node source=state type={ readonly count: int32 }
/// @type.node source=state.count type=int32
/// @resolution.name source=state target=state
/// @resolution.member source=state.count receiver={ readonly count: int32 } kind=field key=count
/// @type.node source=1 type=1

/// @check.stats.solve variables=1 types=8 constraints=3 obligations=1 solutions=1 bounds=1 decisions=2
"#,
        r#"
/// @diagnostic.error code=EC204 message="cannot assign to '{ count: int32 }.count': member 'count' is readonly"
/// @diagnostic.label line=3 column=1 source="state.count = 1;"
"#,
    );
}
