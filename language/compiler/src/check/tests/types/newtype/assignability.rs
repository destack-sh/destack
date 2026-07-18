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
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int64;

const id: UserId = 42;

=== checked ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64

const id: UserId = 42;
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.name source=UserId target=UserId
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '42' is not assignable to type 'UserId'"
/// @diagnostic.label line=4 column=20 span="42" line_source="const id: UserId = 42;"
"#,
    );
}

#[test]
fn test_newtype_assigned_to_backing_type_reports_error() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

const raw: int64 = UserId(42);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int64;

const raw: int64 = UserId(42);

=== checked ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64

const raw: int64 = UserId(42);
/// @type.symbol symbol=raw source=raw type=int64
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId backing=int64
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'UserId' is not assignable to type 'int64'"
/// @diagnostic.label line=4 column=20 span="UserId(42)" line_source="const raw: int64 = UserId(42);"
"#,
    );
}

#[test]
fn test_newtype_cast_to_backing_type_is_explicit() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

const id = UserId(42);
const raw = id as int64;
raw satisfies int64;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int64;

const id: UserId = UserId(42);
const raw: int64 = id as int64;
raw satisfies int64;

=== checked ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64

const id = UserId(42);
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId backing=int64

const raw = id as int64;
/// @type.symbol symbol=raw source=raw type=int64
/// @resolution.name source=id target=id

raw satisfies int64;
/// @resolution.name source=raw target=raw
"#,
    );
}

#[test]
fn test_newtype_assigns_to_same_declaration() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

const source = UserId(42);
const target: UserId = source;
target satisfies UserId;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int64;

const source: UserId = UserId(42);
const target: UserId = source;
target satisfies UserId;

=== checked ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64

const source = UserId(42);
/// @type.symbol symbol=source source=source type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId backing=int64

const target: UserId = source;
/// @type.symbol symbol=target source=target type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.name source=source target=source

target satisfies UserId;
/// @resolution.name source=target target=target
/// @resolution.name source=UserId target=UserId
"#,
    );
}

#[test]
fn test_distinct_newtypes_with_same_backing_report_error() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;
newtype OrderId = int64;

const user = UserId(42);
const order: OrderId = user;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int64;
newtype OrderId = int64;

const user: UserId = UserId(42);
const order: OrderId = user;

=== checked ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64

newtype OrderId = int64;
/// @type.symbol symbol=OrderId source="newtype OrderId = int64" type=OrderId
/// @definition.newtype symbol=OrderId source="newtype OrderId = int64" backing=int64

const user = UserId(42);
/// @type.symbol symbol=user source=user type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId backing=int64

const order: OrderId = user;
/// @type.symbol symbol=order source=order type=OrderId
/// @resolution.name source=OrderId target=OrderId
/// @resolution.name source=user target=user
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'UserId' is not assignable to type 'OrderId'"
/// @diagnostic.label line=6 column=24 span="user" line_source="const order: OrderId = user;"
"#,
    );
}

#[test]
fn test_imported_newtypes_with_same_name_report_error() {
    let compiler = TestSession::builder()
        .module(
            "left.ds",
            r#"
export newtype UserId = int64;
"#,
        )
        .module(
            "right.ds",
            r#"
export newtype UserId = int64;
"#,
        )
        .module(
            "main.ds",
            r#"
import { UserId as LeftUserId } from "./left.ds";
import { UserId as RightUserId } from "./right.ds";

const id: LeftUserId = RightUserId(42);
"#,
        )
        .build();

    compiler.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { UserId as LeftUserId } from "./left.ds";
import { UserId as RightUserId } from "./right.ds";

const id: UserId = RightUserId(42);

=== checked ===
import { UserId as LeftUserId } from "./left.ds";
import { UserId as RightUserId } from "./right.ds";

const id: LeftUserId = RightUserId(42);
/// @type.symbol symbol=id source=id type=left.UserId
/// @resolution.name source=LeftUserId target=left.UserId
/// @resolution.name source=RightUserId target=right.UserId
/// @resolution.construct source=RightUserId(42) parameters=(int64) arguments=(provided(42) as int64) return=right.UserId kind=newtype target=right.UserId backing=int64
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'right.UserId' is not assignable to type 'left.UserId'"
/// @diagnostic.label line=5 column=24 span="RightUserId(42)" line_source="const id: LeftUserId = RightUserId(42);"
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
        DirRows::checked(),
        r#"
=== annotated ===
newtype Config = { debug: boolean };

const config: Config = { debug: true };

=== checked ===
newtype Config = { debug: boolean };
/// @type.symbol symbol=Config source="newtype Config = { debug: boolean }" type=Config
/// @definition.newtype symbol=Config source="newtype Config = { debug: boolean }" backing={ debug: boolean }

const config: Config = { debug: true };
/// @type.symbol symbol=config source=config type=Config
/// @resolution.name source=Config target=Config
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '{ debug: true }' is not assignable to type 'Config'"
/// @diagnostic.label line=4 column=24 span="{ debug: true }" line_source="const config: Config = { debug: true };"
"#,
    );
}
