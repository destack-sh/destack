use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_binding_rejects_assignment() {
    let session = TestSession::builder()
        .module(
            "counter.ds",
            r#"
export let counter: int32 = 0;
"#,
        )
        .module(
            "main.ds",
            r#"
import { counter } from "./counter.ds";

counter = 1;
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
import { counter } from "./counter.ds";

counter = 1;

=== checked ===
import { counter } from "./counter.ds";

counter = 1;
/// @type.node source="counter = 1" type=1
/// @type.node source=counter type=int32
/// @resolution.name source=counter target=counter.counter
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=2 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error code=EC204 message="cannot assign to 'counter': imported bindings cannot be assigned"
/// @diagnostic.label line=4 column=1 source="counter = 1;"
"#,
    );
}
