use super::super::snapshot::assert_check_snapshots;
use crate::tests::TestCompiler;

#[test]
fn test_check_records_circular_struct_type_targets() {
    let compiler = TestCompiler::new()
        .module(
            "left.ds",
            r#"
import { Right } from "./right.ds";

export struct Left {
    right: Right;
}
"#,
        )
        .module(
            "right.ds",
            r#"
import { Left } from "./left.ds";

export struct Right {
    left: Left;
}
"#,
        )
        .build();

    assert_check_snapshots(
        &compiler,
        &["left.ds", "right.ds"],
        r#"
=== left.ds ===
import { Right } from "./right.ds";
/// @resolution.name source=Right target=right.Right

export struct Left {
/// @type.symbol key=Left value=Left

    right: Right;
    /// @resolution.name source=Right target=right.Right
    /// @type.symbol key=Left.right value=right.Right
}

/// @type.summary types=2 nodes=0 symbols=2
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=2 labels=0 members=0 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0

=== right.ds ===
import { Left } from "./left.ds";
/// @resolution.name source=Left target=left.Left

export struct Right {
/// @type.symbol key=Right value=Right

    left: Left;
    /// @resolution.name source=Left target=left.Left
    /// @type.symbol key=Right.left value=left.Left
}

/// @type.summary types=2 nodes=0 symbols=2
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=2 labels=0 members=0 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}
