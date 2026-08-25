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

function run<'a, P1: Place>(boxed: Box<Point>, readonlyBoxed: &'a readonly Box<Point>): int32 {
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
/// @generic.template symbol=run parameters=('a, P1: Place)
/// @type.symbol symbol=run type=<run.'a, run.P1: Place>(Box<Point>, &run.'a readonly Box<Point>) => int32
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
    /// @resolution.member source=boxed.x receiver=Box<Point> type=int32 kind=field target_receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "readonly">) -> Point) key=x target=Point.x target_type=int32
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed
    /// @resolution.access source=boxed.x root=run.boxed keys=[x]
    /// @generic.instantiation id="dereference<Point, \"readonly\", \"local\">" template=dereference arguments=(Point, "readonly", "local")

    boxed.x = 2;
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed
    /// @resolution.pattern.assign source=boxed.x kind=place
    /// @resolution.access source=boxed.x root=run.boxed keys=[x]
    /// @resolution.assignment source=boxed.x write="receiver=Box<Point>, target=field(receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, \"mutable\">) -> Point), target=Point.x, type=int32), type=int32" type=int32
    /// @generic.instantiation id="dereference<Point, \"mutable\", \"local\">" template=dereference arguments=(Point, "mutable", "local")

    (*boxed).y = 3;
    /// @resolution.pattern.assign source=(*boxed).y kind=place
    /// @resolution.assignment source=(*boxed).y write="receiver=Point, target=field(receiver=Point, target=Point.y, type=int32), type=int32" type=int32
    /// @resolution.place source=*boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*boxed type=Point operator="*" kind=call parameters=() return=WithAccess<&'frame Point, "readonly"> kind=symbol target=dereference receiver=Box<Point> adjustments=(borrow(&'frame readonly Box<Point>)) instance="Box<Point>.<extension#2>.dereference<\"local\">"
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed

    const viewed = readonlyBoxed.x;
    /// @type.symbol symbol=run.viewed source=viewed type=int32
    /// @resolution.pattern source=viewed kind=binding target=run.viewed
    /// @resolution.name source=readonlyBoxed target=run.readonlyBoxed
    /// @resolution.member source=readonlyBoxed.x receiver=&run.'a readonly Box<Point> type=int32 kind=field target_receiver=&run.'a readonly Box<Point> adjustments=(&run.'a readonly Box<Point> => direct -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&run.'a Point, "readonly">) -> Point) key=x target=Point.x target_type=int32
    /// @resolution.place source=readonlyBoxed placement=run.P1 lifetime=run.'a access="readonly"
    /// @resolution.access source=readonlyBoxed root=run.readonlyBoxed
    /// @resolution.access source=readonlyBoxed.x root=run.readonlyBoxed keys=[x]
    /// @generic.instantiation id="dereference<Point, \"readonly\", run.P1>" template=dereference arguments=(Point, "readonly", run.P1)

    readonlyBoxed.x = 4;
    /// @resolution.name source=readonlyBoxed target=run.readonlyBoxed
    /// @resolution.place source=readonlyBoxed placement=run.P1 lifetime=run.'a access="readonly"
    /// @resolution.access source=readonlyBoxed root=run.readonlyBoxed
    /// @resolution.rejected source=readonlyBoxed.x

    return read + viewed;
    /// @resolution.name source=read target=run.read
    /// @resolution.operator source="read + viewed" type=int32 operator="+" kind=builtin operands=[read as int32 families=(integer), viewed as int32 families=(integer)]
    /// @resolution.place source=read placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=read root=run.read
    /// @resolution.name source=viewed target=run.viewed
    /// @resolution.place source=viewed placement="local" lifetime="frame" access="readonly"
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
    /// @resolution.operator source=*boxed type=int32 operator="*" kind=call parameters=() return=WithAccess<&'frame int32, "readonly"> kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#2>.dereference<\"local\">"
    /// @generic.instantiation id="dereference<int32, \"readonly\", \"local\">" template=dereference arguments=(int32, "readonly", "local")
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed

    *boxed = before + 1;
    /// @resolution.pattern.assign source=*boxed kind=place
    /// @resolution.assignment source=*boxed write="Box<int32> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame int32, \"mutable\">) -> int32" type=int32
    /// @generic.instantiation id="dereference<int32, \"mutable\", \"local\">" template=dereference arguments=(int32, "mutable", "local")
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed
    /// @resolution.name source=before target=run.before
    /// @resolution.operator source="before + 1" type=int32 operator="+" kind=builtin operands=[before as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=before placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=before root=run.before

    const view: &readonly int32 = &readonly *boxed;
    /// @type.symbol symbol=run.view source=view type=&'frame readonly int32
    /// @resolution.pattern source=view kind=binding target=run.view
    /// @resolution.place source=*boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*boxed type=int32 operator="*" kind=call parameters=() return=WithAccess<&'frame int32, "readonly"> kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#2>.dereference<\"local\">"
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed

    *boxed = 3;
    /// @resolution.pattern.assign source=*boxed kind=place
    /// @resolution.assignment source=*boxed write="Box<int32> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame int32, \"mutable\">) -> int32" type=int32
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed

    const useView = *view;
    /// @type.symbol symbol=run.useView source=useView type=int32
    /// @resolution.pattern source=useView kind=binding target=run.useView
    /// @resolution.operator source=*view type=int32 operator="*" kind=builtin operands=[view as &'frame readonly int32]
    /// @resolution.name source=view target=run.view
    /// @resolution.place source=view placement="local" lifetime="frame" access="readonly"
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
    /// @resolution.member source=shared.x receiver=Rc<Point> type=int32 kind=field target_receiver=Rc<Point> adjustments=(Rc<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "readonly">) -> Point) key=x target=Point.x target_type=int32
    /// @resolution.place source=shared placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=shared root=run.shared
    /// @resolution.access source=shared.x root=run.shared keys=[x]
    /// @generic.instantiation id="dereference<Point, \"local\">" template=dereference arguments=(Point, "local")

    shared.x = 2;
    /// @resolution.name source=shared target=run.shared
    /// @resolution.place source=shared placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=shared root=run.shared
    /// @resolution.rejected source=shared.x

    return read;
    /// @resolution.name source=read target=run.read
    /// @resolution.place source=read placement="local" lifetime="frame" access="readonly"
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
    /// @resolution.operator source=*left type=int32 operator="*" kind=call parameters=() return=WithAccess<&'frame int32, "readonly"> kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#2>.dereference<\"local\">"
    /// @generic.instantiation id="dereference<int32, \"readonly\", \"local\">" template=dereference arguments=(int32, "readonly", "local")
    /// @resolution.name source=left target=run.left
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=run.left
    /// @resolution.place source=*right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*right type=int32 operator="*" kind=call parameters=() return=WithAccess<&'frame int32, "readonly"> kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#2>.dereference<\"local\">"
    /// @resolution.name source=right target=run.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=run.right

    const same = *left == *right;
    /// @type.symbol symbol=run.same source=same type=boolean
    /// @resolution.pattern source=same kind=binding target=run.same
    /// @resolution.operator source="*left == *right" type=boolean operator="==" kind=builtin operands=[*left as int32 families=(integer), *right as int32 families=(integer)]
    /// @resolution.place source=*left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*left type=int32 operator="*" kind=call parameters=() return=WithAccess<&'frame int32, "readonly"> kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#2>.dereference<\"local\">"
    /// @resolution.name source=left target=run.left
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=run.left
    /// @resolution.place source=*right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.operator source=*right type=int32 operator="*" kind=call parameters=() return=WithAccess<&'frame int32, "readonly"> kind=symbol target=dereference receiver=Box<int32> adjustments=(borrow(&'frame readonly Box<int32>)) instance="Box<int32>.<extension#2>.dereference<\"local\">"
    /// @resolution.name source=right target=run.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=run.right

    return same ? sum : 0;
    /// @resolution.name source=same target=run.same
    /// @resolution.place source=same placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=same root=run.same
    /// @resolution.name source=sum target=run.sum
    /// @resolution.place source=sum placement="local" lifetime="frame" access="readonly"
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

    bump(&exclusive this): void {
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

    bump(&exclusive this): void {
        this.x += 1;
    }
}

function run<'a, P1: Place>(
    boxed: Box<Point>,
    nested: Box<Box<Point>>,
    viewed: &'a readonly Box<Point>,
): int32 {
    boxed.bump<"local">();
    nested.bump<"local">();
    nested.x = 4;
    viewed.bump();
    return boxed.sum<"local">() + nested.sum<"local">() + viewed.sum<"local">() + nested.y;
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
/// @definition.method symbol=bump slot=bump type=<bump.'a, bump.P1: Place>(this: &bump.'a exclusive this) => void
/// @definition.method symbol=sum slot=sum type=<sum.'a, sum.P1: Place>(this: &sum.'a readonly this) => int32
/// @resolution.name source=Point target=Point

    sum(&readonly this): int32 {
    /// @generic.template symbol=sum parameters=('a, P1: Place)
    /// @type.symbol symbol=sum type=<sum.'a, sum.P1: Place>(this: &sum.'a readonly this) => int32
    /// @type.symbol symbol=sum.this source="&readonly this" type=&sum.'a readonly this

        return this.x + this.y;
        /// @resolution.member source=this.x receiver=&sum.'a readonly Point type=int32 kind=field target_receiver=&sum.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.operator source="this.x + this.y" type=int32 operator="+" kind=builtin operands=[this.x as int32 families=(integer), this.y as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&sum.'a readonly Point
        /// @resolution.place source=this placement=sum.P1 lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=sum.P1 lifetime=sum.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @resolution.member source=this.y receiver=&sum.'a readonly Point type=int32 kind=field target_receiver=&sum.'a readonly Point key=y target=Point.y target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&sum.'a readonly Point
        /// @resolution.place source=this placement=sum.P1 lifetime=sum.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.y placement=sum.P1 lifetime=sum.'a access="readonly"
        /// @resolution.access source=this.y root=this keys=[y]

    }

    bump(&exclusive this): void {
    /// @generic.template symbol=bump parameters=('a, P1: Place)
    /// @type.symbol symbol=bump type=<bump.'a, bump.P1: Place>(this: &bump.'a exclusive this) => void
    /// @type.symbol symbol=bump.this source="&exclusive this" type=&bump.'a exclusive this

        this.x += 1;
        /// @resolution.operator source="this.x += 1" type=int32 operator="+" kind=builtin operands=[this.x as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&bump.'a exclusive Point
        /// @resolution.place source=this placement=bump.P1 lifetime=bump.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.x kind=place
        /// @resolution.assignment source=this.x read="receiver=&bump.'a exclusive Point, target=field(receiver=&bump.'a exclusive Point, target=Point.x, type=int32), type=int32" write="receiver=&bump.'a exclusive Point, target=field(receiver=&bump.'a exclusive Point, target=Point.x, type=int32), type=int32" type=int32
        /// @resolution.access source=this.x root=this keys=[x]

    }
}

function run(boxed: Box<Point>, nested: Box<Box<Point>>, viewed: &readonly Box<Point>): int32 {
/// @generic.template symbol=run parameters=('a, P1: Place)
/// @type.symbol symbol=run type=<run.'a, run.P1: Place>(Box<Point>, Box<Box<Point>>, &run.'a readonly Box<Point>) => int32
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
    /// @resolution.member source=boxed.bump receiver=Box<Point> type=<bump.'a, bump.P1: Place>(this: &bump.'a exclusive Point) => void kind=symbol target_receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "exclusive">) -> Point) target=bump
    /// @resolution.call source=boxed.bump() parameters=() return=void kind=symbol target=bump receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "exclusive">) -> Point, borrow(&'frame exclusive Point)) instance="Point.<extension#1>.bump<\"local\">"
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed
    /// @generic.instantiation id="bump<\"local\">" template=bump arguments=("local")
    /// @generic.instantiation id="dereference<Point, \"exclusive\", \"local\">" template=dereference arguments=(Point, "exclusive", "local")

    nested.bump();
    /// @resolution.name source=nested target=run.nested
    /// @resolution.member source=nested.bump receiver=Box<Box<Point>> type=<bump.'a, bump.P1: Place>(this: &bump.'a exclusive Point) => void kind=symbol target_receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Box<Point>, "exclusive">) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "exclusive">) -> Point) target=bump
    /// @resolution.call source=nested.bump() parameters=() return=void kind=symbol target=bump receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Box<Point>, "exclusive">) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "exclusive">) -> Point, borrow(&'frame exclusive Point)) instance="Point.<extension#1>.bump<\"local\">"
    /// @resolution.place source=nested placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested root=run.nested
    /// @generic.instantiation id="dereference<Box<Point>, \"exclusive\", \"local\">" template=dereference arguments=(Box<Point>, "exclusive", "local")

    nested.x = 4;
    /// @resolution.name source=nested target=run.nested
    /// @resolution.place source=nested placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested root=run.nested
    /// @resolution.pattern.assign source=nested.x kind=place
    /// @resolution.access source=nested.x root=run.nested keys=[x]
    /// @resolution.assignment source=nested.x write="receiver=Box<Box<Point>>, target=field(receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Box<Point>, \"mutable\">) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, \"mutable\">) -> Point), target=Point.x, type=int32), type=int32" type=int32
    /// @generic.instantiation id="dereference<Box<Point>, \"mutable\", \"local\">" template=dereference arguments=(Box<Point>, "mutable", "local")
    /// @generic.instantiation id="dereference<Point, \"mutable\", \"local\">" template=dereference arguments=(Point, "mutable", "local")

    viewed.bump();
    /// @resolution.name source=viewed target=run.viewed
    /// @resolution.place source=viewed placement=run.P1 lifetime=run.'a access="readonly"
    /// @resolution.access source=viewed root=run.viewed
    /// @resolution.rejected source=viewed.bump
    /// @resolution.rejected source=viewed.bump()

    return boxed.sum() + nested.sum() + viewed.sum() + nested.y;
    /// @resolution.name source=boxed target=run.boxed
    /// @resolution.member source=boxed.sum receiver=Box<Point> type=<sum.'a, sum.P1: Place>(this: &sum.'a readonly Point) => int32 kind=symbol target_receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "readonly">) -> Point) target=sum
    /// @resolution.call source=boxed.sum() parameters=() return=int32 kind=symbol target=sum receiver=Box<Point> adjustments=(Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "readonly">) -> Point, borrow(&'frame readonly Point)) instance="Point.<extension#1>.sum<\"local\">"
    /// @resolution.operator source="boxed.sum() + nested.sum() + viewed.sum() + nested.y" type=int32 operator="+" kind=builtin operands=[boxed.sum() + nested.sum() + viewed.sum() as int32 families=(integer), nested.y as int32 families=(integer)]
    /// @resolution.operator source="boxed.sum() + nested.sum() + viewed.sum()" type=int32 operator="+" kind=builtin operands=[boxed.sum() + nested.sum() as int32 families=(integer), viewed.sum() as int32 families=(integer)]
    /// @resolution.operator source="boxed.sum() + nested.sum()" type=int32 operator="+" kind=builtin operands=[boxed.sum() as int32 families=(integer), nested.sum() as int32 families=(integer)]
    /// @resolution.place source=boxed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=boxed root=run.boxed
    /// @generic.instantiation id="dereference<Point, \"readonly\", \"local\">" template=dereference arguments=(Point, "readonly", "local")
    /// @generic.instantiation id="sum<\"local\">" template=sum arguments=("local")
    /// @resolution.name source=nested target=run.nested
    /// @resolution.member source=nested.sum receiver=Box<Box<Point>> type=<sum.'a, sum.P1: Place>(this: &sum.'a readonly Point) => int32 kind=symbol target_receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Box<Point>, "readonly">) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "readonly">) -> Point) target=sum
    /// @resolution.call source=nested.sum() parameters=() return=int32 kind=symbol target=sum receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Box<Point>, "readonly">) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "readonly">) -> Point, borrow(&'frame readonly Point)) instance="Point.<extension#1>.sum<\"local\">"
    /// @resolution.place source=nested placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested root=run.nested
    /// @generic.instantiation id="dereference<Box<Point>, \"readonly\", \"local\">" template=dereference arguments=(Box<Point>, "readonly", "local")
    /// @resolution.name source=viewed target=run.viewed
    /// @resolution.member source=viewed.sum receiver=&run.'a readonly Box<Point> type=<sum.'a, sum.P1: Place>(this: &sum.'a readonly Point) => int32 kind=symbol target_receiver=&run.'a readonly Box<Point> adjustments=(&run.'a readonly Box<Point> => direct -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&run.'a Point, "readonly">) -> Point) target=sum
    /// @resolution.call source=viewed.sum() parameters=() return=int32 kind=symbol target=sum receiver=&run.'a readonly Box<Point> adjustments=(&run.'a readonly Box<Point> => direct -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&run.'a Point, "readonly">) -> Point, borrow(&run.'a readonly Point)) instance="Point.<extension#1>.sum<\"local\">"
    /// @resolution.place source=viewed placement=run.P1 lifetime=run.'a access="readonly"
    /// @resolution.access source=viewed root=run.viewed
    /// @generic.instantiation id="dereference<Point, \"readonly\", run.P1>" template=dereference arguments=(Point, "readonly", run.P1)
    /// @resolution.name source=nested target=run.nested
    /// @resolution.member source=nested.y receiver=Box<Box<Point>> type=int32 kind=field target_receiver=Box<Box<Point>> adjustments=(Box<Box<Point>> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Box<Point>, "readonly">) -> Box<Point>, Box<Point> => dereference(parameters=(), arguments=(), return=WithAccess<&'frame Point, "readonly">) -> Point) key=y target=Point.y target_type=int32
    /// @resolution.place source=nested placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested root=run.nested
    /// @resolution.place source=nested.y placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=nested.y root=run.nested keys=[y]

}
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'exclusive' access is not granted by a value of type '&'a readonly Box<Point>'"
/// @diagnostic.label line=23 column=12 span="bump" line_source="viewed.bump();"
/// @diagnostic.note message="the source grants at most 'readonly' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
/// @diagnostic.error id=missing-member message="member 'bump' does not exist on type '&'a readonly Box<Point>'"
/// @diagnostic.label line=23 column=12 span="bump" line_source="viewed.bump();"
"#,
    );
}
