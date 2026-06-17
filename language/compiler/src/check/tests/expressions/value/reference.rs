use crate::tests::{DirRows, TestSession};

#[test]
fn test_local_reference_selects_declared_binding() {
    let session = TestSession::single(
        r#"
const value = 1;
const copy = value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: 1 = 1;
const copy: 1 = value;

=== checked ===
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1

const copy = value;
/// @type.symbol symbol=copy source=copy type=1
/// @type.node source=value type=1
/// @resolution.name source=value target=value

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1
"#,
    );
}
