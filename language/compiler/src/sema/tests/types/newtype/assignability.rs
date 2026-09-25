use crate::tests::{DirRows, TestSession};

/// Assigning a backing value to a newtype reports a diagnostic.
#[test]
fn test_backing_value_assigned_to_newtype_reports_error() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

const id: UserId = 42;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int64;

const id: UserId = 42;

=== dir ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64 constructors=[(int64) => UserId]

const id: UserId = 42;
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.pattern source=id kind=binding target=id
/// @resolution.name source=UserId target=UserId
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '42' is not assignable to type 'UserId'"
/// @diagnostic.label line=4 column=20 span="42" line_source="const id: UserId = 42;"
/// @diagnostic.related line=4 column=11 span="UserId" line_source="const id: UserId = 42;" message="expected due to this annotation"
"#,
    );
}

/// Assigning a newtype to its backing type reports a diagnostic.
#[test]
fn test_newtype_assigned_to_backing_type_reports_error() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

const raw: int64 = UserId(42);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int64;

const raw: int64 = UserId(42);

=== dir ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64 constructors=[(int64) => UserId]

const raw: int64 = UserId(42);
/// @type.symbol symbol=raw source=raw type=int64
/// @resolution.pattern source=raw kind=binding target=raw
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId backing=int64
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'UserId' is not assignable to type 'int64'"
/// @diagnostic.label line=4 column=20 span="UserId(42)" line_source="const raw: int64 = UserId(42);"
/// @diagnostic.related line=4 column=12 span="int64" line_source="const raw: int64 = UserId(42);" message="expected due to this annotation"
"#,
    );
}

/// An explicit cast converts a newtype to its backing type.
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int64;

const id: UserId = UserId(42);
const raw: int64 = id as int64;
raw satisfies int64;

=== dir ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64 constructors=[(int64) => UserId]

const id = UserId(42);
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.pattern source=id kind=binding target=id
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId backing=int64

const raw = id as int64;
/// @type.symbol symbol=raw source=raw type=int64
/// @resolution.pattern source=raw kind=binding target=raw
/// @resolution.name source=id target=id
/// @resolution.place source=id placement="local" lifetime="static" access="immutable"
/// @resolution.access source=id root=id

raw satisfies int64;
/// @resolution.name source=raw target=raw
/// @resolution.place source=raw placement="local" lifetime="static" access="immutable"
/// @resolution.access source=raw root=raw
"#,
    );
}

/// A newtype assigns to another binding of the same declaration.
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int64;

const source: UserId = UserId(42);
const target: UserId = source;
target satisfies UserId;

=== dir ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64 constructors=[(int64) => UserId]

const source = UserId(42);
/// @type.symbol symbol=source source=source type=UserId
/// @resolution.pattern source=source kind=binding target=source
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId backing=int64

const target: UserId = source;
/// @type.symbol symbol=target source=target type=UserId
/// @resolution.pattern source=target kind=binding target=target
/// @resolution.name source=UserId target=UserId
/// @resolution.name source=source target=source
/// @resolution.place source=source placement="local" lifetime="static" access="immutable"
/// @resolution.access source=source root=source

target satisfies UserId;
/// @resolution.name source=target target=target
/// @resolution.place source=target placement="local" lifetime="static" access="immutable"
/// @resolution.access source=target root=target
/// @resolution.name source=UserId target=UserId
"#,
    );
}

/// Assigning between two newtypes over one backing type reports a diagnostic.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int64;
newtype OrderId = int64;

const user: UserId = UserId(42);
const order: OrderId = user;

=== dir ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64 constructors=[(int64) => UserId]

newtype OrderId = int64;
/// @type.symbol symbol=OrderId source="newtype OrderId = int64" type=OrderId
/// @definition.newtype symbol=OrderId source="newtype OrderId = int64" backing=int64 constructors=[(int64) => OrderId]

const user = UserId(42);
/// @type.symbol symbol=user source=user type=UserId
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId backing=int64

const order: OrderId = user;
/// @type.symbol symbol=order source=order type=OrderId
/// @resolution.pattern source=order kind=binding target=order
/// @resolution.name source=OrderId target=OrderId
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'UserId' is not assignable to type 'OrderId'"
/// @diagnostic.label line=6 column=24 span="user" line_source="const order: OrderId = user;"
/// @diagnostic.related line=6 column=14 span="OrderId" line_source="const order: OrderId = user;" message="expected due to this annotation"
"#,
    );
}

/// Assigning between two imported newtypes of the same name reports a diagnostic.
#[test]
fn test_imported_newtypes_with_same_name_report_error() {
    let compiler = TestSession::builder()
        .module(
            "left.tspp",
            r#"
export newtype UserId = int64;
"#,
        )
        .module(
            "right.tspp",
            r#"
export newtype UserId = int64;
"#,
        )
        .module(
            "main.tspp",
            r#"
import { UserId as LeftUserId } from "./left.tspp";
import { UserId as RightUserId } from "./right.tspp";

const id: LeftUserId = RightUserId(42);
"#,
        )
        .build();

    compiler.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { UserId as LeftUserId } from "./left.tspp";
import { UserId as RightUserId } from "./right.tspp";

const id: UserId = RightUserId(42);

=== dir ===
import { UserId as LeftUserId } from "./left.tspp";
import { UserId as RightUserId } from "./right.tspp";

const id: LeftUserId = RightUserId(42);
/// @type.symbol symbol=id source=id type=left.UserId
/// @resolution.pattern source=id kind=binding target=id
/// @resolution.name source=LeftUserId target=left.UserId
/// @resolution.name source=RightUserId target=right.UserId
/// @resolution.construct source=RightUserId(42) parameters=(int64) arguments=(provided(42) as int64) return=right.UserId kind=newtype target=right.UserId backing=int64
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'right.UserId' is not assignable to type 'left.UserId'"
/// @diagnostic.label line=5 column=24 span="RightUserId(42)" line_source="const id: LeftUserId = RightUserId(42);"
/// @diagnostic.related line=5 column=11 span="LeftUserId" line_source="const id: LeftUserId = RightUserId(42);" message="expected due to this annotation"
"#,
    );
}

/// Assigning an object literal to a newtype over an object reports a diagnostic.
#[test]
fn test_object_literal_assigned_to_newtype_reports_error() {
    let session = TestSession::single(
        r#"
newtype Config = { debug: boolean };

const config: Config = { debug: true };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Config = { debug: boolean };

const config: Config = { debug: true };

=== dir ===
newtype Config = { debug: boolean };
/// @type.symbol symbol=Config source="newtype Config = { debug: boolean }" type=Config
/// @definition.newtype symbol=Config source="newtype Config = { debug: boolean }" backing={ debug: boolean } constructors=[({ debug: boolean }) => Config]
/// @type.symbol symbol=Config.debug source="debug: boolean" type=boolean

const config: Config = { debug: true };
/// @type.symbol symbol=config source=config type=Config
/// @resolution.pattern source=config kind=binding target=config
/// @resolution.name source=Config target=Config
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ debug: boolean }' is not assignable to type 'Config'"
/// @diagnostic.label line=4 column=24 span="{ debug: true }" line_source="const config: Config = { debug: true };"
/// @diagnostic.related line=4 column=15 span="Config" line_source="const config: Config = { debug: true };" message="expected due to this annotation"
"#,
    );
}
