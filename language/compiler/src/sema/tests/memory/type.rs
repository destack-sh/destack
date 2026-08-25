use crate::tests::{DirRows, TestSession};

#[test]
fn test_with_access_sets_readonly_and_exclusive_access() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type ReadonlyOwned = WithAccess<^Cell, "readonly">;
type ExclusiveBorrow = WithAccess<Borrowed<Cell, "static">, "exclusive">;

declare const readonlyOwned: ReadonlyOwned;
declare const exclusiveBorrow: ExclusiveBorrow;

readonlyOwned satisfies ^readonly Cell;
exclusiveBorrow satisfies Borrowed<Cell, "static", "exclusive">;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type ReadonlyOwned = WithAccess<^Cell, "readonly">;
type ExclusiveBorrow = WithAccess<Borrowed<Cell, "static">, "exclusive">;

declare const readonlyOwned: readonly Cell;
declare const exclusiveBorrow: Borrowed<Cell, "static", "exclusive">;

readonlyOwned satisfies ^readonly Cell;
exclusiveBorrow satisfies Borrowed<Cell, "static", "exclusive">;

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type ReadonlyOwned = WithAccess<^Cell, "readonly">;
/// @type.symbol symbol=ReadonlyOwned source="type ReadonlyOwned = WithAccess<^Cell, \"readonly\">" type=Readonly<Cell>
/// @definition.type symbol=ReadonlyOwned source="type ReadonlyOwned = WithAccess<^Cell, \"readonly\">" value=Readonly<Cell>
/// @resolution.name source=WithAccess target=WithAccess
/// @resolution.name source=Cell target=Cell

type ExclusiveBorrow = WithAccess<Borrowed<Cell, "static">, "exclusive">;
/// @type.symbol symbol=ExclusiveBorrow source="type ExclusiveBorrow = WithAccess<Borrowed<Cell, \"static\">, \"exclusive\">" type=&'static exclusive Cell
/// @definition.type symbol=ExclusiveBorrow source="type ExclusiveBorrow = WithAccess<Borrowed<Cell, \"static\">, \"exclusive\">" value=&'static exclusive Cell
/// @resolution.name source=WithAccess target=WithAccess
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Cell target=Cell

declare const readonlyOwned: ReadonlyOwned;
/// @type.symbol symbol=readonlyOwned source=readonlyOwned type=Readonly<Cell>
/// @resolution.pattern source=readonlyOwned kind=binding target=readonlyOwned
/// @resolution.name source=ReadonlyOwned target=ReadonlyOwned

declare const exclusiveBorrow: ExclusiveBorrow;
/// @type.symbol symbol=exclusiveBorrow source=exclusiveBorrow type=&'static exclusive Cell
/// @resolution.pattern source=exclusiveBorrow kind=binding target=exclusiveBorrow
/// @resolution.name source=ExclusiveBorrow target=ExclusiveBorrow

readonlyOwned satisfies ^readonly Cell;
/// @resolution.name source=readonlyOwned target=readonlyOwned
/// @resolution.place source=readonlyOwned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=readonlyOwned root=readonlyOwned
/// @resolution.name source=Cell target=Cell

exclusiveBorrow satisfies Borrowed<Cell, "static", "exclusive">;
/// @resolution.name source=exclusiveBorrow target=exclusiveBorrow
/// @resolution.place source=exclusiveBorrow placement="static" lifetime="static" access="exclusive"
/// @resolution.access source=exclusiveBorrow root=exclusiveBorrow
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Cell target=Cell
"#,
    );
}

#[test]
fn test_clamp_borrow_access_over_readonly_payload() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type ReadonlyBorrow = Borrowed<Readonly<Cell>, "static">;

declare const borrow: ReadonlyBorrow;

borrow satisfies Borrowed<Cell, "static", "readonly">;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type ReadonlyBorrow = Borrowed<Readonly<Cell>, "static">;

declare const borrow: Borrowed<Cell, "static", "readonly">;

borrow satisfies Borrowed<Cell, "static", "readonly">;

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type ReadonlyBorrow = Borrowed<Readonly<Cell>, "static">;
/// @type.symbol symbol=ReadonlyBorrow source="type ReadonlyBorrow = Borrowed<Readonly<Cell>, \"static\">" type=&'static readonly Cell
/// @definition.type symbol=ReadonlyBorrow source="type ReadonlyBorrow = Borrowed<Readonly<Cell>, \"static\">" value=&'static readonly Cell
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Readonly target=Readonly
/// @resolution.name source=Cell target=Cell

declare const borrow: ReadonlyBorrow;
/// @type.symbol symbol=borrow source=borrow type=&'static readonly Cell
/// @resolution.pattern source=borrow kind=binding target=borrow
/// @resolution.name source=ReadonlyBorrow target=ReadonlyBorrow

borrow satisfies Borrowed<Cell, "static", "readonly">;
/// @resolution.name source=borrow target=borrow
/// @resolution.place source=borrow placement="static" lifetime="static" access="readonly"
/// @resolution.access source=borrow root=borrow
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Cell target=Cell
"#,
    );
}
