use crate::tests::{DirRows, TestSession};

/// Derive an exported const's type from its struct literal initializer.
#[test]
fn test_derive_export_type_from_struct_literal() {
    let session = TestSession::single(
        r#"
struct Position {
    x: int32;
    y: int32;
}

export const ZERO = Position { x: 0, y: 0 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Position {
    x: int32;
    y: int32;
}

export const ZERO: Position = Position { x: 0, y: 0 };

=== dir ===
struct Position {
/// @type.symbol symbol=Position type=Position
/// @definition.struct symbol=Position
/// @definition.field symbol=Position.x source="x: int32" key=x type=int32
/// @definition.field symbol=Position.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Position.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Position.y source="y: int32" type=int32

}

export const ZERO = Position { x: 0, y: 0 };
/// @type.symbol symbol=ZERO source=ZERO type=Position
/// @resolution.pattern source=ZERO kind=binding target=ZERO
/// @resolution.name source=Position target=Position
"#,
        r#"
"#,
    );
}

/// Bind a struct const as a const generic argument.
#[test]
fn test_bind_struct_const_as_const_generic_argument() {
    let session = TestSession::single(
        r#"
struct Position {
    x: int32;
    y: int32;
}

const ZERO = Position { x: 0, y: 0 };

declare function anchor<const P: Position>(): int32;

const value = anchor<ZERO>();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Position {
    x: int32;
    y: int32;
}

const ZERO: Position = Position { x: 0, y: 0 };

declare function anchor<const P: Position>(): int32;

const value: int32 = anchor<ZERO>();

=== dir ===
struct Position {
/// @type.symbol symbol=Position type=Position
/// @definition.struct symbol=Position
/// @definition.field symbol=Position.x source="x: int32" key=x type=int32
/// @definition.field symbol=Position.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Position.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Position.y source="y: int32" type=int32

}

const ZERO = Position { x: 0, y: 0 };
/// @type.symbol symbol=ZERO source=ZERO type=Position
/// @resolution.pattern source=ZERO kind=binding target=ZERO
/// @resolution.name source=Position target=Position

declare function anchor<const P: Position>(): int32;
/// @generic.template symbol=anchor parameters=(const P: Position)
/// @type.symbol symbol=anchor source="declare function anchor<const P: Position>(): int32" type=<const P: Position>() => int32
/// @type.symbol symbol=anchor.P source="const P: Position" type=P
/// @resolution.name source=Position target=Position

const value = anchor<ZERO>();
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=anchor target=anchor
/// @resolution.call source=anchor<ZERO>() parameters=() return=int32 kind=symbol target=anchor instance="anchor<Position { x: 0; y: 0 }>"
/// @generic.instantiation id="anchor<Position { x: 0; y: 0 }>" template=anchor arguments=(Position { x: 0; y: 0 })
/// @resolution.name source=ZERO target=ZERO
"#,
        r#"
"#,
    );
}

/// Share one instantiation across struct consts written in different field orders.
#[test]
fn test_share_instantiation_across_field_orders() {
    let session = TestSession::single(
        r#"
struct Position {
    x: int32;
    y: int32;
}

const ZERO = Position { x: 0, y: 0 };
const SWAPPED = Position { y: 0, x: 0 };

declare function anchor<const P: Position>(): int32;

const left = anchor<ZERO>();
const right = anchor<SWAPPED>();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Position {
    x: int32;
    y: int32;
}

const ZERO: Position = Position { x: 0, y: 0 };
const SWAPPED: Position = Position { y: 0, x: 0 };

declare function anchor<const P: Position>(): int32;

const left: int32 = anchor<ZERO>();
const right: int32 = anchor<SWAPPED>();

=== dir ===
struct Position {
/// @type.symbol symbol=Position type=Position
/// @definition.struct symbol=Position
/// @definition.field symbol=Position.x source="x: int32" key=x type=int32
/// @definition.field symbol=Position.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Position.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Position.y source="y: int32" type=int32

}

const ZERO = Position { x: 0, y: 0 };
/// @type.symbol symbol=ZERO source=ZERO type=Position
/// @resolution.pattern source=ZERO kind=binding target=ZERO
/// @resolution.name source=Position target=Position

const SWAPPED = Position { y: 0, x: 0 };
/// @type.symbol symbol=SWAPPED source=SWAPPED type=Position
/// @resolution.pattern source=SWAPPED kind=binding target=SWAPPED
/// @resolution.name source=Position target=Position

declare function anchor<const P: Position>(): int32;
/// @generic.template symbol=anchor parameters=(const P: Position)
/// @type.symbol symbol=anchor source="declare function anchor<const P: Position>(): int32" type=<const P: Position>() => int32
/// @type.symbol symbol=anchor.P source="const P: Position" type=P
/// @resolution.name source=Position target=Position

const left = anchor<ZERO>();
/// @type.symbol symbol=left source=left type=int32
/// @resolution.pattern source=left kind=binding target=left
/// @resolution.name source=anchor target=anchor
/// @resolution.call source=anchor<ZERO>() parameters=() return=int32 kind=symbol target=anchor instance="anchor<Position { x: 0; y: 0 }>"
/// @generic.instantiation id="anchor<Position { x: 0; y: 0 }>" template=anchor arguments=(Position { x: 0; y: 0 })
/// @resolution.name source=ZERO target=ZERO

const right = anchor<SWAPPED>();
/// @type.symbol symbol=right source=right type=int32
/// @resolution.pattern source=right kind=binding target=right
/// @resolution.name source=anchor target=anchor
/// @resolution.call source=anchor<SWAPPED>() parameters=() return=int32 kind=symbol target=anchor instance="anchor<Position { x: 0; y: 0 }>"
/// @generic.instantiation id="anchor<Position { x: 0; y: 0 }>" template=anchor arguments=(Position { x: 0; y: 0 })
/// @resolution.name source=SWAPPED target=SWAPPED
"#,
        r#"
"#,
    );
}

/// Bind a scalar const as a const generic argument.
#[test]
fn test_bind_scalar_const_as_const_generic_argument() {
    let session = TestSession::single(
        r#"
const SIZE = 3;

declare function anchor<const N: int32>(): int32;

const value = anchor<SIZE>();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const SIZE: 3 = 3;

declare function anchor<const N: int32>(): int32;

const value: int32 = anchor<SIZE>();

=== dir ===
const SIZE = 3;
/// @type.symbol symbol=SIZE source=SIZE type=3
/// @resolution.pattern source=SIZE kind=binding target=SIZE

declare function anchor<const N: int32>(): int32;
/// @generic.template symbol=anchor parameters=(const N: int32)
/// @type.symbol symbol=anchor source="declare function anchor<const N: int32>(): int32" type=<const N: int32>() => int32
/// @type.symbol symbol=anchor.N source="const N: int32" type=N

const value = anchor<SIZE>();
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=anchor target=anchor
/// @resolution.call source=anchor<SIZE>() parameters=() return=int32 kind=symbol target=anchor instance=anchor<3>
/// @generic.instantiation id=anchor<3> template=anchor arguments=(3)
/// @resolution.name source=SIZE target=SIZE
"#,
        r#"
"#,
    );
}

/// Reject a value binding written as a plain type annotation.
#[test]
fn test_reject_value_binding_as_annotation() {
    let session = TestSession::single(
        r#"
struct Position {
    x: int32;
}

const ZERO = Position { x: 0 };

declare const probe: ZERO;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Position {
    x: int32;
}

const ZERO: Position = Position { x: 0 };

declare const probe: ZERO;

=== dir ===
struct Position {
/// @type.symbol symbol=Position type=Position
/// @definition.struct symbol=Position
/// @definition.field symbol=Position.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Position.x source="x: int32" type=int32

}

const ZERO = Position { x: 0 };
/// @type.symbol symbol=ZERO source=ZERO type=Position
/// @resolution.pattern source=ZERO kind=binding target=ZERO
/// @resolution.name source=Position target=Position

declare const probe: ZERO;
/// @type.symbol symbol=probe source=probe type=<error>
/// @resolution.pattern source=probe kind=binding target=probe
/// @resolution.name source=ZERO target=ZERO
"#,
        r#"
/// @diagnostic.error id=value-used-as-type message="expected a type, found value 'ZERO'"
/// @diagnostic.label line=8 column=22 span="ZERO" line_source="declare const probe: ZERO;"
"#,
    );
}

/// Type an associated const by the newtype literal its value writes.
#[test]
fn test_type_an_associated_const_from_its_newtype_literal() {
    let session = TestSession::single(
        r#"
newtype Mask = uint32;

export extension of Mask {
    const read = Mask(0x1);
}

export function readable(): Mask {
    return Mask.read;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Mask = uint32;

export extension of Mask {
    const read = Mask(0x1);
}

export function readable(): Mask {
    return Mask.read;
}

=== dir ===
newtype Mask = uint32;
/// @type.symbol symbol=Mask source="newtype Mask = uint32" type=Mask
/// @definition.newtype symbol=Mask source="newtype Mask = uint32" backing=uint32 constructors=[(uint32) => Mask]

export extension of Mask {
/// @definition.extension symbol=<module>#2 form=exported target=Mask
/// @definition.associated.const symbol=read source="const read = Mask(0x1)" key=read type=Mask
/// @resolution.name source=Mask target=Mask

    const read = Mask(0x1);
    /// @type.symbol symbol=read source="const read = Mask(0x1)" type=Mask
    /// @resolution.name source=Mask target=Mask

}

export function readable(): Mask {
/// @type.symbol symbol=readable type=() => Mask
/// @resolution.name source=Mask target=Mask

    return Mask.read;
    /// @resolution.name source=Mask target=Mask
    /// @resolution.member source=Mask.read receiver=Mask type=Mask kind=symbol target_receiver=Mask target=read

}
"#,
        r#"
"#,
    );
}
