use crate::tests::{DirRows, TestSession};

#[test]
fn test_initializer_assignment_mismatch_reports_error() {
    let session = TestSession::single(
        r#"
const value: int32 = "text";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: int32 = "text";
/// @type.symbol symbol=value type=int32
/// @type.node source="\"text\"" type="text"

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=22 source="const value: int32 = \"text\";"
"#,
    );
}

#[test]
fn test_readonly_member_assignment_reports_error() {
    let session = TestSession::single(
        r#"
const state: { readonly count: int32 } = { count: 0 };
state.count = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const state: { readonly count: int32 } = { count: 0 };
/// @type.symbol symbol=state type={ readonly count: int32 }
/// @type.symbol symbol=count type=int32
/// @type.node source="{ count: 0 }" type={ count: int32 }
/// @type.node source=0 type=int32

state.count = 1;
/// @type.node source="state.count = 1" type=int32
/// @type.node source=state type={ readonly count: int32 }
/// @type.node source=state.count type=int32
/// @resolution.name source=state target=state
/// @resolution.member source=state.count receiver={ readonly count: int32 } kind=field key=count
/// @type.node source=1 type=int32
"#,
        r#"
/// @diagnostic.error code=EC204 message="assignment target is not writable"
/// @diagnostic.label line=3 column=1 source="state.count = 1;"
"#,
    );
}
