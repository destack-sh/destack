use crate::tests::{DirRows, TestSession};

#[test]
fn test_exported_const_literal_keeps_precision_across_import() {
    let session = TestSession::builder()
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== values.ds ===

=== annotated ===
export const version: 1 = 1;

=== checked ===
export const version = 1;
/// @type.symbol symbol=version source=version type=1
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0

=== main.ds ===

=== annotated ===
import { version } from "./values.ds";

const copy: 1 = version;

=== checked ===
import { version } from "./values.ds";

const copy = version;
/// @type.symbol symbol=copy source=copy type=1
/// @type.node source=version type=1
/// @resolution.name source=version target=values.version

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_exported_let_literal_widens_across_import() {
    let session = TestSession::builder()
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== values.ds ===

=== annotated ===
export let counter: float64 = 1 as float64;

=== checked ===
export let counter = 1;
/// @type.symbol symbol=counter source=counter type=float64
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0

=== main.ds ===

=== annotated ===
import { counter } from "./values.ds";

const copy: float64 = counter;

=== checked ===
import { counter } from "./values.ds";

const copy = counter;
/// @type.symbol symbol=copy source=copy type=float64
/// @type.node source=counter type=float64
/// @resolution.name source=counter target=values.counter

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1
"#,
    );
}
