use crate::tests::{DirRows, TestSession};

#[test]
fn test_extends_relation_evaluates_assignability() {
    let session = TestSession::single(
        r#"
type IsNumber = int32 extends number;
type IsString = string extends int32;

const yes: IsNumber = true;
const no: IsString = false;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type IsNumber = int32 extends number;
type IsString = string extends int32;

const yes: IsNumber = true;
const no: IsString = false;

=== checked ===
type IsNumber = int32 extends number;
/// @type.symbol symbol=IsNumber source="type IsNumber = int32 extends number" type=true
/// @definition.type symbol=IsNumber source="type IsNumber = int32 extends number" value=true

type IsString = string extends int32;
/// @type.symbol symbol=IsString source="type IsString = string extends int32" type=false
/// @definition.type symbol=IsString source="type IsString = string extends int32" value=false

const yes: IsNumber = true;
/// @type.symbol symbol=yes source=yes type=true
/// @resolution.name source=IsNumber target=IsNumber

const no: IsString = false;
/// @type.symbol symbol=no source=no type=false
/// @resolution.name source=IsString target=IsString
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
/// @type.symbol symbol=VoidExtendsUnit source="type VoidExtendsUnit = void extends ()" type=true
/// @definition.type symbol=VoidExtendsUnit source="type VoidExtendsUnit = void extends ()" value=true

type UnitExtendsVoid = () extends void;
/// @type.symbol symbol=UnitExtendsVoid source="type UnitExtendsVoid = () extends void" type=true
/// @definition.type symbol=UnitExtendsVoid source="type UnitExtendsVoid = () extends void" value=true

const left: VoidExtendsUnit = true;
/// @type.symbol symbol=left source=left type=true
/// @resolution.name source=VoidExtendsUnit target=VoidExtendsUnit

const right: UnitExtendsVoid = true;
/// @type.symbol symbol=right source=right type=true
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
/// @type.symbol symbol=NeverExtendsVoid source="type NeverExtendsVoid = never extends void" type=true
/// @definition.type symbol=NeverExtendsVoid source="type NeverExtendsVoid = never extends void" value=true

type NeverExtendsUnit = never extends ();
/// @type.symbol symbol=NeverExtendsUnit source="type NeverExtendsUnit = never extends ()" type=true
/// @definition.type symbol=NeverExtendsUnit source="type NeverExtendsUnit = never extends ()" value=true

const left: NeverExtendsVoid = true;
/// @type.symbol symbol=left source=left type=true
/// @resolution.name source=NeverExtendsVoid target=NeverExtendsVoid

const right: NeverExtendsUnit = true;
/// @type.symbol symbol=right source=right type=true
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
/// @type.symbol symbol=VoidExtendsNever source="type VoidExtendsNever = void extends never" type=false
/// @definition.type symbol=VoidExtendsNever source="type VoidExtendsNever = void extends never" value=false

type UnitExtendsNever = () extends never;
/// @type.symbol symbol=UnitExtendsNever source="type UnitExtendsNever = () extends never" type=false
/// @definition.type symbol=UnitExtendsNever source="type UnitExtendsNever = () extends never" value=false

const left: VoidExtendsNever = false;
/// @type.symbol symbol=left source=left type=false
/// @resolution.name source=VoidExtendsNever target=VoidExtendsNever

const right: UnitExtendsNever = false;
/// @type.symbol symbol=right source=right type=false
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

    draw(): void;
    /// @type.symbol symbol=Drawable.draw type=() => void
}

struct DrawnPoint implements Drawable {
/// @type.symbol symbol=DrawnPoint type=DrawnPoint
/// @definition.struct symbol=DrawnPoint
/// @resolution.name source=Drawable target=Drawable

    x: int32;
    /// @type.symbol symbol=DrawnPoint.x source="x: int32" type=int32

    draw(): void {}
    /// @type.symbol symbol=DrawnPoint.draw type=(this: DrawnPoint) => void
}

struct PlainPoint {
/// @type.symbol symbol=PlainPoint type=PlainPoint
/// @definition.struct symbol=PlainPoint

    x: int32;
    /// @type.symbol symbol=PlainPoint.x source="x: int32" type=int32
}

type IsDrawn = DrawnPoint implements Drawable;
/// @type.symbol symbol=IsDrawn source="type IsDrawn = DrawnPoint implements Drawable" type=true
/// @definition.type symbol=IsDrawn source="type IsDrawn = DrawnPoint implements Drawable" value=true
/// @resolution.name source=DrawnPoint target=DrawnPoint
/// @resolution.name source=Drawable target=Drawable

type IsPlain = PlainPoint implements Drawable;
/// @type.symbol symbol=IsPlain source="type IsPlain = PlainPoint implements Drawable" type=false
/// @definition.type symbol=IsPlain source="type IsPlain = PlainPoint implements Drawable" value=false
/// @resolution.name source=PlainPoint target=PlainPoint
/// @resolution.name source=Drawable target=Drawable

const drawn: IsDrawn = true;
/// @type.symbol symbol=drawn source=drawn type=true
/// @resolution.name source=IsDrawn target=IsDrawn

const plain: IsPlain = false;
/// @type.symbol symbol=plain source=plain type=false
/// @resolution.name source=IsPlain target=IsPlain
"#,
    );
}
