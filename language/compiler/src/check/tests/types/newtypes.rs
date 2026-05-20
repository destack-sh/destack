use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_newtype_union_layout() {
    let session = TestSession::single(
        r#"
struct Rectangle {}
struct Circle {}

newtype Shape = Rectangle | Circle;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_layout(),
        r#"
struct Rectangle {}
/// @type.symbol symbol=Rectangle type=Rectangle

struct Circle {}
/// @type.symbol symbol=Circle type=Circle

newtype Shape = Rectangle | Circle;
/// @type.symbol symbol=Shape type=Shape
/// @layout.type type=Shape shape=variant
"#,
    );
}

#[test]
fn test_check_records_newtype_constructor_resolution() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

const id = UserId(42);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
newtype UserId = int64;
/// @type.symbol symbol=UserId type=UserId

const id = UserId(42);
/// @resolution.name source=UserId target=UserId
/// @resolution.call source="UserId(42)" parameters=[int64] return=UserId kind=construct target=UserId
/// @type.symbol symbol=id type=UserId
"#,
    );
}

#[test]
fn test_check_reports_backing_values_assigned_to_newtypes() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

const id: UserId = 42;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
newtype UserId = int64;
/// @type.symbol symbol=UserId type=UserId

const id: UserId = 42;
/// @resolution.name source=UserId target=UserId
/// @type.node source=42 type=int64
/// @type.symbol symbol=id type=UserId
"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=4 column=20 source="const id: UserId = 42;"
"#,
    );
}
