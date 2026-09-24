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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const enabled: boolean;

const value: 1 | 2 = if (enabled) {
    1
} else {
    2
};

=== dir ===
declare const enabled: boolean;
/// @type.symbol symbol=enabled source=enabled type=boolean
/// @resolution.pattern source=enabled kind=binding target=enabled

const value = if (enabled) {
/// @type.symbol symbol=value source=value type=1 | 2
/// @resolution.pattern source=value kind=binding target=value
/// @type.node type=1 | 2
/// @type.node source=enabled type=boolean
/// @resolution.name source=enabled target=enabled
/// @resolution.place source=enabled placement="local" lifetime="static" access="immutable"
/// @resolution.access source=enabled root=enabled

    1
    /// @type.node source=1 type=1

} else {
    2
    /// @type.node source=2 type=2

};
"#,
    );
}

#[test]
fn test_if_expression_widens_a_literal_branch_into_its_join() {
    let session = TestSession::single(
        r#"
function pick(count: isize): isize {
    let kept = if (count < 0) { 0 } else { count };

    return kept;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
function pick(count: isize): isize {
    let kept: isize = if (count < 0) { 0 } else { count };

    return kept;
}

=== dir ===
function pick(count: isize): isize {
/// @type.symbol symbol=pick type=(isize) => isize
/// @type.symbol symbol=pick.count source="count: isize" type=isize

    let kept = if (count < 0) { 0 } else { count };
    /// @type.symbol symbol=pick.kept source=kept type=isize
    /// @resolution.pattern source=kept kind=binding target=pick.kept
    /// @type.node source="if (count < 0) { 0 } else { count }" type=isize
    /// @type.node source="count < 0" type=boolean
    /// @type.node source=count type=isize
    /// @resolution.name source=count target=pick.count
    /// @resolution.operator source="count < 0" type=boolean operator="<" kind=builtin operands=[count as isize families=(integer), 0 as isize families=(integer)]
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=pick.count
    /// @type.node source=0 type=0
    /// @coercion.node source=0 from=0 adjustments=[{ kind: materialize, target: isize }] origin=implicit
    /// @type.node source=0 type=0
    /// @coercion.node source=0 from=0 adjustments=[{ kind: materialize, target: isize }] origin=implicit
    /// @type.node source=count type=isize
    /// @resolution.name source=count target=pick.count
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=pick.count

    return kept;
    /// @type.node source=kept type=isize
    /// @resolution.name source=kept target=pick.kept
    /// @resolution.place source=kept placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=kept root=pick.kept

}
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function add(left: int32, right: int32): int32 {
    left + right
}

=== dir ===
function add(left: int32, right: int32): int32 {
/// @type.symbol symbol=add type=(int32, int32) => int32
/// @type.symbol symbol=add.left source="left: int32" type=int32
/// @type.symbol symbol=add.right source="right: int32" type=int32

    left + right
    /// @type.node source="left + right" type=int32
    /// @type.node source=left type=int32
    /// @resolution.name source=left target=add.left
    /// @resolution.operator source="left + right" type=int32 operator="+" kind=builtin operands=[left as int32 families=(integer), right as int32 families=(integer)]
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=add.left
    /// @type.node source=right type=int32
    /// @resolution.name source=right target=add.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=add.right

}
"#,
    );
}
