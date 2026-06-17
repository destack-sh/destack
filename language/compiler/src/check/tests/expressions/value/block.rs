use crate::tests::{DirRows, TestSession};

#[test]
fn test_if_expression_joins_branch_values() {
    let session = TestSession::single(
        r#"
declare const enabled: boolean;

const value = if (enabled) {
    1
} else {
    2
};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const enabled: boolean;

const value: float64 = if (enabled) {
    1
} else {
    2
};

=== checked ===
declare const enabled: boolean;
/// @type.symbol symbol=enabled source=enabled type=boolean

const value = if (enabled) {
/// @type.symbol symbol=value source=value type=float64
/// @type.node type=float64
/// @type.node source=enabled type=boolean
/// @resolution.name source=enabled target=enabled

    1
    /// @type.node source=1 type=float64

} else {
    2
    /// @type.node source=2 type=float64

};
"#,
    );
}

#[test]
fn test_function_body_uses_tail_expression_return() {
    let session = TestSession::single(
        r#"
function add(left: int32, right: int32): int32 {
    left + right
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function add(left: int32, right: int32): int32 {
    left + right
}

=== checked ===
function add(left: int32, right: int32): int32 {
/// @type.symbol symbol=add type=(int32, int32) => int32
/// @type.symbol symbol=left source="left: int32" type=int32
/// @type.symbol symbol=right source="right: int32" type=int32

    left + right
    /// @type.node source="left + right" type=int32
    /// @type.node source=left type=int32
    /// @resolution.name source=left target=left
    /// @resolution.call source="left + right" parameters=() return=int32 kind=builtin builtin=binary.add
    /// @type.node source=right type=int32
    /// @resolution.name source=right target=right

}
"#,
    );
}
