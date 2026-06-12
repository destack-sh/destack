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
=== annotated ===
type Count = 0..5;

declare const count: Count;

=== checked ===
type Count = 0..5;
/// @type.symbol symbol=Count source="type Count = 0..5" type=0..5
/// @definition.type symbol=Count source="type Count = 0..5" value=0..5

declare const count: Count;
/// @type.symbol symbol=count source=count type=0..5
/// @resolution.name source=Count target=Count
"#,
    );
}
