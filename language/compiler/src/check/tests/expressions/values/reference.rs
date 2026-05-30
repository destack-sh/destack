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
        DirRows::checked()
            .with_reference_types()
            .with_check_stats(),
        r#"
const value = 1;
/// @type.symbol symbol=value type=1
/// @type.node source=1 type=1

const copy = value;
/// @type.symbol symbol=copy type=1
/// @type.node source=value type=1
/// @resolution.name source=value target=value
/// @check.stats.solve variables=0 definitions=0 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
/// @check.stats.output total=5 type_nodes=3 type_symbols=2 static_nodes=0 static_symbols=0
"#,
    );
}
