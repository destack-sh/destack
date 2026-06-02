use crate::tests::{DirRows, TestSession};

#[test]
fn test_scalar_newtype_call_constructs_nominal_value() {
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
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @nominal.newtype symbol=UserId source="newtype UserId = int64"

const id = UserId(42);
/// @type.symbol symbol=id source=id type=UserId
/// @type.node source=UserId type=UserId
/// @type.node source=UserId(42) type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) return=UserId kind=newtype target=UserId
/// @type.node source=42 type=42
"#,
    );
}

#[test]
fn test_tuple_newtype_call_constructs_nominal_value() {
    let session = TestSession::single(
        r#"
newtype Pair = (int32, string);

const pair = Pair((1, "x"));
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
newtype Pair = (int32, string);
/// @type.symbol symbol=Pair source="newtype Pair = (int32, string)" type=Pair
/// @nominal.newtype symbol=Pair source="newtype Pair = (int32, string)"

const pair = Pair((1, "x"));
/// @type.symbol symbol=pair source=pair type=Pair
/// @type.node source="Pair((1, \"x\"))" type=Pair
/// @type.node source=Pair type=Pair
/// @resolution.name source=Pair target=Pair
/// @resolution.construct source="Pair((1, \"x\"))" parameters=((int32, string)) return=Pair kind=newtype target=Pair
/// @type.node source="(1, \"x\")" type=(1, "x")
/// @type.node source=1 type=1
/// @type.node source="\"x\"" type="x"
"#,
    );
}

#[test]
fn test_object_newtype_call_constructs_nominal_value() {
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
/// @type.symbol symbol=Config source="newtype Config = { debug: boolean }" type=Config
/// @nominal.newtype symbol=Config source="newtype Config = { debug: boolean }"
/// @type.symbol symbol=Config.debug source="debug: boolean" type=boolean

const config = Config({ debug: true });
/// @type.symbol symbol=config source=config type=Config
/// @type.node source="Config({ debug: true })" type=Config
/// @type.node source=Config type=Config
/// @resolution.name source=Config target=Config
/// @resolution.construct source="Config({ debug: true })" parameters=({ debug: boolean }) return=Config kind=newtype target=Config
/// @type.node source="{ debug: true }" type={ debug: true }
/// @type.node source=true type=true
"#,
    );
}
