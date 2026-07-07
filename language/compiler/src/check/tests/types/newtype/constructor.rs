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
=== annotated ===
newtype UserId = int64;

const id: UserId = UserId(42);

=== checked ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" value=int64

const id = UserId(42);
/// @type.symbol symbol=id source=id type=UserId
/// @type.node source=UserId type=UserId
/// @type.node source=UserId(42) type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId
/// @type.node source=42 type=42
"#,
    );
}

#[test]
fn test_tuple_newtype_call_constructs_nominal_value() {
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
=== annotated ===
newtype Pair = (int32, string);

const pair: Pair = Pair(1, "x");

=== checked ===
newtype Pair = (int32, string);
/// @type.symbol symbol=Pair source="newtype Pair = (int32, string)" type=Pair
/// @definition.newtype symbol=Pair source="newtype Pair = (int32, string)" value=(int32, string)

const pair = Pair(1, "x");
/// @type.symbol symbol=pair source=pair type=Pair
/// @type.node source="Pair(1, \"x\")" type=Pair
/// @type.node source=Pair type=Pair
/// @resolution.name source=Pair target=Pair
/// @resolution.construct source="Pair(1, \"x\")" parameters=(int32, string) arguments=(provided(1) as int32, provided("x") as string) return=Pair kind=newtype target=Pair
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
=== annotated ===
newtype Config = { debug: boolean };

const config: Config = Config({ debug: true });

=== checked ===
newtype Config = { debug: boolean };
/// @type.symbol symbol=Config source="newtype Config = { debug: boolean }" type=Config
/// @definition.newtype symbol=Config source="newtype Config = { debug: boolean }" value={ debug: boolean }

const config = Config({ debug: true });
/// @type.symbol symbol=config source=config type=Config
/// @type.node source="Config({ debug: true })" type=Config
/// @type.node source=Config type=Config
/// @resolution.name source=Config target=Config
/// @resolution.construct source="Config({ debug: true })" parameters=({ debug: boolean }) arguments=(provided({ debug: true }) as { debug: boolean }) return=Config kind=newtype target=Config
/// @type.node source={ debug: true } type={ debug: true }
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_scalar_newtype_inferred_call_constructs_nominal_value() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

const id: UserId = _(42);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype UserId = int64;

const id: UserId = UserId(42);

=== checked ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" value=int64

const id: UserId = _(42);
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.name source=UserId target=UserId
/// @type.node source=_(42) type=UserId
/// @resolution.construct source=_(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId
/// @type.node source=42 type=42
"#,
    );
}

#[test]
fn test_tuple_newtype_inferred_call_constructs_nominal_value() {
    let session = TestSession::single(
        r#"
newtype Point = (int32, int32);

const point: Point = _(1, 2);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Point = (int32, int32);

const point: Point = Point(1, 2);

=== checked ===
newtype Point = (int32, int32);
/// @type.symbol symbol=Point source="newtype Point = (int32, int32)" type=Point
/// @definition.newtype symbol=Point source="newtype Point = (int32, int32)" value=(int32, int32)

const point: Point = _(1, 2);
/// @type.symbol symbol=point source=point type=Point
/// @resolution.name source=Point target=Point
/// @type.node source="_(1, 2)" type=Point
/// @resolution.construct source="_(1, 2)" parameters=(int32, int32) arguments=(provided(1) as int32, provided(2) as int32) return=Point kind=newtype target=Point
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_object_newtype_inferred_call_constructs_nominal_value() {
    let session = TestSession::single(
        r#"
newtype Config = { debug: boolean };

const config: Config = _({ debug: true });
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Config = { debug: boolean };

const config: Config = Config({ debug: true });

=== checked ===
newtype Config = { debug: boolean };
/// @type.symbol symbol=Config source="newtype Config = { debug: boolean }" type=Config
/// @definition.newtype symbol=Config source="newtype Config = { debug: boolean }" value={ debug: boolean }

const config: Config = _({ debug: true });
/// @type.symbol symbol=config source=config type=Config
/// @resolution.name source=Config target=Config
/// @type.node source="_({ debug: true })" type=Config
/// @resolution.construct source="_({ debug: true })" parameters=({ debug: boolean }) arguments=(provided({ debug: true }) as { debug: boolean }) return=Config kind=newtype target=Config
/// @type.node source={ debug: true } type={ debug: true }
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_generic_newtype_inferred_call_uses_expected_arguments() {
    let session = TestSession::single(
        r#"
newtype Box<T> = T;

const value: Box<int32> = _(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Box<T> = T;

const value: Box<int32> = Box<int32>(1);

=== checked ===
newtype Box<T> = T;
/// @generic.template symbol=Box parameters=(T)
/// @type.symbol symbol=Box source="newtype Box<T> = T" type=Box
/// @definition.newtype symbol=Box source="newtype Box<T> = T" template=(T) value=T
/// @type.symbol symbol=Box.T source=T type=T
/// @resolution.name source=T target=Box.T

const value: Box<int32> = _(1);
/// @type.symbol symbol=value source=value type=Box<int32>
/// @resolution.name source=Box target=Box
/// @type.node source=_(1) type=Box<int32>
/// @resolution.construct source=_(1) parameters=(int32) arguments=(provided(1) as int32) return=Box<int32> kind=newtype target=Box instance=Box<int32>
/// @generic.instance source=_(1) id=Box<int32>
/// @type.node source=1 type=1

/// @generic.instance id=Box<int32> template=Box arguments=(int32)
"#,
    );
}

#[test]
fn test_generic_newtype_union_constructor_uses_expected_result() {
    let session = TestSession::single(
        r#"
newtype Result<T, E> = T | E;

function from<T, E>(value: E): Result<T, E> {
    Result(value)
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Result<T, E> = T | E;

function from<T, E>(value: E): Result<T, E> {
    Result<T, E>(value as T | E)
}

=== checked ===
newtype Result<T, E> = T | E;
/// @generic.template symbol=Result parameters=(T#1, E#1)
/// @type.symbol symbol=Result source="newtype Result<T, E> = T | E" type=Result
/// @definition.newtype symbol=Result source="newtype Result<T, E> = T | E" template=(T#1, E#1) value=T#1 | E#1
/// @type.symbol symbol=Result.T source=T type=T#1
/// @type.symbol symbol=Result.E source=E type=E#1
/// @resolution.name source=T target=Result.T
/// @resolution.name source=E target=Result.E

function from<T, E>(value: E): Result<T, E> {
/// @generic.template symbol=from parameters=(T#2, E#2)
/// @type.symbol symbol=from type=<T#2, E#2>(E#2) => Result<T#2, E#2>
/// @type.symbol symbol=from.T source=T type=T#2
/// @type.symbol symbol=from.E source=E type=E#2
/// @type.symbol symbol=from.value source="value: E" type=E#2
/// @resolution.name source=E target=from.E
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=from.T
/// @resolution.name source=E target=from.E

    Result(value)
    /// @type.node source=Result type=Result
    /// @type.node source=Result(value) type=Result<T#2, E#2>
    /// @resolution.name source=Result target=Result
    /// @resolution.construct source=Result(value) parameters=(T#2 | E#2) arguments=(provided(value) as T#2 | E#2) return=Result<T#2, E#2> kind=newtype target=Result instance="Result<T#2, E#2>"
    /// @generic.instance source=Result(value) id="Result<T#2, E#2>"
    /// @type.node source=value type=E#2
    /// @resolution.name source=value target=from.value

}

/// @generic.instance id="Result<T#2, E#2>" template=Result arguments=(T#2, E#2)
"#,
    );
}

#[test]
fn test_inferred_call_without_expected_newtype_reports_error() {
    let session = TestSession::single(
        r#"
const value = _(1);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value = _(1);

=== checked ===
const value = _(1);
/// @type.symbol symbol=value source=value type=<error>
"#,
        r#"
/// @diagnostic.error code=EC100 message="cannot infer a type here"
/// @diagnostic.label line=2 column=15 span="_(1)" line_source="const value = _(1);"
"#,
    );
}

#[test]
fn test_inferred_call_with_non_newtype_target_reports_error() {
    let session = TestSession::single(
        r#"
const value: string = _(1);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value: string = _(1);

=== checked ===
const value: string = _(1);
/// @type.symbol symbol=value source=value type=string
"#,
        r#"
/// @diagnostic.error code=EC323 message="type 'string' cannot be constructed with '_(...)'"
/// @diagnostic.label line=2 column=23 span="_(1)" line_source="const value: string = _(1);"
"#,
    );
}
