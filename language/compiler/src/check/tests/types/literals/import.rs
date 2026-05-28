use crate::tests::{DirRows, TestSession};

#[test]
fn test_exported_const_literal_keeps_precision_across_import() {
    let session = TestSession::new()
        .module(
            "values.ds",
            r#"
export const version = 1;
"#,
        )
        .module(
            "main.ds",
            r#"
import { version } from "./values.ds";

const copy = version;
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["values.ds", "main.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== values.ds ===
export const version = 1;
/// @type.symbol symbol=version type=1
/// @type.node source=1 type=1

=== main.ds ===
import { version } from "./values.ds";
/// @type.symbol symbol=version type=1

const copy = version;
/// @type.symbol symbol=copy type=1
/// @type.node source=version type=1
/// @resolution.name source=version target=values.version
"#,
    );
}

#[test]
fn test_exported_let_literal_widens_across_import() {
    let session = TestSession::new()
        .module(
            "values.ds",
            r#"
export let counter = 1;
"#,
        )
        .module(
            "main.ds",
            r#"
import { counter } from "./values.ds";

const copy = counter;
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["values.ds", "main.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== values.ds ===
export let counter = 1;
/// @type.symbol symbol=counter type=int32
/// @type.node source=1 type=int32

=== main.ds ===
import { counter } from "./values.ds";
/// @type.symbol symbol=counter type=int32

const copy = counter;
/// @type.symbol symbol=copy type=int32
/// @type.node source=counter type=int32
/// @resolution.name source=counter target=values.counter
"#,
    );
}
