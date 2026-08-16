use crate::tests::{DirRows, TestSession};

#[test]
fn test_materialize_closes_a_called_function_instance() {
    let session = TestSession::single(
        r#"
function pick<T>(value: T): T {
    return value;
}

const chosen = pick(1.5);
"#,
    );

    session.assert_dir_materialized(
        "main.ds",
        DirRows::checked(),
        r#"
function pick<T>(value: T): T {
    return value;
}

const chosen = pick(1.5);
/// @instance.closed id=pick<1.5> template=pick arguments=(1.5)
"#,
    );
}

#[test]
fn test_materialize_deduplicates_repeated_instance_arguments() {
    let session = TestSession::single(
        r#"
function pick<T>(value: T): T {
    return value;
}

const first = pick(1);
const second = pick(2);
const other = pick("text");
"#,
    );

    session.assert_dir_materialized(
        "main.ds",
        DirRows::checked(),
        r#"
function pick<T>(value: T): T {
    return value;
}

const first = pick(1);
/// @instance.closed id=pick<1> template=pick arguments=(1)

const second = pick(2);
/// @instance.closed id=pick<2> template=pick arguments=(2)

const other = pick("text");
/// @instance.closed id="pick<\"text\">" template=pick arguments=("text")
"#,
    );
}

#[test]
fn test_materialize_closes_transitive_instances_through_a_template_body() {
    let session = TestSession::single(
        r#"
function inner<T>(value: T): T {
    return value;
}

function outer<T>(value: T): T {
    return inner(value);
}

const chosen = outer(true);
"#,
    );

    session.assert_dir_materialized(
        "main.ds",
        DirRows::checked(),
        r#"
function inner<T>(value: T): T {
    return value;
}

function outer<T>(value: T): T {
    return inner(value);
}

const chosen = outer(true);
/// @instance.closed id=inner<true> template=inner arguments=(true)
/// @instance.closed id=outer<true> template=outer arguments=(true)
"#,
    );
}

#[test]
fn test_materialize_closes_an_imported_template_instance() {
    let session = TestSession::single(
        r#"
function positive(values: int32[]): int32[] {
    return values.map((value) => value + 1);
}
"#,
    );

    session.assert_dir_materialized("main.ds", DirRows::checked(), r#"
function positive(values: int32[]): int32[] {
    return values.map((value) => value + 1);
    /// @instance.closed id="collections.array.map#2<int32, int32>" template=collections.array.map#2 arguments=(int32, int32)

}
"#);
}

#[test]
fn test_materialize_evaluates_a_computed_template_type() {
    let session = TestSession::single(
        r#"
type Choice<T> = T extends string ? int32 : boolean;

declare function choose<T>(): Choice<T>;

function tag<T>(value: T): Choice<T> {
    return choose<T>();
}

const chosen = tag("name");
"#,
    );

    session.assert_dir_materialized("main.ds", DirRows::checked(), r#"
type Choice<T> = T extends string ? int32 : boolean;
/// @definition.type symbol=Choice source="type Choice<T> = T extends string ? int32 : boolean" template=(T#1) value=T#1 extends string ? int32 : boolean

declare function choose<T>(): Choice<T>;

function tag<T>(value: T): Choice<T> {
    return choose<T>();
}

const chosen = tag("name");
/// @instance.type instance=tag<string> source=Choice<T#3> type=int32
/// @instance.closed id=choose<string> template=choose arguments=(string)
/// @instance.closed id=tag<string> template=tag arguments=(string)
"#);
}

#[test]
fn test_materialize_adds_no_rows_to_a_monomorphic_module() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

const origin = Point { x: 0 };
"#,
    );

    // a module without generics closes no instances, so the tail renders bare
    session.assert_dir_materialized(
        "main.ds",
        DirRows::checked(),
        r#"
struct Point {
    x: int32;
}

const origin = Point { x: 0 };
"#,
    );
}
