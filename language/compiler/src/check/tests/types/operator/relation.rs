use crate::tests::{DirRows, TestSession};

#[test]
fn test_extends_relation_evaluates_assignability() {
    // scalar widths never widen implicitly, so int32 does not extend number
    let session = TestSession::single(
        r#"
type IsString = "id" extends string;
type IsNumber = int32 extends number;

const yes: IsString = true;
const no: IsNumber = false;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type IsString = "id" extends string;
type IsNumber = int32 extends number;

const yes: IsString = true;
const no: IsNumber = false;

=== checked ===
type IsString = "id" extends string;
/// @type.symbol symbol=IsString source="type IsString = \"id\" extends string" type="id" extends string ? true : false reduced=true
/// @definition.type symbol=IsString source="type IsString = \"id\" extends string" value="id" extends string ? true : false reduced=true

type IsNumber = int32 extends number;
/// @type.symbol symbol=IsNumber source="type IsNumber = int32 extends number" type=int32 extends float64 ? true : false reduced=false
/// @definition.type symbol=IsNumber source="type IsNumber = int32 extends number" value=int32 extends float64 ? true : false reduced=false

const yes: IsString = true;
/// @type.symbol symbol=yes source=yes type=IsString reduced=true
/// @resolution.pattern source=yes kind=binding target=yes
/// @resolution.name source=IsString target=IsString

const no: IsNumber = false;
/// @type.symbol symbol=no source=no type=IsNumber reduced=false
/// @resolution.pattern source=no kind=binding target=no
/// @resolution.name source=IsNumber target=IsNumber
"#,
    );
}

#[test]
fn test_void_and_unit_extend_each_other() {
    let session = TestSession::single(
        r#"
type VoidExtendsUnit = void extends ();
type UnitExtendsVoid = () extends void;

const left: VoidExtendsUnit = true;
const right: UnitExtendsVoid = true;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type VoidExtendsUnit = void extends ();
type UnitExtendsVoid = () extends void;

const left: VoidExtendsUnit = true;
const right: UnitExtendsVoid = true;

=== checked ===
type VoidExtendsUnit = void extends ();
/// @type.symbol symbol=VoidExtendsUnit source="type VoidExtendsUnit = void extends ()" type=void extends () ? true : false reduced=true
/// @definition.type symbol=VoidExtendsUnit source="type VoidExtendsUnit = void extends ()" value=void extends () ? true : false reduced=true

type UnitExtendsVoid = () extends void;
/// @type.symbol symbol=UnitExtendsVoid source="type UnitExtendsVoid = () extends void" type=() extends void ? true : false reduced=true
/// @definition.type symbol=UnitExtendsVoid source="type UnitExtendsVoid = () extends void" value=() extends void ? true : false reduced=true

const left: VoidExtendsUnit = true;
/// @type.symbol symbol=left source=left type=VoidExtendsUnit reduced=true
/// @resolution.pattern source=left kind=binding target=left
/// @resolution.name source=VoidExtendsUnit target=VoidExtendsUnit

const right: UnitExtendsVoid = true;
/// @type.symbol symbol=right source=right type=UnitExtendsVoid reduced=true
/// @resolution.pattern source=right kind=binding target=right
/// @resolution.name source=UnitExtendsVoid target=UnitExtendsVoid
"#,
    );
}

#[test]
fn test_never_extends_void_and_unit() {
    let session = TestSession::single(
        r#"
type NeverExtendsVoid = never extends void;
type NeverExtendsUnit = never extends ();

const left: NeverExtendsVoid = true;
const right: NeverExtendsUnit = true;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type NeverExtendsVoid = never extends void;
type NeverExtendsUnit = never extends ();

const left: NeverExtendsVoid = true;
const right: NeverExtendsUnit = true;

=== checked ===
type NeverExtendsVoid = never extends void;
/// @type.symbol symbol=NeverExtendsVoid source="type NeverExtendsVoid = never extends void" type=never extends void ? true : false reduced=true
/// @definition.type symbol=NeverExtendsVoid source="type NeverExtendsVoid = never extends void" value=never extends void ? true : false reduced=true

type NeverExtendsUnit = never extends ();
/// @type.symbol symbol=NeverExtendsUnit source="type NeverExtendsUnit = never extends ()" type=never extends () ? true : false reduced=true
/// @definition.type symbol=NeverExtendsUnit source="type NeverExtendsUnit = never extends ()" value=never extends () ? true : false reduced=true

const left: NeverExtendsVoid = true;
/// @type.symbol symbol=left source=left type=NeverExtendsVoid reduced=true
/// @resolution.pattern source=left kind=binding target=left
/// @resolution.name source=NeverExtendsVoid target=NeverExtendsVoid

const right: NeverExtendsUnit = true;
/// @type.symbol symbol=right source=right type=NeverExtendsUnit reduced=true
/// @resolution.pattern source=right kind=binding target=right
/// @resolution.name source=NeverExtendsUnit target=NeverExtendsUnit
"#,
    );
}

#[test]
fn test_void_and_unit_do_not_extend_never() {
    let session = TestSession::single(
        r#"
type VoidExtendsNever = void extends never;
type UnitExtendsNever = () extends never;

const left: VoidExtendsNever = false;
const right: UnitExtendsNever = false;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type VoidExtendsNever = void extends never;
type UnitExtendsNever = () extends never;

const left: VoidExtendsNever = false;
const right: UnitExtendsNever = false;

=== checked ===
type VoidExtendsNever = void extends never;
/// @type.symbol symbol=VoidExtendsNever source="type VoidExtendsNever = void extends never" type=void extends never ? true : false reduced=false
/// @definition.type symbol=VoidExtendsNever source="type VoidExtendsNever = void extends never" value=void extends never ? true : false reduced=false

type UnitExtendsNever = () extends never;
/// @type.symbol symbol=UnitExtendsNever source="type UnitExtendsNever = () extends never" type=() extends never ? true : false reduced=false
/// @definition.type symbol=UnitExtendsNever source="type UnitExtendsNever = () extends never" value=() extends never ? true : false reduced=false

const left: VoidExtendsNever = false;
/// @type.symbol symbol=left source=left type=VoidExtendsNever reduced=false
/// @resolution.pattern source=left kind=binding target=left
/// @resolution.name source=VoidExtendsNever target=VoidExtendsNever

const right: UnitExtendsNever = false;
/// @type.symbol symbol=right source=right type=UnitExtendsNever reduced=false
/// @resolution.pattern source=right kind=binding target=right
/// @resolution.name source=UnitExtendsNever target=UnitExtendsNever
"#,
    );
}

#[test]
fn test_implements_relation_evaluates_nominal_conformance() {
    let session = TestSession::single(
        r#"
interface Drawable {
    draw(): void;
}

struct DrawnPoint implements Drawable {
    x: int32;

    draw(): void {}
}

struct PlainPoint {
    x: int32;
}

type IsDrawn = DrawnPoint implements Drawable;
type IsPlain = PlainPoint implements Drawable;

const drawn: IsDrawn = true;
const plain: IsPlain = false;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Drawable {
    draw(): void;
}

struct DrawnPoint implements Drawable {
    x: int32;

    draw(): void {}
}

struct PlainPoint {
    x: int32;
}

type IsDrawn = DrawnPoint implements Drawable;
type IsPlain = PlainPoint implements Drawable;

const drawn: IsDrawn = true;
const plain: IsPlain = false;

=== checked ===
interface Drawable {
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable
/// @definition.method symbol=Drawable.draw source="draw(): void" slot=draw type=(this: this) => void

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=(this: this) => void

}

struct DrawnPoint implements Drawable {
/// @type.symbol symbol=DrawnPoint type=DrawnPoint
/// @definition.struct symbol=DrawnPoint
/// @definition.where symbol=DrawnPoint source=Drawable relation=satisfies left=this right=Drawable
/// @definition.implements symbol=DrawnPoint source=Drawable target=Drawable
/// @definition.field symbol=DrawnPoint.x source="x: int32" key=x type=int32
/// @definition.method symbol=DrawnPoint.draw source="draw(): void {}" slot=draw type=<DrawnPoint.draw.'l0>(this: &DrawnPoint.draw.'l0 exclusive this) => void
/// @resolution.name source=Drawable target=Drawable

    x: int32;
    /// @type.symbol symbol=DrawnPoint.x source="x: int32" type=int32

    draw(): void {}
    /// @generic.template symbol=DrawnPoint.draw parent=template#1 parameters=('l0)
    /// @type.symbol symbol=DrawnPoint.draw source="draw(): void {}" type=<DrawnPoint.draw.'l0>(this: &DrawnPoint.draw.'l0 exclusive this) => void

}

struct PlainPoint {
/// @type.symbol symbol=PlainPoint type=PlainPoint
/// @definition.struct symbol=PlainPoint
/// @definition.field symbol=PlainPoint.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=PlainPoint.x source="x: int32" type=int32

}

type IsDrawn = DrawnPoint implements Drawable;
/// @type.symbol symbol=IsDrawn source="type IsDrawn = DrawnPoint implements Drawable" type=DrawnPoint extends Drawable ? true : false reduced=true
/// @definition.type symbol=IsDrawn source="type IsDrawn = DrawnPoint implements Drawable" value=DrawnPoint extends Drawable ? true : false reduced=true
/// @resolution.name source=DrawnPoint target=DrawnPoint
/// @resolution.name source=Drawable target=Drawable

type IsPlain = PlainPoint implements Drawable;
/// @type.symbol symbol=IsPlain source="type IsPlain = PlainPoint implements Drawable" type=PlainPoint extends Drawable ? true : false reduced=false
/// @definition.type symbol=IsPlain source="type IsPlain = PlainPoint implements Drawable" value=PlainPoint extends Drawable ? true : false reduced=false
/// @resolution.name source=PlainPoint target=PlainPoint
/// @resolution.name source=Drawable target=Drawable

const drawn: IsDrawn = true;
/// @type.symbol symbol=drawn source=drawn type=IsDrawn reduced=true
/// @resolution.pattern source=drawn kind=binding target=drawn
/// @resolution.name source=IsDrawn target=IsDrawn

const plain: IsPlain = false;
/// @type.symbol symbol=plain source=plain type=IsPlain reduced=false
/// @resolution.pattern source=plain kind=binding target=plain
/// @resolution.name source=IsPlain target=IsPlain
"#,
    );
}
