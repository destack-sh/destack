use crate::tests::{DirRows, TestSession};

#[test]
fn test_free_function_call_selects_function_symbol() {
    let session = TestSession::single(
        r#"
function add(left: int32, right: int32): int32 {
    return left + right;
}

const value = add(1, 2);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types().without_reference_types(),
        r#"
function add(left: int32, right: int32): int32 {
/// @type.symbol symbol=add type=(int32, int32) => int32
/// @type.symbol symbol=left type=int32
/// @type.symbol symbol=right type=int32

    return left + right;
    /// @type.node source="left + right" type=int32
    /// @resolution.name source=left target=left
    /// @resolution.call source="left + right" parameters=(int32, int32) return=int32 kind=builtin builtin=binary.add
    /// @resolution.name source=right target=right

}

const value = add(1, 2);
/// @type.symbol symbol=value type=int32
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) return=int32 kind=symbol target=add
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32
"#);
}

#[test]
fn test_imported_function_call_selects_exported_symbol() {
    let compiler = TestSession::new()
        .module(
            "math.ds",
            r#"
export function add(left: int32, right: int32): int32 {
    return left + right;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { add } from "./math.ds";

const value = add(1, 2);
"#,
        )
        .build();

    compiler.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types().without_reference_types(),
        r#"
import { add } from "./math.ds";

const value = add(1, 2);
/// @type.symbol symbol=value type=int32
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=math.add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) return=int32 kind=symbol target=math.add
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32
"#);
}
