use crate::tests::{DirRows, TestSession};

#[test]
fn test_interval_type_stays_compact() {
    let session = TestSession::single(
        r#"
type Count = 0..5;

declare const count: Count;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Count = 0..5;
/// @type.symbol symbol=Count type=0..5

declare const count: Count;
/// @resolution.name source=Count target=Count
/// @type.symbol symbol=count type=0..5
"#,
    );
}
