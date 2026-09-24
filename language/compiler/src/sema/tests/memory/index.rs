use crate::tests::{DirRows, TestSession};

#[test]
fn test_select_index_access_from_the_place_context() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

function run(values: int32[], points: Point[], view: &readonly int32[]): int32 {
    const read = values[0];
    values[0] = 1;
    values[1] += 2;
    points[0].x = 3;
    const borrowed: &int32 = &values[0];
    const exclusive: &int32 = &values[1];
    const viewed = view[0];
    view[0] = 4;
    return read + viewed;
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

function run<'a>(values: int32[], points: Point[], view: &'a readonly int32[]): int32 {
    const read: int32 = values[0];
    values[0] = 1;
    values[1] += 2;
    points[0].x = 3;
    const borrowed: &'frame int32 = &values[0];
    const exclusive: &'frame int32 = &values[1];
    const viewed: int32 = view[0];
    view[0] = 4;
    return read + viewed;
}

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

function run(values: int32[], points: Point[], view: &readonly int32[]): int32 {
/// @generic.template symbol=run parameters=('a)
/// @type.symbol symbol=run type=<run.'a>(int32[], Point[], &run.'a readonly int32[]) => int32
/// @type.symbol symbol=run.values source="values: int32[]" type=int32[]
/// @type.symbol symbol=run.points source="points: Point[]" type=Point[]
/// @resolution.name source=Point target=Point
/// @type.symbol symbol=run.view source="view: &readonly int32[]" type=&run.'a readonly int32[]

    const read = values[0];
    /// @type.symbol symbol=run.read source=read type=int32
    /// @resolution.pattern source=read kind=binding target=run.read
    /// @resolution.name source=values target=run.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=run.values
    /// @resolution.subscript source=values[0] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=int32, regions=(\"managed\" & \"local\"))"
    /// @generic.instantiation id="index#2<int32, \"managed\" & \"local\">" template=index#2 arguments=(int32, "managed" & "local")

    values[0] = 1;
    /// @resolution.name source=values target=run.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=run.values
    /// @resolution.pattern.assign source=values[0] kind=place
    /// @resolution.assignment source=values[0] write="indexSet#1(parameters=(isize, int32), arguments=(provided(0) as isize, supplied(0) as int32), return=void, regions=(\"managed\" & \"local\"))" type=int32
    /// @generic.instantiation id="indexSet#1<int32, \"managed\" & \"local\">" template=indexSet#1 arguments=(int32, "managed" & "local")

    values[1] += 2;
    /// @resolution.name source=values target=run.values
    /// @resolution.operator source="values[1] += 2" type=int32 operator="+" kind=builtin operands=[values[1] as int32 families=(integer), 2 as int32 families=(integer)]
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=run.values
    /// @resolution.pattern.assign source=values[1] kind=place
    /// @resolution.assignment source=values[1] read="index#2(parameters=(isize), arguments=(provided(1) as isize), return=int32, regions=(\"managed\" & \"local\"))" write="indexSet#1(parameters=(isize, int32), arguments=(provided(1) as isize, supplied(0) as int32), return=void, regions=(\"managed\" & \"local\"))" type=int32

    points[0].x = 3;
    /// @resolution.name source=points target=run.points
    /// @resolution.place source=points placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=points root=run.points
    /// @resolution.subscript source=points[0] type=Point kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=Point, regions=(\"managed\" & \"local\"))"
    /// @resolution.pattern.assign source=points[0].x kind=place
    /// @resolution.place source=points[0].x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=points[0].x write="receiver=Point, target=field(receiver=Point, target=Point.x, type=int32), type=int32" type=int32
    /// @generic.instantiation id="index#2<Point, \"managed\" & \"local\">" template=index#2 arguments=(Point, "managed" & "local")

    const borrowed: &int32 = &values[0];
    /// @type.symbol symbol=run.borrowed source=borrowed type=&'frame int32
    /// @resolution.pattern source=borrowed kind=binding target=run.borrowed
    /// @resolution.name source=values target=run.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=run.values
    /// @resolution.subscript source=values[0] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=int32, regions=(\"managed\" & \"local\"))"

    const exclusive: &int32 = &values[1];
    /// @type.symbol symbol=run.exclusive source=exclusive type=&'frame int32
    /// @resolution.pattern source=exclusive kind=binding target=run.exclusive
    /// @resolution.name source=values target=run.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=run.values
    /// @resolution.subscript source=values[1] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(1) as isize), return=int32, regions=(\"managed\" & \"local\"))"

    const viewed = view[0];
    /// @type.symbol symbol=run.viewed source=viewed type=int32
    /// @resolution.pattern source=viewed kind=binding target=run.viewed
    /// @resolution.name source=view target=run.view
    /// @resolution.place source=view placement=run.'a lifetime=run.'a access="readonly"
    /// @resolution.access source=view root=run.view
    /// @resolution.subscript source=view[0] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=int32, regions=(run.'a))"
    /// @generic.instantiation id="index#2<int32, run.'a>" template=index#2 arguments=(int32, run.'a)

    view[0] = 4;
    /// @resolution.name source=view target=run.view
    /// @resolution.place source=view placement=run.'a lifetime=run.'a access="readonly"
    /// @resolution.access source=view root=run.view
    /// @resolution.rejected source=view[0]

    return read + viewed;
    /// @resolution.name source=read target=run.read
    /// @resolution.operator source="read + viewed" type=int32 operator="+" kind=builtin operands=[read as int32 families=(integer), viewed as int32 families=(integer)]
    /// @resolution.place source=read placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=read root=run.read
    /// @resolution.name source=viewed target=run.viewed
    /// @resolution.place source=viewed placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=viewed root=run.viewed

}
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '[]' is not defined for '&'a readonly int32[]' and '0'"
/// @diagnostic.label line=14 column=9 span="[" line_source="view[0] = 4;"
"#,
    );
}

#[test]
fn test_copy_elements_between_arrays_by_index() {
    let session = TestSession::single(
        r#"
function copy(source: &readonly int32[], target: &int32[]): void {
    for (let index = 0; index < source.length; index += 1) {
        target[index] = source[index];
    }
}

function copyManaged(target: int32[], source: int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function copy<'a, 'b>(source: &'a readonly int32[], target: &'b int32[]): void {
    for (let index: isize = 0; index < source.length; index += 1) {
        target[index] = source[index];
    }
}

function copyManaged(target: int32[], source: int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
    }
}

=== dir ===
function copy(source: &readonly int32[], target: &int32[]): void {
/// @generic.template symbol=copy parameters=('a, 'b)
/// @type.symbol symbol=copy type=<copy.'a, copy.'b>(&copy.'a readonly int32[], &copy.'b int32[]) => void
/// @type.symbol symbol=copy.source source="source: &readonly int32[]" type=&copy.'a readonly int32[]
/// @type.symbol symbol=copy.target source="target: &int32[]" type=&copy.'b int32[]

    for (let index = 0; index < source.length; index += 1) {
    /// @type.symbol symbol=copy.index source=index type=isize
    /// @resolution.pattern source=index kind=binding target=copy.index
    /// @resolution.name source=index target=copy.index
    /// @resolution.operator source="index < source.length" type=boolean operator="<" kind=builtin operands=[index as isize families=(integer), source.length as isize families=(integer)]
    /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=index root=copy.index
    /// @resolution.name source=source target=copy.source
    /// @resolution.member source=source.length receiver=&copy.'a readonly int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(copy.'a))"
    /// @resolution.place source=source placement=copy.'a lifetime=copy.'a access="readonly"
    /// @resolution.access source=source root=copy.source
    /// @generic.instantiation id="length<int32, copy.'a>" template=length arguments=(int32, copy.'a)
    /// @resolution.name source=index target=copy.index
    /// @resolution.operator source="index += 1" type=isize operator="+" kind=builtin operands=[index as isize families=(integer), 1 as isize families=(integer)]
    /// @resolution.pattern.assign source=index kind=place
    /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=index read=binding(copy.index) write=binding(copy.index) type=isize
    /// @resolution.access source=index root=copy.index

        target[index] = source[index];
        /// @resolution.name source=target target=copy.target
        /// @resolution.place source=target placement=copy.'b lifetime=copy.'b access="mutable"
        /// @resolution.access source=target root=copy.target
        /// @resolution.pattern.assign source=target[index] kind=place
        /// @resolution.assignment source=target[index] write="indexSet#1(parameters=(isize, int32), arguments=(provided(index) as isize, supplied(0) as int32), return=void, regions=(copy.'b))" type=int32
        /// @generic.instantiation id="indexSet#1<int32, copy.'b>" template=indexSet#1 arguments=(int32, copy.'b)
        /// @resolution.name source=index target=copy.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=index root=copy.index
        /// @resolution.name source=source target=copy.source
        /// @resolution.place source=source placement=copy.'a lifetime=copy.'a access="readonly"
        /// @resolution.access source=source root=copy.source
        /// @resolution.subscript source=source[index] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(index) as isize), return=int32, regions=(copy.'a))"
        /// @generic.instantiation id="index#2<int32, copy.'a>" template=index#2 arguments=(int32, copy.'a)
        /// @resolution.name source=index target=copy.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=index root=copy.index

    }
}

function copyManaged(target: int32[], source: int32[]): void {
/// @type.symbol symbol=copyManaged type=(int32[], int32[]) => void
/// @type.symbol symbol=copyManaged.target source="target: int32[]" type=int32[]
/// @type.symbol symbol=copyManaged.source source="source: int32[]" type=int32[]

    for (let index: isize = 0; index < source.length; index++) {
    /// @type.symbol symbol=copyManaged.index source=index type=isize
    /// @resolution.pattern source=index kind=binding target=copyManaged.index
    /// @resolution.name source=index target=copyManaged.index
    /// @resolution.operator source="index < source.length" type=boolean operator="<" kind=builtin operands=[index as isize families=(integer), source.length as isize families=(integer)]
    /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=index root=copyManaged.index
    /// @resolution.name source=source target=copyManaged.source
    /// @resolution.member source=source.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
    /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=source root=copyManaged.source
    /// @generic.instantiation id="length<int32, \"managed\" & \"local\">" template=length arguments=(int32, "managed" & "local")
    /// @resolution.name source=index target=copyManaged.index
    /// @resolution.assignment source=index read=binding(copyManaged.index) write=binding(copyManaged.index) type=isize
    /// @resolution.access source=index root=copyManaged.index
    /// @resolution.operator source=index++ type=isize operator="++" kind=builtin operands=[index as isize families=(integer)]

        target[index] = source[index];
        /// @resolution.name source=target target=copyManaged.target
        /// @resolution.place source=target placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=target root=copyManaged.target
        /// @resolution.pattern.assign source=target[index] kind=place
        /// @resolution.assignment source=target[index] write="indexSet#1(parameters=(isize, int32), arguments=(provided(index) as isize, supplied(0) as int32), return=void, regions=(\"managed\" & \"local\"))" type=int32
        /// @generic.instantiation id="indexSet#1<int32, \"managed\" & \"local\">" template=indexSet#1 arguments=(int32, "managed" & "local")
        /// @resolution.name source=index target=copyManaged.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=index root=copyManaged.index
        /// @resolution.name source=source target=copyManaged.source
        /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=source root=copyManaged.source
        /// @resolution.subscript source=source[index] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(index) as isize), return=int32, regions=(\"managed\" & \"local\"))"
        /// @generic.instantiation id="index#2<int32, \"managed\" & \"local\">" template=index#2 arguments=(int32, "managed" & "local")
        /// @resolution.name source=index target=copyManaged.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=index root=copyManaged.index

    }
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_index_newtype_slices_through_place_generic_extensions() {
    let session = TestSession::builder()
        .module(
            "packet.ds",
            r#"
export newtype Packet<T> = [T];

export extension<T> of Packet<T> implements Index<isize> {
    type Output = T;

    index<const A: Access = "readonly">(
        this: WithAccess<&[T], A>,
        index: isize,
    ): WithAccess<&T, A> {
        todo("Packet.index")
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Packet } from "./packet.ds";

export extension<T: Copy> of Packet<T> implements Iterable<T> {
    /// Iterate copied values.
    iterator(&readonly this): Iterator<T> {
        todo("Packet.iterator")
    }

    first(): T | undefined {
        this[0]
    }
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Packet } from "./packet.ds";

export extension<T: Copy> of Packet<T> implements Iterable<T> {
    /// Iterate copied values.
    iterator(&readonly this): Iterator<T> {
        todo("Packet.iterator" as string | undefined)
    }

    first(): T | undefined {
        this[0] as T | undefined
    }
}

=== dir ===
import { Packet } from "./packet.ds";

export extension<T: Copy> of Packet<T> implements Iterable<T> {
/// @generic.template symbol=<module>#2 parameters=(T: Copy)
/// @definition.extension symbol=<module>#2 form=exported target=packet.Packet<T>
/// @definition.implements symbol=<module>#2 source=Iterable<T> target=Iterable<T>
/// @definition.method symbol=first slot=first type=(this: packet.Packet<T>) => T | undefined
/// @definition.method symbol=iterator slot=iterator type=<iterator.'a>(this: &iterator.'a readonly packet.Packet<T>) => Iterator<T>
/// @definition.conformance symbol=<module>#2 member=Iterable.Iterator requirement=Iterable.Iterator
/// @definition.conformance symbol=<module>#2 member=iterator requirement=Iterable.iterator
/// @type.symbol symbol=T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Packet target=packet.Packet
/// @resolution.name source=T target=T
/// @resolution.name source=Iterable target=Iterable
/// @resolution.name source=T target=T

    /// Iterate copied values.
    iterator(&readonly this): Iterator<T> {
    /// @generic.template symbol=iterator parent=template#0 parameters=('a)
    /// @type.symbol symbol=iterator type=<iterator.'a>(this: &iterator.'a readonly packet.Packet<T>) => Iterator<T>
    /// @type.symbol symbol=iterator.this source="&readonly this" type=&iterator.'a readonly packet.Packet<T>
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=T target=T

        todo("Packet.iterator")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Packet.iterator\")" parameters=(string | undefined) arguments=(provided("Packet.iterator") as string | undefined) return=never kind=symbol target=todo

    }

    first(): T | undefined {
    /// @type.symbol symbol=first type=(this: packet.Packet<T>) => T | undefined
    /// @type.symbol symbol=first.this type=packet.Packet<T>
    /// @resolution.name source=T target=T

        this[0]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=packet.Packet<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.subscript source=this[0] type=T kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=T, regions=(\"managed\" & \"local\"))"
        /// @generic.instantiation id="index#2<T, \"managed\" & \"local\">" template=index#2 arguments=(T, "managed" & "local") owner=first
        /// @generic.instantiation id=packet.Packet<T> template=packet.Packet arguments=(T) owner=first

    }
}
"#,
        r#"
/// @diagnostic.error id=unnamed-exported-nonlocal-extension message="exported extension on nonlocal type 'Packet<T>' must have a name"
/// @diagnostic.label line=4 column=30 span="Packet" line_source="export extension<T: Copy> of Packet<T> implements Iterable<T> {"
/// @diagnostic.error id=interface-not-implemented message="type 'Packet<T>' does not implement interface 'Index<isize>'"
/// @diagnostic.label line=4 column=45 span="Index" line_source="export extension<T> of Packet<T> implements Index<isize> {"
"#,
    );
}
