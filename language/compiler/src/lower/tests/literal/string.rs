use crate::tests::TestSession;

/// Read a string literal from the constant String object declared for its content.
#[test]
fn test_lower_string_literal_to_constant_object_read() {
    let session = TestSession::single(
        r#"
function greet(): string {
    return "hello 😀";
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.greet",
        r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.greet(): ref<String, managed, mutable, local> {
entry:
    v0: ref<String, managed, mutable, local> = address @string.0
    return v0
}
"#,
    );
}

/// Read repeated string literals of one content from a single constant String object.
#[test]
fn test_intern_repeated_string_literals_into_one_object() {
    let session = TestSession::single(
        r#"
function pair(): string {
    let first = "hi";
    return "hi";
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.pair",
        r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.pair(): ref<String, managed, mutable, local> {
    local l0: ref<String, managed, mutable, local>

entry:
    v0: ref<String, managed, mutable, local> = address @string.0
    store l0, v0
    v1: ref<String, managed, mutable, local> = address @string.0
    return v1
}
"#,
    );
}

#[test]
fn test_lower_an_interpolated_template_through_its_join() {
    let session = TestSession::single(
        r#"
import { Display } from "tspp:ops";

struct Point {
    x: int32;
}

extension of Point implements Display {
    display(&immutable this): ^string {
        return "point";
    }
}

function label(point: Point): string {
    return `at ${point}`;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Point.Display.display", r#"
type test.main.Point {
    x: int32;
}

@nocopy
@languageItem("string.String")
type String;

function test.main.Point.Display.display<'a>(v0: ref<test.main.Point, borrowed, 'a, immutable>): String {
    local l0: ref<test.main.Point, borrowed, 'a, immutable>

entry(v0: ref<test.main.Point, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<String, managed, mutable, local> = address @string.0
    v2: ref<String, borrowed, 'managed, immutable> = cast.bit v1 -> ref<String, borrowed, 'managed, immutable>
    v3: String = call String.Clone.clone(v2): (ref<String, borrowed, 'managed, immutable>) => String
    return v3
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.label", r#"
type test.main.Point {
    x: int32;
}

@nocopy
@languageItem("string.String")
type String;

function test.main.label(v0: test.main.Point): ref<String, managed, mutable, local> {
    local l0: test.main.Point
    local l1: test.main.Point
    local l2: [ref<String, managed, mutable, local>; 2], readonly
    local l3: [ref<String, managed, mutable, local>; 1], readonly

entry(v0: test.main.Point):
    store l0, v0
    v1: test.main.Point = load l0
    store l1, v1
    v2: ref<test.main.Point, borrowed, 'frame, immutable> = address l1
    v3: String = call test.main.Point.Display.display(v2): (ref<test.main.Point, borrowed, 'frame, immutable>) => String
    v4: ref<String, managed, mutable, local> = address @string.1
    v5: ref<String, managed, mutable, local> = address @string.2
    v6: [ref<String, managed, mutable, local>; 2] = aggregate (v4, v5)
    store l2, v6
    v7: usize = 0
    v8: usize = 2
    v9: slice<ref<String, managed, mutable, local>, borrowed, 'frame, readonly> = address l2[v7; v8]
    v10: slice<ref<String, managed, mutable, local>, borrowed, 'l0, readonly> = address (*v9)
    v11: ref<String, managed, mutable, local> = new.complete v3
    v12: [ref<String, managed, mutable, local>; 1] = aggregate (v11)
    store l3, v12
    v13: usize = 0
    v14: usize = 1
    v15: slice<ref<String, managed, mutable, local>, borrowed, 'frame, readonly> = address l3[v13; v14]
    v16: slice<ref<String, managed, mutable, local>, borrowed, 'l1, readonly> = address (*v15)
    v17: String = call stringFromTemplate(v10, v16): <'a, 'b>(slice<ref<String, managed, mutable, local>, borrowed, 'a, readonly>, slice<ref<String, managed, mutable, local>, borrowed, 'b, readonly>) => String
    v18: ref<String, managed, mutable, local> = new.complete v17
    return v18
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#);
}

/// A string literal under an owned expectation clones its constant into owned storage.
#[test]
fn test_lower_an_owned_string_literal_through_its_clone() {
    let session = TestSession::single(
        r#"
function name(): ^string {
    return "text";
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.name",
        r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.name(): String {
entry:
    v0: ref<String, managed, mutable, local> = address @string.0
    v1: ref<String, borrowed, 'managed, immutable> = cast.bit v0 -> ref<String, borrowed, 'managed, immutable>
    v2: String = call String.Clone.clone(v1): (ref<String, borrowed, 'managed, immutable>) => String
    return v2
}
"#,
    );
}
