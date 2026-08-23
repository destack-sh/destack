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
        "main.ds",
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
        "main.ds",
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
/// @diagnostic.error id=constraint-not-satisfied message="type 'static' does not satisfy 'Position'"
/// @diagnostic.label line=11 column=15 span="anchor<ZERO>()" line_source="const value = anchor<ZERO>();"
/// @diagnostic.related line=9 column=31 span="P" line_source="declare function anchor<const P: Position>(): int32;" message="required by this bound on 'P'"
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
        "main.ds",
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
