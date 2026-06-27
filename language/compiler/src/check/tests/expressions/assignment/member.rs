use crate::tests::{DirRows, TestSession};

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
/// @type.node source={ count: 0 } type={ readonly count: int32 }
/// @type.node source=0 type=0

state.count = 1;
/// @type.node source="state.count = 1" type=1
/// @type.node source=state type={ readonly count: int32 }
/// @type.node source=state.count type=int32
/// @resolution.name source=state target=state
/// @resolution.member source=state.count receiver={ readonly count: int32 } kind=field key=count
/// @resolution.pattern.assign source=state.count kind=place place=state.count
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=5 constraints=3 obligations=1 solutions=0 bounds=0 decisions=3
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'count'"
/// @diagnostic.label line=3 column=7 span="count" line_source="state.count = 1;"
"#,
    );
}
