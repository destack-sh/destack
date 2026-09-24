use crate::tests::{DirRows, TestSession};

#[test]
fn test_set_readonly_and_mutable_access_through_with_access() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type ReadonlyOwned = WithAccess<^Cell, "readonly">;
type ExclusiveBorrow = WithAccess<Borrowed<Cell, "static">, "mutable">;

declare const readonlyOwned: ReadonlyOwned;
declare const exclusiveBorrow: ExclusiveBorrow;

readonlyOwned satisfies ^readonly Cell;
exclusiveBorrow satisfies Borrowed<Cell, "static", "mutable">;
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
type ExclusiveBorrow = WithAccess<Borrowed<Cell, "static">, "mutable">;

declare const readonlyOwned: ReadonlyOwned;
declare const exclusiveBorrow: ExclusiveBorrow;

readonlyOwned satisfies ^readonly Cell;
exclusiveBorrow satisfies Borrowed<Cell, "static", "mutable">;

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type ReadonlyOwned = WithAccess<^Cell, "readonly">;
/// @type.symbol symbol=ReadonlyOwned source="type ReadonlyOwned = WithAccess<^Cell, \"readonly\">" type=readonly Cell
/// @generic.instance id="WithAccess<Cell, \"readonly\">" template=WithAccess arguments=(Cell, "readonly")
/// @definition.type symbol=ReadonlyOwned source="type ReadonlyOwned = WithAccess<^Cell, \"readonly\">" value=WithAccess<Cell, "readonly">
/// @resolution.name source=WithAccess target=WithAccess
/// @resolution.name source=Cell target=Cell

type ExclusiveBorrow = WithAccess<Borrowed<Cell, "static">, "mutable">;
/// @type.symbol symbol=ExclusiveBorrow source="type ExclusiveBorrow = WithAccess<Borrowed<Cell, \"static\">, \"mutable\">" type=&'static Cell
/// @generic.instance id="WithAccess<&'bound0 Cell, \"mutable\">" template=WithAccess arguments=(&'bound0 Cell, "mutable")
/// @definition.type symbol=ExclusiveBorrow source="type ExclusiveBorrow = WithAccess<Borrowed<Cell, \"static\">, \"mutable\">" value=WithAccess<&'static Cell, "mutable">
/// @resolution.name source=WithAccess target=WithAccess
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Cell target=Cell

declare const readonlyOwned: ReadonlyOwned;
/// @type.symbol symbol=readonlyOwned source=readonlyOwned type=ReadonlyOwned
/// @resolution.pattern source=readonlyOwned kind=binding target=readonlyOwned
/// @resolution.name source=ReadonlyOwned target=ReadonlyOwned

declare const exclusiveBorrow: ExclusiveBorrow;
/// @type.symbol symbol=exclusiveBorrow source=exclusiveBorrow type=ExclusiveBorrow
/// @resolution.pattern source=exclusiveBorrow kind=binding target=exclusiveBorrow
/// @resolution.name source=ExclusiveBorrow target=ExclusiveBorrow

readonlyOwned satisfies ^readonly Cell;
/// @resolution.name source=readonlyOwned target=readonlyOwned
/// @resolution.place source=readonlyOwned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=readonlyOwned root=readonlyOwned
/// @resolution.name source=Cell target=Cell

exclusiveBorrow satisfies Borrowed<Cell, "static", "mutable">;
/// @resolution.name source=exclusiveBorrow target=exclusiveBorrow
/// @resolution.place source=exclusiveBorrow placement="local" lifetime="static" access="immutable"
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

declare const borrow: ReadonlyBorrow;

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
/// @type.symbol symbol=borrow source=borrow type=ReadonlyBorrow
/// @resolution.pattern source=borrow kind=binding target=borrow
/// @resolution.name source=ReadonlyBorrow target=ReadonlyBorrow

borrow satisfies Borrowed<Cell, "static", "readonly">;
/// @resolution.name source=borrow target=borrow
/// @resolution.place source=borrow placement="local" lifetime="static" access="immutable"
/// @resolution.access source=borrow root=borrow
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Cell target=Cell
"#,
    );
}
