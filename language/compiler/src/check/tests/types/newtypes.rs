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
        DirRows::checked().with_reference_types(),
        r#"
newtype UserId = int64;
/// @type.symbol symbol=UserId type=UserId

const id = UserId(42);
/// @type.symbol symbol=id type=UserId
/// @type.node source=UserId type=UserId
/// @type.node source=UserId(42) type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.call source=UserId(42) parameters=[int64] return=UserId kind=construct target=UserId
/// @type.node source=42 type=int64
"#,
    );
}

#[test]
fn test_check_records_tuple_newtype_constructor_resolution() {
    let session = TestSession::single(
        r#"
newtype Pair = (int32, string);

const pair = Pair(1, "x");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
newtype Pair = (int32, string);
/// @type.symbol symbol=Pair type=Pair

const pair = Pair(1, "x");
/// @type.symbol symbol=pair type=Pair
/// @type.node source="Pair(1, \"x\")" type=Pair
/// @type.node source=Pair type=Pair
/// @resolution.name source=Pair target=Pair
/// @resolution.call source="Pair(1, \"x\")" parameters=[int32, string] return=Pair kind=construct target=Pair
/// @type.node source=1 type=int32
/// @type.node source="\"x\"" type=string
"#,
    );
}

#[test]
fn test_check_records_object_newtype_constructor_resolution() {
    let session = TestSession::single(
        r#"
newtype Config = { debug: boolean };

const config = Config({ debug: true });
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
newtype Config = { debug: boolean };
/// @type.symbol symbol=Config type=Config
/// @type.symbol symbol=Config.debug type=boolean

const config = Config({ debug: true });
/// @type.symbol symbol=config type=Config
/// @type.node source="Config({ debug: true })" type=Config
/// @type.node source=Config type=Config
/// @resolution.name source=Config target=Config
/// @resolution.call source="Config({ debug: true })" parameters=[{ debug: boolean }] return=Config kind=construct target=Config
/// @type.node source="{ debug: true }" type={ debug: boolean }
/// @type.node source=true type=boolean
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
        DirRows::checked().with_reference_types(),
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

#[test]
fn test_check_reports_object_literals_assigned_to_newtypes() {
    let session = TestSession::single(
        r#"
newtype Config = { debug: boolean };

const config: Config = { debug: true };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
newtype Config = { debug: boolean };
/// @type.symbol symbol=Config type=Config
/// @type.symbol symbol=Config.debug type=boolean

const config: Config = { debug: true };
/// @type.symbol symbol=config type=Config
/// @type.node source="{ debug: true }" type={ debug: boolean }
/// @type.node source=true type=boolean
"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=4 column=24 source="const config: Config = { debug: true };"
"#,
    );
}
