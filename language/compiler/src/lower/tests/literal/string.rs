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
        "main.ds",
        "test.main.greet",
        r#"
@languageItem("string.String")
type String;

function test.main.greet(): ref<String, managed, mutable, local> {
entry:
    v0: ref<String, managed, mutable, local> = global.address string.0
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
        "main.ds",
        "test.main.pair",
        r#"
@languageItem("string.String")
type String;

function test.main.pair(): ref<String, managed, mutable, local> {
    local l0: ref<String, managed, mutable, local>

entry:
    v0: ref<String, managed, mutable, local> = global.address string.0
    local.set l0, v0
    v1: ref<String, managed, mutable, local> = global.address string.0
    return v1
}
"#,
    );
}

#[test]
fn test_lower_an_interpolated_template_through_its_join() {
    let session = TestSession::single(
        r#"
import { MaybeOwned } from "destack:memory";
import { Display } from "destack:ops";

struct Point {
    x: int32;
}

extension of Point implements Display {
    display(&readonly this): MaybeOwned<string> {
        return MaybeOwned.borrowed("point");
    }
}

function label(point: Point): string {
    return `at ${point}`;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.Point.Display.display", r#"
@copy
type test.main.Point {
    x: int32;
}

@languageItem("string.String")
type String;

@copy
@languageItem("memory.Cow")
type Cow<'a, T>;

function test.main.Point.Display.display<'a>(v0: ref<test.main.Point, borrowed, 'a, readonly, local>): Cow<'a & local, ref<String, managed, mutable, local>> {
    local l0: ref<test.main.Point, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Point, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<String, managed, mutable, local> = global.address string.0
    v2: ref<String, borrowed, 'a, readonly, local> = cast.bit v1 -> ref<String, borrowed, 'a, readonly, local>
    v3: Cow<'a & local, ref<String, managed, mutable, local>> = call Cow.borrowed<ref<String, managed, mutable, local>>(v2): <'a>(ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly, local>) => Cow<'a & local, ref<String, managed, mutable, local>>
    return v3
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.label", r#"
@copy
type test.main.Point {
    x: int32;
}

@languageItem("string.String")
type String;

@copy
@languageItem("memory.Cow")
type Cow<'a, T>;

function test.main.label(v0: test.main.Point): ref<String, managed, mutable, local> {
    local l0: test.main.Point
    local l1: test.main.Point, readonly
    local l2: [ref<String, managed, mutable, local>; 2], readonly
    local l3: [Cow<'l1 & local, ref<String, managed, mutable, local>>; 1], readonly

entry(v0: test.main.Point):
    local.set l0, v0
    v1: test.main.Point = local.get l0
    local.set l1, v1
    v2: ref<test.main.Point, borrowed, 'frame, readonly, local> = local.address l1
    v3: Cow<'frame & local, ref<String, managed, mutable, local>> = call test.main.Point.Display.display(v2): <'a>(ref<test.main.Point, borrowed, 'a, readonly, local>) => Cow<'a & local, ref<String, managed, mutable, local>>
    v4: ref<String, managed, mutable, local> = global.address string.1
    v5: ref<String, managed, mutable, local> = global.address string.2
    v6: [ref<String, managed, mutable, local>; 2] = aggregate (v4, v5)
    local.set l2, v6
    v7: ref<[ref<String, managed, mutable, local>; 2], borrowed, 'frame, readonly, frame> = local.address l2
    v8: uint64 = 0
    v9: usize = 2
    v10: slice<ref<String, managed, mutable, local>, borrowed, 'frame, readonly, frame> = slice.view v7, v8, v9
    v11: slice<ref<String, managed, mutable, local>, borrowed, 'l0, readonly, local> = cast.bit v10 -> slice<ref<String, managed, mutable, local>, borrowed, 'l0, readonly, local>
    v12: [Cow<'l1 & local, ref<String, managed, mutable, local>>; 1] = aggregate (v3)
    local.set l3, v12
    v13: ref<[Cow<'l1 & local, ref<String, managed, mutable, local>>; 1], borrowed, 'frame, readonly, frame> = local.address l3
    v14: uint64 = 0
    v15: usize = 1
    v16: slice<Cow<'l1 & local, ref<String, managed, mutable, local>>, borrowed, 'frame, readonly, frame> = slice.view v13, v14, v15
    v17: slice<Cow<'l1 & local, ref<String, managed, mutable, local>>, borrowed, 'l2, readonly, local> = cast.bit v16 -> slice<Cow<'l1 & local, ref<String, managed, mutable, local>>, borrowed, 'l2, readonly, local>
    v18: String = call stringFromTemplate(v11, v17): <'a, 'b, 'c>(slice<ref<String, managed, mutable, local>, borrowed, 'a, readonly, local>, slice<Cow<'b & local, ref<String, managed, mutable, local>>, borrowed, 'c, readonly, local>) => String
    v19: ref<String, managed, mutable, local> = new.complete v18
    return v19
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#);
}
