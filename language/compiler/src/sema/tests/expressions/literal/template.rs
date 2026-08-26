use crate::tests::{DirRows, TestSession};

#[test]
fn test_type_a_template_literal_with_interpolation_as_string() {
    let session = TestSession::single(
        r#"
const name = "Ada";
const greeting = `hello ${name}`;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const name: "Ada" = "Ada";
const greeting: string = `hello ${name}`;

=== dir ===
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
fn test_assign_a_template_literal_to_a_string_annotation() {
    let session = TestSession::single(
        r#"
const greeting: string = `hello`;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const greeting: string = `hello`;

=== dir ===
const greeting: string = `hello`;
/// @type.symbol symbol=greeting source=greeting type=string
/// @resolution.pattern source=greeting kind=binding target=greeting
/// @type.node source=`hello` type="hello"
"#,
    );
}

#[test]
fn test_reject_a_template_literal_assigned_to_a_number() {
    let session = TestSession::single(
        r#"
const value: number = `hello`;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: float64 = `hello`;

=== dir ===
const value: number = `hello`;
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=`hello` type="hello"
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"hello\"' is not assignable to type 'float64'"
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

    session.assert_dir(
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

=== dir ===
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
/// @definition.implements symbol=<module>#2 source=Display target=Display
/// @definition.method symbol=display slot=display type=<display.'a, display.P1: Place>(this: &display.'a readonly Point) => MaybeOwned<string>
/// @definition.conformance symbol=<module>#2 member=display requirement=Display.display
/// @resolution.name source=Point target=Point
/// @resolution.name source=Display target=Display

    display(&readonly this): MaybeOwned<string> {
    /// @generic.template symbol=display parent=template#0 parameters=('a, P1: Place)
    /// @type.symbol symbol=display type=<display.'a, display.P1: Place>(this: &display.'a readonly Point) => MaybeOwned<string>
    /// @generic.instance id=MaybeOwned<string> template=MaybeOwned arguments=(string)
    /// @type.symbol symbol=display.this source="&readonly this" type=&display.'a readonly this
    /// @resolution.name source=MaybeOwned target=MaybeOwned
    /// @generic.instance id="CowBorrowed<&'frame readonly string>" template=CowBorrowed arguments=(&'frame readonly string)
    /// @generic.instance id=Cow<string> template=Cow arguments=(string)
    /// @generic.instance id=CowOwned<Owned<string>> template=CowOwned arguments=(Owned<string>)

        return todo("Point.display");
        /// @type.node source="todo(\"Point.display\")" type=never
        /// @type.node source=todo type=(string | undefined?) => never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Point.display\")" parameters=(string | undefined) arguments=(provided("Point.display") as string | undefined) return=never kind=symbol target=todo
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

function label(point: Point): string {
    return `point ${point}`;
}

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

function label(point: Point): string {
/// @type.symbol symbol=label type=(Point) => string
/// @type.symbol symbol=label.point source="point: Point" type=Point
/// @resolution.name source=Point target=Point

    return `point ${point}`;
    /// @resolution.name source=point target=label.point
    /// @resolution.place source=point placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=point root=label.point

}
"#,
        r#"
"#,
    );
}

/// Give a template expression a literal type when it has no spans.
#[test]
fn test_give_a_spanless_template_expression_a_literal_type() {
    let session = TestSession::single(
        r#"
declare const count: number;

const text = `hello`;
const rendered = `x${count}`;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const count: float64;

const text: "hello" = `hello`;
const rendered: string = `x${count}`;

=== dir ===
declare const count: number;
/// @type.symbol symbol=count source=count type=float64
/// @resolution.pattern source=count kind=binding target=count

const text = `hello`;
/// @type.symbol symbol=text source=text type="hello"
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source=`hello` type="hello"

const rendered = `x${count}`;
/// @type.symbol symbol=rendered source=rendered type=string
/// @resolution.pattern source=rendered kind=binding target=rendered
/// @type.node source=`x${count}` type=string
/// @type.node source=count type=float64
/// @resolution.name source=count target=count
/// @resolution.place source=count placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=count root=count
"#,
        r#"
"#,
    );
}

/// Take a template literal type from the context or from a const assertion.
#[test]
fn test_take_a_contextual_template_literal_type() {
    let session = TestSession::single(
        r#"
declare const count: number;

const contextual: `x${number}` = `x${count}`;
const asserted = `x${count}` as const;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const count: float64;

const contextual: `x${float64}` = `x${count}`;
const asserted: `x${float64}` = `x${count}` as const;

=== dir ===
declare const count: number;
/// @type.symbol symbol=count source=count type=float64
/// @resolution.pattern source=count kind=binding target=count

const contextual: `x${number}` = `x${count}`;
/// @type.symbol symbol=contextual source=contextual type=`x${float64}`
/// @resolution.pattern source=contextual kind=binding target=contextual
/// @type.node source=`x${count}` type=`x${float64}`
/// @type.node source=count type=float64
/// @resolution.name source=count target=count
/// @resolution.place source=count placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=count root=count

const asserted = `x${count}` as const;
/// @type.symbol symbol=asserted source=asserted type=`x${float64}`
/// @resolution.pattern source=asserted kind=binding target=asserted
/// @type.node source="`x${count}` as const" type=`x${float64}`
/// @type.node source=`x${count}` type=`x${float64}`
/// @type.node source=count type=float64
/// @resolution.name source=count target=count
/// @resolution.place source=count placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=count root=count
"#,
        r#"
"#,
    );
}
