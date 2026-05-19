use super::super::snapshot::{assert_check_module_snapshot, assert_check_snapshot};
use crate::tests::TestCompiler;

#[test]
fn test_check_records_free_call_resolution() {
    assert_check_snapshot(
        r#"
function add(left: int32, right: int32): int32 {
    return left + right;
}

const value = add(1, 2);
"#,
        r#"
function add(left: int32, right: int32): int32 {
/// @type.symbol key=add value=(int32, int32) => int32

    return left + right;
}

const value = add(1, 2);
/// @resolution.name source=add target=add
/// @resolution.call source="add(1, 2)" parameters=[int32, int32] return=int32 kind=direct target=add
/// @type.node source="add(1, 2)" value=int32
/// @type.symbol key=value value=int32

/// @type.summary types=4 nodes=1 symbols=4
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=1 labels=0 members=0 calls=1
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}

#[test]
fn test_check_records_imported_call_targets() {
    let compiler = TestCompiler::new()
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

    assert_check_module_snapshot(
        &compiler,
        "main.ds",
        r#"
import { add } from "./math.ds";
/// @resolution.name source=add target=math.add

const value = add(1, 2);
/// @resolution.name source=add target=math.add
/// @resolution.call source="add(1, 2)" parameters=[int32, int32] return=int32 kind=direct target=math.add
/// @type.node source="add(1, 2)" value=int32
/// @type.symbol key=value value=int32

/// @type.summary types=3 nodes=1 symbols=1
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=2 labels=0 members=0 calls=1
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}
