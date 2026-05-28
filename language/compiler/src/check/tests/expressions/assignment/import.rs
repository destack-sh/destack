use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_binding_assignment_reports_error() {
    let session = TestSession::new()
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
        DirRows::checked().with_reference_types(),
        r#"
import { counter } from "./counter.ds";
/// @type.symbol symbol=counter type=int32

counter = 1;
/// @type.node source="counter = 1" type=int32
/// @type.node source=counter type=int32
/// @resolution.name source=counter target=counter.counter
/// @type.node source=1 type=int32
"#,
        r#"
/// @diagnostic.error code=EC204 message="assignment target is not writable"
/// @diagnostic.label line=4 column=1 source="counter = 1;"
"#,
    );
}
