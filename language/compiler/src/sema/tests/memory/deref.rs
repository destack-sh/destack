use crate::tests::{DirRows, TestSession};

#[test]
fn test_read_and_write_through_a_box_dereference() {
    let session = TestSession::single(
        r#"
import { Box } from "destack:memory";

struct Point {
    x: int32;
    y: int32;
}

function run(boxed: Box<Point>, readonlyBoxed: &readonly Box<Point>): int32 {
    const read = boxed.x;
    boxed.x = 2;
    (*boxed).y = 3;
    const viewed = readonlyBoxed.x;
    readonlyBoxed.x = 4;
    return read + viewed;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Box } from "destack:memory";

struct Point {
    x: int32;
    y: int32;
}

function run<'a>(boxed: Box<Point>, readonlyBoxed: &'a readonly Box<Point>): int32 {
    const read: int32 = boxed.x;
    boxed.x = 2;
    (*boxed).y = 3;
    const viewed: int32 = readonlyBoxed.x;
    readonlyBoxed.x = 4;
    return read + viewed;
}

=== dir ===
import { Box } from "destack:memory";

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

function run(boxed: Box<Point>, readonlyBoxed: &readonly Box<Point>): int32 {
/// @generic.template symbol=run parameters=('a)
/// @type.symbol symbol=run type=<run.'a>(Box<Point>, &run.'a readonly Box<Point>) => int32
/// @type.symbol symbol=run.boxed source="boxed: Box<Point>" type=Box<Point>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Point target=Point
/// @type.symbol symbol=run.readonlyBoxed source="readonlyBoxed: &readonly Box<Point>" type=&run.'a readonly Box<Point>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Point target=Point

    const read = boxed.x;
    /// @type.symbol symbol=run.read source=read type=int32
    /// @resolution.pattern source=read kind=binding target=run.read
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.member source=boxed.x receiver=Box<Point> type=int32 kind=field target_receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=&'frame readonly Point, regions=("frame" & "local")) -> Point) key=x target=Point.x target_type=int32
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed
    /// @resolution.access source=boxed.x root=run.boxed keys=[x]
    /// @generic.instantiation id="dereference<Point, \"readonly\", \"frame\" & \"local\">" template=dereference arguments=(Point, "readonly", "frame" & "local")

    boxed.x = 2;
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed
    /// @resolution.pattern.assign source=boxed.x kind=place
    /// @resolution.place source=boxed.x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed.x root=run.boxed keys=[x]
    /// @resolution.assignment source=boxed.x write="receiver=Box<Point>, target=field(receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=&'frame Point, regions=(\"frame\" & \"local\")) -> Point), target=Point.x, type=int32), type=int32" type=int32
    /// @generic.instantiation id="dereference<Point, \"mutable\", \"frame\" & \"local\">" template=dereference arguments=(Point, "mutable", "frame" & "local")

    (*boxed).y = 3;
    /// @resolution.pattern.assign source=(*boxed).y kind=place
    /// @resolution.place source=(*boxed).y placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=(*boxed).y write="receiver=Point, target=field(receiver=Point, target=Point.y, type=int32), type=int32" type=int32
    /// @resolution.place source=*boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*boxed type=Point operator="*" kind=call parameters=() return=&'frame readonly Point regions=("frame" & "local") kind=symbol target=dereference receiver=Box<Point> adjustments=(borrow(&'frame readonly Box<Point>)) instance="Box<Point>.<extension#3>.dereference<\"frame\" & \"local\">"
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed

    const viewed = readonlyBoxed.x;
    /// @type.symbol symbol=run.viewed source=viewed type=int32
    /// @resolution.pattern source=viewed kind=binding target=run.viewed
    /// @resolution.name source=readonlyBoxed target=run.readonlyBoxed
    /// @resolution.member source=readonlyBoxed.x receiver=&run.'a readonly Box<Point> type=int32 kind=field target_receiver=&run.'a readonly Box<Point> adjustments=(&run.'a readonly Box<Point> => builtin -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=&run.'a readonly Point, regions=(run.'a)) -> Point) key=x target=Point.x target_type=int32
    /// @resolution.place source=readonlyBoxed placement=run.'a lifetime=run.'a access="readonly"
    /// @resolution.access source=readonlyBoxed root=run.readonlyBoxed
    /// @resolution.access source=readonlyBoxed.x root=run.readonlyBoxed keys=[x]
    /// @generic.instantiation id="dereference<Point, \"readonly\", run.'a>" template=dereference arguments=(Point, "readonly", run.'a)

    readonlyBoxed.x = 4;
    /// @resolution.name source=readonlyBoxed target=run.readonlyBoxed
    /// @resolution.place source=readonlyBoxed placement=run.'a lifetime=run.'a access="readonly"
    /// @resolution.access source=readonlyBoxed root=run.readonlyBoxed
    /// @resolution.rejected source=readonlyBoxed.x

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
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type 'readonly &'a readonly Box<Point>'"
/// @diagnostic.label line=14 column=19 span="x" line_source="readonlyBoxed.x = 4;"
/// @diagnostic.note message="the source grants at most 'readonly' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
"#,
    );
}

#[test]
fn test_record_an_exclusive_write_through_a_dereference() {
    let session = TestSession::single(
        r#"
import { Box } from "destack:memory";

function run(boxed: Box<int32>): void {
    const before = *boxed;
    *boxed = before + 1;
    const view: &readonly int32 = &readonly *boxed;
    *boxed = 3;
    const useView = *view;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Box } from "destack:memory";

function run(boxed: Box<int32>): void {
    const before: int32 = *boxed;
    *boxed = before + 1;
    const view: &'frame readonly int32 = &readonly *boxed;
    *boxed = 3;
    const useView: int32 = *view;
}

=== dir ===
import { Box } from "destack:memory";

function run(boxed: Box<int32>): void {
/// @type.symbol symbol=run type=(Box<int32>) => void
/// @type.symbol symbol=run.boxed source="boxed: Box<int32>" type=Box<int32>
/// @resolution.name source=Box target=Box

    const before = *boxed;
    /// @type.symbol symbol=run.before source=before type=int32
    /// @resolution.pattern source=before kind=binding target=run.before
    /// @resolution.operator source=*boxed type=int32 operator="*" kind=call parameters=() return=&'frame readonly int32 regions=("frame" & "local") kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#3>.dereference<\"frame\" & \"local\">"
    /// @generic.instantiation id="dereference<int32, \"readonly\", \"frame\" & \"local\">" template=dereference arguments=(int32, "readonly", "frame" & "local")
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed

    *boxed = before + 1;
    /// @resolution.pattern.assign source=*boxed kind=place
    /// @resolution.place source=*boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=*boxed write="Box<int32> => dereference(parameters=(), arguments=(), return=&'frame int32, regions=(\"frame\" & \"local\")) -> int32" type=int32
    /// @generic.instantiation id="dereference<int32, \"mutable\", \"frame\" & \"local\">" template=dereference arguments=(int32, "mutable", "frame" & "local")
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed
    /// @resolution.name source=before target=run.before
    /// @resolution.operator source="before + 1" type=int32 operator="+" kind=builtin operands=[before as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=before placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=before root=run.before

    const view: &readonly int32 = &readonly *boxed;
    /// @type.symbol symbol=run.view source=view type=&'frame readonly int32
    /// @resolution.pattern source=view kind=binding target=run.view
    /// @resolution.place source=*boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*boxed type=int32 operator="*" kind=call parameters=() return=&'frame readonly int32 regions=("frame" & "local") kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#3>.dereference<\"frame\" & \"local\">"
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed

    *boxed = 3;
    /// @resolution.pattern.assign source=*boxed kind=place
    /// @resolution.place source=*boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=*boxed write="Box<int32> => dereference(parameters=(), arguments=(), return=&'frame int32, regions=(\"frame\" & \"local\")) -> int32" type=int32
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed

    const useView = *view;
    /// @type.symbol symbol=run.useView source=useView type=int32
    /// @resolution.pattern source=useView kind=binding target=run.useView
    /// @resolution.operator source=*view type=int32 operator="*" kind=builtin operands=[view as &'frame readonly int32]
    /// @resolution.name source=view target=run.view
    /// @resolution.place source=view placement="frame" & "local" lifetime="frame" & "local" access="immutable"
    /// @resolution.access source=view root=run.view

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_dereference_a_shared_handle_readonly() {
    let session = TestSession::single(
        r#"
import { rc } from "destack:memory";

struct Point {
    x: int32;
}

function run(shared: rc.Rc<Point>): int32 {
    const read = shared.x;
    shared.x = 2;
    return read;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { rc } from "destack:memory";

struct Point {
    x: int32;
}

function run(shared: Rc<Point>): int32 {
    const read: int32 = shared.x;
    shared.x = 2;
    return read;
}

=== dir ===
import { rc } from "destack:memory";

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

function run(shared: rc.Rc<Point>): int32 {
/// @type.symbol symbol=run type=(Rc<Point>) => int32
/// @type.symbol symbol=run.shared source="shared: rc.Rc<Point>" type=Rc<Point>
/// @resolution.name source=rc.Rc target=Rc
/// @resolution.name source=Point target=Point

    const read = shared.x;
    /// @type.symbol symbol=run.read source=read type=int32
    /// @resolution.pattern source=read kind=binding target=run.read
    /// @resolution.name source=shared target=run.shared
    /// @resolution.member source=shared.x receiver=Rc<Point> type=int32 kind=field target_receiver=Rc<Point> adjustments=(Rc<Point> => dereference(parameters=(), arguments=(), return=&'frame immutable Point, regions=("frame" & "local")) -> Point) key=x target=Point.x target_type=int32
    /// @resolution.place source=shared placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=shared root=run.shared
    /// @resolution.access source=shared.x root=run.shared keys=[x]
    /// @generic.instantiation id="dereference<Point, \"frame\" & \"local\">" template=dereference arguments=(Point, "frame" & "local")

    shared.x = 2;
    /// @resolution.name source=shared target=run.shared
    /// @resolution.place source=shared placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=shared root=run.shared
    /// @resolution.rejected source=shared.x

    return read;
    /// @resolution.name source=read target=run.read
    /// @resolution.place source=read placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=read root=run.read

}
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type 'Rc<Point>'"
/// @diagnostic.label line=10 column=12 span="x" line_source="shared.x = 2;"
/// @diagnostic.note message="the source grants at most 'readonly' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
"#,
    );
}

#[test]
fn test_dereference_an_operand_in_an_operator() {
    let session = TestSession::single(
        r#"
import { Box } from "destack:memory";

function run(left: Box<int32>, right: Box<int32>): int32 {
    const sum = *left + *right;
    const same = *left == *right;
    return same ? sum : 0;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Box } from "destack:memory";

function run(left: Box<int32>, right: Box<int32>): int32 {
    const sum: int32 = *left + *right;
    const same: boolean = *left == *right;
    return same ? sum : 0;
}

=== dir ===
import { Box } from "destack:memory";

function run(left: Box<int32>, right: Box<int32>): int32 {
/// @type.symbol symbol=run type=(Box<int32>, Box<int32>) => int32
/// @type.symbol symbol=run.left source="left: Box<int32>" type=Box<int32>
/// @resolution.name source=Box target=Box
/// @type.symbol symbol=run.right source="right: Box<int32>" type=Box<int32>
/// @resolution.name source=Box target=Box

    const sum = *left + *right;
    /// @type.symbol symbol=run.sum source=sum type=int32
    /// @resolution.pattern source=sum kind=binding target=run.sum
    /// @resolution.operator source="*left + *right" type=int32 operator="+" kind=builtin operands=[*left as int32 families=(integer), *right as int32 families=(integer)]
    /// @resolution.place source=*left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*left type=int32 operator="*" kind=call parameters=() return=&'frame readonly int32 regions=("frame" & "local") kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#3>.dereference<\"frame\" & \"local\">"
    /// @generic.instantiation id="dereference<int32, \"readonly\", \"frame\" & \"local\">" template=dereference arguments=(int32, "readonly", "frame" & "local")
    /// @resolution.name source=left target=run.left
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=run.left
    /// @resolution.place source=*right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*right type=int32 operator="*" kind=call parameters=() return=&'frame readonly int32 regions=("frame" & "local") kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#3>.dereference<\"frame\" & \"local\">"
    /// @resolution.name source=right target=run.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=run.right

    const same = *left == *right;
    /// @type.symbol symbol=run.same source=same type=boolean
    /// @resolution.pattern source=same kind=binding target=run.same
    /// @resolution.operator source="*left == *right" type=boolean operator="==" kind=builtin operands=[*left as int32 families=(integer), *right as int32 families=(integer)]
    /// @resolution.place source=*left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*left type=int32 operator="*" kind=call parameters=() return=&'frame readonly int32 regions=("frame" & "local") kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#3>.dereference<\"frame\" & \"local\">"
    /// @resolution.name source=left target=run.left
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=run.left
    /// @resolution.place source=*right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*right type=int32 operator="*" kind=call parameters=() return=&'frame readonly int32 regions=("frame" & "local") kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#3>.dereference<\"frame\" & \"local\">"
    /// @resolution.name source=right target=run.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=run.right

    return same ? sum : 0;
    /// @resolution.name source=same target=run.same
    /// @resolution.place source=same placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=same root=run.same
    /// @resolution.name source=sum target=run.sum
    /// @resolution.place source=sum placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=sum root=run.sum

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_dereference_nested_boxes_and_call_methods_through_them() {
    let session = TestSession::single(
        r#"
import { Box } from "destack:memory";

struct Point {
    x: int32;
    y: int32;
}

extension of Point {
    sum(&readonly this): int32 {
        return this.x + this.y;
    }

    bump(&this): void {
        this.x += 1;
    }
}

function run(boxed: Box<Point>, nested: Box<Box<Point>>, viewed: &readonly Box<Point>): int32 {
    boxed.bump();
    nested.bump();
    nested.x = 4;
    viewed.bump();
    return boxed.sum() + nested.sum() + viewed.sum() + nested.y;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Box } from "destack:memory";

struct Point {
    x: int32;
    y: int32;
}

extension of Point {
    sum(&readonly this): int32 {
        return this.x + this.y;
    }

    bump(&this): void {
        this.x += 1;
    }
}

function run<'a>(
    boxed: Box<Point>,
    nested: Box<Box<Point>>,
    viewed: &'a readonly Box<Point>,
): int32 {
    boxed.bump<"frame">();
    nested.bump<"frame">();
    nested.x = 4;
    viewed.bump();
    return boxed.sum<"frame">() + nested.sum<"frame">() + viewed.sum<'a>() + nested.y;
}

=== dir ===
import { Box } from "destack:memory";

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

extension of Point {
/// @definition.extension symbol=<module>#2 form=local target=Point
/// @definition.method symbol=bump slot=bump type=<bump.'a>(this: &bump.'a Point) => void
/// @definition.method symbol=sum slot=sum type=<sum.'a>(this: &sum.'a readonly Point) => int32
/// @resolution.name source=Point target=Point

    sum(&readonly this): int32 {
    /// @generic.template symbol=sum parameters=('a)
    /// @type.symbol symbol=sum type=<sum.'a>(this: &sum.'a readonly Point) => int32
    /// @type.symbol symbol=sum.this source="&readonly this" type=&sum.'a readonly Point

        return this.x + this.y;
        /// @resolution.member source=this.x receiver=&sum.'a readonly Point type=int32 kind=field target_receiver=&sum.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.operator source="this.x + this.y" type=int32 operator="+" kind=builtin operands=[this.x as int32 families=(integer), this.y as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&sum.'a readonly Point
        /// @resolution.place source=this placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @resolution.member source=this.y receiver=&sum.'a readonly Point type=int32 kind=field target_receiver=&sum.'a readonly Point key=y target=Point.y target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&sum.'a readonly Point
        /// @resolution.place source=this placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.y placement=sum.'a lifetime=sum.'a access="readonly"
        /// @resolution.access source=this.y root=this keys=[y]

    }

    bump(&this): void {
    /// @generic.template symbol=bump parameters=('a)
    /// @type.symbol symbol=bump type=<bump.'a>(this: &bump.'a Point) => void
    /// @type.symbol symbol=bump.this source=&this type=&bump.'a Point

        this.x += 1;
        /// @resolution.operator source="this.x += 1" type=int32 operator="+" kind=builtin operands=[this.x as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&bump.'a Point
        /// @resolution.place source=this placement=bump.'a lifetime=bump.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.x kind=place
        /// @resolution.place source=this.x placement=bump.'a lifetime=bump.'a access="mutable"
        /// @resolution.assignment source=this.x read="receiver=&bump.'a Point, target=field(receiver=&bump.'a Point, target=Point.x, type=int32), type=int32" write="receiver=&bump.'a Point, target=field(receiver=&bump.'a Point, target=Point.x, type=int32), type=int32" type=int32
        /// @resolution.access source=this.x root=this keys=[x]

    }
}

function run(boxed: Box<Point>, nested: Box<Box<Point>>, viewed: &readonly Box<Point>): int32 {
/// @generic.template symbol=run parameters=('a)
/// @type.symbol symbol=run type=<run.'a>(Box<Point>, Box<Box<Point>>, &run.'a readonly Box<Point>) => int32
/// @type.symbol symbol=run.boxed source="boxed: Box<Point>" type=Box<Point>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Point target=Point
/// @type.symbol symbol=run.nested source="nested: Box<Box<Point>>" type=Box<Box<Point>>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box target=Box
/// @resolution.name source=Point target=Point
/// @type.symbol symbol=run.viewed source="viewed: &readonly Box<Point>" type=&run.'a readonly Box<Point>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Point target=Point

    boxed.bump();
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.member source=boxed.bump receiver=Box<Point> type=<bump.'a>(this: &bump.'a Point) => void kind=symbol target_receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=&'frame Point, regions=("frame" & "local")) -> Point) target=bump
    /// @resolution.call source=boxed.bump() parameters=() return=void regions=("frame" & "local") kind=symbol target=bump receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=&'frame Point, regions=("frame" & "local")) -> Point, borrow(&'frame Point)) instance="Point.<extension#1>.bump<\"frame\" & \"local\">"
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed
    /// @generic.instantiation id="bump<\"frame\" & \"local\">" template=bump arguments=("frame" & "local")
    /// @generic.instantiation id="dereference<Point, \"mutable\", \"frame\" & \"local\">" template=dereference arguments=(Point, "mutable", "frame" & "local")

    nested.bump();
    /// @resolution.name source=nested target=run.nested
    /// @resolution.member source=nested.bump receiver=Box<Box<Point>> type=<bump.'a>(this: &bump.'a Point) => void kind=symbol target_receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=&'frame Box<Point>, regions=("frame" & "local")) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=&'frame Point, regions=("frame" & "local")) -> Point) target=bump
    /// @resolution.call source=nested.bump() parameters=() return=void regions=("frame" & "local") kind=symbol target=bump receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=&'frame Box<Point>, regions=("frame" & "local")) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=&'frame Point, regions=("frame" & "local")) -> Point, borrow(&'frame Point)) instance="Point.<extension#1>.bump<\"frame\" & \"local\">"
    /// @resolution.place source=nested placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested root=run.nested
    /// @generic.instantiation id="dereference<Box<Point>, \"mutable\", \"frame\" & \"local\">" template=dereference arguments=(Box<Point>, "mutable", "frame" & "local")

    nested.x = 4;
    /// @resolution.name source=nested target=run.nested
    /// @resolution.place source=nested placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested root=run.nested
    /// @resolution.pattern.assign source=nested.x kind=place
    /// @resolution.place source=nested.x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested.x root=run.nested keys=[x]
    /// @resolution.assignment source=nested.x write="receiver=Box<Box<Point>>, target=field(receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=&'frame Box<Point>, regions=(\"frame\" & \"local\")) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=&'frame Point, regions=(\"frame\" & \"local\")) -> Point), target=Point.x, type=int32), type=int32" type=int32

    viewed.bump();
    /// @resolution.name source=viewed target=run.viewed
    /// @resolution.place source=viewed placement=run.'a lifetime=run.'a access="readonly"
    /// @resolution.access source=viewed root=run.viewed
    /// @resolution.rejected source=viewed.bump
    /// @resolution.rejected source=viewed.bump()

    return boxed.sum() + nested.sum() + viewed.sum() + nested.y;
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.member source=boxed.sum receiver=Box<Point> type=<sum.'a>(this: &sum.'a readonly Point) => int32 kind=symbol target_receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=&'frame readonly Point, regions=("frame" & "local")) -> Point) target=sum
    /// @resolution.call source=boxed.sum() parameters=() return=int32 regions=("frame" & "local") kind=symbol target=sum receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=&'frame readonly Point, regions=("frame" & "local")) -> Point, borrow(&'frame readonly Point)) instance="Point.<extension#1>.sum<\"frame\" & \"local\">"
    /// @resolution.operator source="boxed.sum() + nested.sum() + viewed.sum() + nested.y" type=int32 operator="+" kind=builtin operands=[boxed.sum() + nested.sum() + viewed.sum() as int32 families=(integer), nested.y as int32 families=(integer)]
    /// @resolution.operator source="boxed.sum() + nested.sum() + viewed.sum()" type=int32 operator="+" kind=builtin operands=[boxed.sum() + nested.sum() as int32 families=(integer), viewed.sum() as int32 families=(integer)]
    /// @resolution.operator source="boxed.sum() + nested.sum()" type=int32 operator="+" kind=builtin operands=[boxed.sum() as int32 families=(integer), nested.sum() as int32 families=(integer)]
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed
    /// @generic.instantiation id="dereference<Point, \"readonly\", \"frame\" & \"local\">" template=dereference arguments=(Point, "readonly", "frame" & "local")
    /// @generic.instantiation id="sum<\"frame\" & \"local\">" template=sum arguments=("frame" & "local")
    /// @resolution.name source=nested target=run.nested
    /// @resolution.member source=nested.sum receiver=Box<Box<Point>> type=<sum.'a>(this: &sum.'a readonly Point) => int32 kind=symbol target_receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=&'frame readonly Box<Point>, regions=("frame" & "local")) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=&'frame readonly Point, regions=("frame" & "local")) -> Point) target=sum
    /// @resolution.call source=nested.sum() parameters=() return=int32 regions=("frame" & "local") kind=symbol target=sum receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=&'frame readonly Box<Point>, regions=("frame" & "local")) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=&'frame readonly Point, regions=("frame" & "local")) -> Point, borrow(&'frame readonly Point)) instance="Point.<extension#1>.sum<\"frame\" & \"local\">"
    /// @resolution.place source=nested placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested root=run.nested
    /// @generic.instantiation id="dereference<Box<Point>, \"readonly\", \"frame\" & \"local\">" template=dereference arguments=(Box<Point>, "readonly", "frame" & "local")
    /// @resolution.name source=viewed target=run.viewed
    /// @resolution.member source=viewed.sum receiver=&run.'a readonly Box<Point> type=<sum.'a>(this: &sum.'a readonly Point) => int32 kind=symbol target_receiver=&run.'a readonly Box<Point> adjustments=(&run.'a readonly Box<Point> => builtin -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=&run.'a readonly Point, regions=(run.'a)) -> Point) target=sum
    /// @resolution.call source=viewed.sum() parameters=() return=int32 regions=(run.'a) kind=symbol target=sum receiver=&run.'a readonly Box<Point> adjustments=(&run.'a readonly Box<Point> => builtin -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=&run.'a readonly Point, regions=(run.'a)) -> Point, borrow(&run.'a readonly Point)) instance=Point.<extension#1>.sum<run.'a>
    /// @resolution.place source=viewed placement=run.'a lifetime=run.'a access="readonly"
    /// @resolution.access source=viewed root=run.viewed
    /// @generic.instantiation id="dereference<Point, \"readonly\", run.'a>" template=dereference arguments=(Point, "readonly", run.'a)
    /// @generic.instantiation id=sum<run.'a> template=sum arguments=(run.'a)
    /// @resolution.name source=nested target=run.nested
    /// @resolution.member source=nested.y receiver=Box<Box<Point>> type=int32 kind=field target_receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=&'frame readonly Box<Point>, regions=("frame" & "local")) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=&'frame readonly Point, regions=("frame" & "local")) -> Point) key=y target=Point.y target_type=int32
    /// @resolution.place source=nested placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested root=run.nested
    /// @resolution.place source=nested.y placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested.y root=run.nested keys=[y]

}
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type '&'a readonly Box<Point>'"
/// @diagnostic.label line=23 column=12 span="bump" line_source="viewed.bump();"
/// @diagnostic.note message="the source grants at most 'readonly' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
/// @diagnostic.error id=missing-member message="member 'bump' does not exist on type '&'a readonly Box<Point>'"
/// @diagnostic.label line=23 column=12 span="bump" line_source="viewed.bump();"
"#,
    );
}

/// Project an associated target through a dereference an extension declares on a struct.
#[test]
fn test_project_an_associated_target_through_an_extension_dereference_on_a_struct() {
    let session = TestSession::single(
        r#"
import { todo } from "destack:error";
import { Borrowed, Region } from "destack:memory";
import { Dereference } from "destack:ops";

struct Cell<T> {
    value: T;
}

extension<T> of Cell<T> implements Dereference {
    type Target = T;

    dereference<const R: Region>(
        this: Borrowed<this, R, "readonly">,
    ): Borrowed<this.Target, R, "immutable"> {
        todo("Cell.dereference")
    }

    borrow(&readonly this): &readonly T {
        this.dereference()
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";
import { Borrowed, Region } from "destack:memory";
import { Dereference } from "destack:ops";

struct Cell<out T> {
    value: T;
}

extension<T> of Cell<T> implements Dereference {
    type Target = T;

    dereference<const R: Region>(this: Borrowed<this, R, "readonly">): Borrowed<T, R, "immutable"> {
        todo("Cell.dereference" as string | undefined)
    }

    borrow(&readonly this): &'a readonly T {
        this.dereference<T, 'a>()
    }
}

=== dir ===
import { todo } from "destack:error";
import { Borrowed, Region } from "destack:memory";
import { Dereference } from "destack:ops";

struct Cell<T> {
/// @generic.template symbol=Cell parameters=(out T#1)
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell template=(out T#1)
/// @definition.field symbol=Cell.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Cell.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Cell.value source="value: T" type=T#1
    /// @resolution.name source=T target=Cell.T

}

extension<T> of Cell<T> implements Dereference {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Cell<T#2>
/// @definition.implements symbol=<module>#2 source=Dereference target=Dereference
/// @definition.associated.type symbol=Target source="type Target = T" key=Target value=T#2
/// @definition.method symbol=borrow slot=borrow type=<borrow.'a>(this: &borrow.'a readonly Cell<T#2>) => &borrow.'a readonly T#2
/// @definition.method symbol=dereference slot=dereference type=<const R>(this: Borrowed<Cell<T#2>, R, "readonly">) => Borrowed<T#2, R, "immutable">
/// @definition.conformance symbol=<module>#2 member=Dereference.OutputAccess requirement=Dereference.OutputAccess
/// @definition.conformance symbol=<module>#2 member=Target requirement=Dereference.Target
/// @definition.conformance symbol=<module>#2 member=dereference requirement=Dereference.dereference
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=T target=T
/// @resolution.name source=Dereference target=Dereference

    type Target = T;
    /// @type.symbol symbol=Target source="type Target = T" type=T#2
    /// @resolution.name source=T target=T

    dereference<const R: Region>(
    /// @generic.template symbol=dereference parent=template#1 parameters=(const R: Region)
    /// @type.symbol symbol=dereference type=<const R>(this: Borrowed<Cell<T#2>, R, "readonly">) => Borrowed<T#2, R, "immutable">
    /// @type.symbol symbol=dereference.R source="const R: Region" type=R
    /// @resolution.name source=Region target=Region

        this: Borrowed<this, R, "readonly">,
        /// @type.symbol symbol=dereference.this source="this: Borrowed<this, R, \"readonly\">" type=Borrowed<Cell<T#2>, R, "readonly">
        /// @resolution.name source=Borrowed target=Borrowed
        /// @resolution.name source=R target=dereference.R

    ): Borrowed<this.Target, R, "immutable"> {
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=this.Target target=Target
    /// @resolution.name source=R target=dereference.R

        todo("Cell.dereference")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Cell.dereference\")" parameters=(string | undefined) arguments=(provided("Cell.dereference") as string | undefined) return=never kind=symbol target=todo

    }

    borrow(&readonly this): &readonly T {
    /// @generic.template symbol=borrow parent=template#1 parameters=('a)
    /// @type.symbol symbol=borrow type=<borrow.'a>(this: &borrow.'a readonly Cell<T#2>) => &borrow.'a readonly T#2
    /// @type.symbol symbol=borrow.this source="&readonly this" type=&borrow.'a readonly Cell<T#2>
    /// @resolution.name source=T target=T

        this.dereference()
        /// @resolution.member source=this.dereference receiver=&borrow.'a readonly Cell<T#2> type=<const R>(this: Borrowed<Cell<T#2>, R, "readonly">) => Borrowed<T#2, R, "immutable"> kind=symbol target_receiver=&borrow.'a readonly Cell<T#2> target=dereference
        /// @resolution.call source=this.dereference() parameters=() return=&borrow.'a immutable T#2 regions=(borrow.'a) kind=symbol target=dereference receiver=&borrow.'a readonly Cell<T#2> instance=Cell<T#2>.<extension#1>.dereference<borrow.'a>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&borrow.'a readonly Cell<T#2>
        /// @resolution.place source=this placement=borrow.'a lifetime=borrow.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="dereference<T#2, borrow.'a>" template=dereference arguments=(T#2, borrow.'a) owner=borrow
        /// @generic.instantiation id=dereference<T#2> template=dereference arguments=(T#2) owner=borrow

    }
}
"#,
        r#"
"#,
    );
}
