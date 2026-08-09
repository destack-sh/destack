use crate::tests::{DirRows, TestSession};

#[test]
fn test_template_literal_has_string_type() {
    let session = TestSession::single(
        r#"
const name = "Ada";
const greeting = `hello ${name}`;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const name: "Ada" = "Ada";
const greeting: string = `hello ${name}`;

=== checked ===
const name = "Ada";
/// @type.symbol symbol=name source=name type="Ada"
/// @resolution.pattern source=name kind=binding target=name
/// @type.node source="\"Ada\"" type="Ada"

const greeting = `hello ${name}`;
/// @type.symbol symbol=greeting source=greeting type=string
/// @resolution.pattern source=greeting kind=binding target=greeting
/// @type.node source="`hello ${name}`" type=string
/// @type.node source=name type="Ada"
/// @resolution.name source=name target=name
/// @resolution.place source=name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=name root=name
"#,
    );
}

#[test]
fn test_template_literal_is_assignable_to_string() {
    let session = TestSession::single(
        r#"
const greeting: string = `hello`;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const greeting: string = `hello`;

=== checked ===
const greeting: string = `hello`;
/// @type.symbol symbol=greeting source=greeting type=string
/// @resolution.pattern source=greeting kind=binding target=greeting
/// @type.node source=`hello` type=string
"#,
    );
}

#[test]
fn test_template_literal_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = `hello`;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: float64 = `hello`;

=== checked ===
const value: number = `hello`;
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=`hello` type=string
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'string' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=23 span="`hello`" line_source="const value: number = `hello`;"
/// @diagnostic.related line=2 column=14 span="number" line_source="const value: number = `hello`;" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_render_template_arguments_through_display() {
    let session = TestSession::single(
        r#"
import { todo } from "destack:error";
import { MaybeOwned } from "destack:memory";
import { Display } from "destack:ops";

struct Point {
    x: int32;
}

extension of Point implements Display {
    display(&readonly this): MaybeOwned<string> {
        return todo("Point.display");
    }
}

function label(point: Point): string {
    return `point ${point}`;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { todo } from "destack:error";
import { MaybeOwned } from "destack:memory";
import { Display } from "destack:ops";

struct Point {
    x: int32;
}

extension of Point implements Display {
    display(&readonly this): MaybeOwned<string> {
        return todo("Point.display" as string | undefined);
    }
}

function label(point: Point): string {
    return `point ${point}`;
}

=== checked ===
import { todo } from "destack:error";
import { MaybeOwned } from "destack:memory";
import { Display } from "destack:ops";

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

extension of Point implements Display {
/// @definition.extension symbol=<module>#2 form=local target=Point
/// @definition.implements symbol=<module>#2 source=Display target=ops.format.Display
/// @definition.method symbol=display slot=display type=<display.'a>(this: &display.'a readonly this) => memory.cow.cow.MaybeOwned<string>
/// @definition.conformance symbol=<module>#2 member=display requirement=ops.format.Display.display
/// @resolution.name source=Point target=Point
/// @resolution.name source=Display target=ops.format.Display

    display(&readonly this): MaybeOwned<string> {
    /// @generic.template symbol=display parent=template#0 parameters=('a)
    /// @type.symbol symbol=display type=<display.'a>(this: &display.'a readonly this) => memory.cow.cow.MaybeOwned<string>
    /// @type.symbol symbol=display.this source="&readonly this" type=&display.'a readonly this
    /// @resolution.name source=MaybeOwned target=memory.cow.cow.MaybeOwned

        return todo("Point.display");
        /// @type.node source="todo(\"Point.display\")" type=never
        /// @type.node source=todo type=(string | undefined?) => never
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"Point.display\")" parameters=(string | undefined) arguments=(provided("Point.display") as string | undefined) return=never kind=symbol target=error.panic.todo
        /// @type.node source="\"Point.display\"" type="Point.display"

    }
}

function label(point: Point): string {
/// @type.symbol symbol=label type=(Point) => string
/// @type.symbol symbol=label.point source="point: Point" type=Point
/// @resolution.name source=Point target=Point

    return `point ${point}`;
    /// @type.node source="`point ${point}`" type=string
    /// @type.node source=point type=Point
    /// @resolution.name source=point target=label.point
    /// @resolution.place source=point placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=point root=label.point

}

/// @generic.instance id=memory.cow.cow.MaybeOwned<string> template=memory.cow.cow.MaybeOwned arguments=(string)
"#,
    );
}

#[test]
fn test_reject_template_arguments_without_display() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

function label(point: Point): string {
    return `point ${point}`;
}
"#,
    );

    session.assert_dir_checked_diagnostics(
        "main.ds", r#"

"#,
    );
}
