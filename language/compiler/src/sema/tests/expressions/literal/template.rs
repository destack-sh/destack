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
/// @resolution.template source="`hello ${name}`" spans=[Display.display(parameters=(), arguments=(), return=^string, regions=("managed" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
/// @generic.instantiation id="Display.display<string, \"managed\" & \"local\">" template=Display.display arguments=("managed" & "local")
/// @generic.instantiation id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
/// @generic.instance id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
/// @type.node source=name type="Ada"
/// @resolution.name source=name target=name
/// @resolution.place source=name placement="local" lifetime="static" access="immutable"
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
import { Display } from "destack:ops";

struct Point {
    x: int32;
}

extension of Point implements Display {
    display(&immutable this): ^string {
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
import { Display } from "destack:ops";

struct Point {
    x: int32;
}

extension of Point implements Display {
    display(&immutable this): ^string {
        return todo("Point.display" as string | undefined);
    }
}

function label(point: Point): string {
    return `point ${point}`;
}

=== dir ===
import { todo } from "destack:error";
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
/// @definition.method symbol=display slot=display type=<display.'a>(this: &display.'a immutable Point) => ^string
/// @definition.conformance symbol=<module>#2 member=display requirement=Display.display
/// @resolution.name source=Point target=Point
/// @resolution.name source=Display target=Display

    display(&immutable this): ^string {
    /// @generic.template symbol=display parent=template#0 parameters=('a)
    /// @type.symbol symbol=display type=<display.'a>(this: &display.'a immutable Point) => ^string
    /// @type.symbol symbol=display.this source="&immutable this" type=&display.'a immutable Point

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
    /// @resolution.template source="`point ${point}`" spans=[display(parameters=(), arguments=(), return=^string, regions=("frame" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
    /// @generic.instantiation id="display<\"frame\" & \"local\">" template=display arguments=("frame" & "local")
    /// @generic.instantiation id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
    /// @generic.instance id="display<\"bound0\" & \"local\">" template=display arguments=("bound0" & "local")
    /// @generic.instance id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
    /// @type.node source=point type=Point
    /// @resolution.name source=point target=label.point
    /// @resolution.place source=point placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=point root=label.point

}
"#,
    );
}

/// Render a struct template span through its derived display.
#[test]
fn test_render_a_struct_template_span_through_its_derived_display() {
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
    /// @resolution.template source="`point ${point}`" spans=[Display.display(parameters=(), arguments=(), return=^string, regions=("frame" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
    /// @generic.instantiation id="Display.display<Point, \"frame\" & \"local\">" template=Display.display arguments=("frame" & "local")
    /// @generic.instantiation id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
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
/// @resolution.template source=`x${count}` spans=[Display.display(parameters=(), arguments=(), return=^string, regions=("static" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
/// @generic.instantiation id="Display.display<float64, \"static\" & \"local\">" template=Display.display arguments=("static" & "local")
/// @generic.instantiation id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
/// @type.node source=count type=float64
/// @resolution.name source=count target=count
/// @resolution.place source=count placement="local" lifetime="static" access="immutable"
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
/// @resolution.template source=`x${count}` spans=[Display.display(parameters=(), arguments=(), return=^string, regions=("static" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
/// @generic.instantiation id="Display.display<float64, \"static\" & \"local\">" template=Display.display arguments=("static" & "local")
/// @generic.instantiation id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
/// @type.node source=count type=float64
/// @resolution.name source=count target=count
/// @resolution.place source=count placement="local" lifetime="static" access="immutable"
/// @resolution.access source=count root=count

const asserted = `x${count}` as const;
/// @type.symbol symbol=asserted source=asserted type=`x${float64}`
/// @resolution.pattern source=asserted kind=binding target=asserted
/// @type.node source="`x${count}` as const" type=`x${float64}`
/// @type.node source=`x${count}` type=`x${float64}`
/// @resolution.template source=`x${count}` spans=[Display.display(parameters=(), arguments=(), return=^string, regions=("static" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
/// @type.node source=count type=float64
/// @resolution.name source=count target=count
/// @resolution.place source=count placement="local" lifetime="static" access="immutable"
/// @resolution.access source=count root=count
"#,
        r#"
"#,
    );
}

/// The module namespace carries the metadata its interface declares.
#[test]
fn test_read_import_meta_through_its_interface() {
    let session = TestSession::single(
        r#"
const location: string = import.meta.url;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const location: string = import.meta.url;

=== dir ===
const location: string = import.meta.url;
/// @type.symbol symbol=location source=location type=string
/// @resolution.pattern source=location kind=binding target=location
/// @resolution.member source=import.meta.url receiver=ImportMeta type=string kind=field target_receiver=ImportMeta dispatch=dynamic constraint=ImportMeta key=url target=ImportMeta.url target_type=string
/// @resolution.place source=import.meta.url placement="local" lifetime="managed" access="mutable"
"#,
        r#"
"#,
    );
}
