use crate::tests::{DirRows, TestSession};

#[test]
fn test_backing_value_assigned_to_newtype_reports_error() {
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
fn test_object_literal_assigned_to_newtype_reports_error() {
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
