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
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" value=int64

const id: UserId = 42;
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.name source=UserId target=UserId
/// @type.node source=42 type=42
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
/// @type.symbol symbol=Config source="newtype Config = { debug: boolean }" type=Config
/// @definition.newtype symbol=Config source="newtype Config = { debug: boolean }" value={ debug: boolean }
/// @type.symbol symbol=Config.debug source="debug: boolean" type=boolean

const config: Config = { debug: true };
/// @type.symbol symbol=config source=config type=Config
/// @resolution.name source=Config target=Config
/// @type.node source="{ debug: true }" type={ debug: true }
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=4 column=24 source="const config: Config = { debug: true };"
"#,
    );
}
