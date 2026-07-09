use crate::tests::{DirRows, TestSession};

#[test]
fn test_read_base_and_payload_from_owned_borrow() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type OwnedBorrow = Owned<Borrowed<Cell, "static">>;
type Base = BaseOf<OwnedBorrow>;
type Payload = PayloadOf<OwnedBorrow>;

declare const base: Base;
declare const payload: Payload;

base satisfies Cell;
payload satisfies Borrowed<Cell, "static">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type OwnedBorrow = Owned<Borrowed<Cell, "static">>;
type Base = BaseOf<OwnedBorrow>;
type Payload = PayloadOf<OwnedBorrow>;

declare const base: Base;
declare const payload: Payload;

base satisfies Cell;
payload satisfies Borrowed<Cell, "static">;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type OwnedBorrow = Owned<Borrowed<Cell, "static">>;
/// @type.symbol symbol=OwnedBorrow source="type OwnedBorrow = Owned<Borrowed<Cell, \"static\">>" type=Owned<Borrowed<Cell, "static", "mutable">>
/// @definition.type symbol=OwnedBorrow source="type OwnedBorrow = Owned<Borrowed<Cell, \"static\">>" value=Owned<Borrowed<Cell, "static", "mutable">>
/// @resolution.name source=Owned target=memory.owned.Owned
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

type Base = BaseOf<OwnedBorrow>;
/// @type.symbol symbol=Base source="type Base = BaseOf<OwnedBorrow>" type=BaseOf<OwnedBorrow> reduced=Cell
/// @definition.type symbol=Base source="type Base = BaseOf<OwnedBorrow>" value=BaseOf<OwnedBorrow> reduced=Cell
/// @resolution.name source=BaseOf target=memory.type.BaseOf
/// @resolution.name source=OwnedBorrow target=OwnedBorrow

type Payload = PayloadOf<OwnedBorrow>;
/// @type.symbol symbol=Payload source="type Payload = PayloadOf<OwnedBorrow>" type=PayloadOf<OwnedBorrow> reduced=Borrowed<Cell, "static", "mutable">
/// @definition.type symbol=Payload source="type Payload = PayloadOf<OwnedBorrow>" value=PayloadOf<OwnedBorrow> reduced=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=PayloadOf target=memory.type.PayloadOf
/// @resolution.name source=OwnedBorrow target=OwnedBorrow

declare const base: Base;
/// @type.symbol symbol=base source=base type=Base reduced=Cell
/// @resolution.name source=Base target=Base

declare const payload: Payload;
/// @type.symbol symbol=payload source=payload type=Payload reduced=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=Payload target=Payload

base satisfies Cell;
/// @resolution.name source=base target=base
/// @resolution.name source=Cell target=Cell

payload satisfies Borrowed<Cell, "static">;
/// @resolution.name source=payload target=payload
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="Borrowed<Cell, \"static\", \"mutable\">" template=memory.borrow.Borrowed arguments=(Cell, "static", "mutable")
/// @generic.instance id="Owned<Borrowed<Cell, \"static\", \"mutable\">>" template=memory.owned.Owned arguments=(Borrowed<Cell, "static", "mutable">)
/// @generic.instance id=BaseOf<OwnedBorrow> template=memory.type.BaseOf arguments=(OwnedBorrow)
/// @generic.instance id=PayloadOf<OwnedBorrow> template=memory.type.PayloadOf arguments=(OwnedBorrow)
"#,
    );
}

#[test]
fn test_read_lifetime_and_access_from_borrow() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type BorrowOwned = Borrowed<Owned<Cell>, "static">;
type BorrowedLifetime = LifetimeOf<BorrowOwned>;
type BorrowedAccess = AccessOf<BorrowOwned>;

declare const borrowedLifetime: BorrowedLifetime;
declare const borrowedAccess: BorrowedAccess;

borrowedLifetime satisfies "static";
borrowedAccess satisfies "mutable";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type BorrowOwned = Borrowed<Owned<Cell>, "static">;
type BorrowedLifetime = LifetimeOf<BorrowOwned>;
type BorrowedAccess = AccessOf<BorrowOwned>;

declare const borrowedLifetime: BorrowedLifetime;
declare const borrowedAccess: BorrowedAccess;

borrowedLifetime satisfies "static";
borrowedAccess satisfies "mutable";

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type BorrowOwned = Borrowed<Owned<Cell>, "static">;
/// @type.symbol symbol=BorrowOwned source="type BorrowOwned = Borrowed<Owned<Cell>, \"static\">" type=Borrowed<Owned<Cell>, "static", "mutable"> reduced=Borrowed<Cell, "static", "mutable">
/// @definition.type symbol=BorrowOwned source="type BorrowOwned = Borrowed<Owned<Cell>, \"static\">" value=Borrowed<Owned<Cell>, "static", "mutable"> reduced=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Owned target=memory.owned.Owned
/// @resolution.name source=Cell target=Cell

type BorrowedLifetime = LifetimeOf<BorrowOwned>;
/// @type.symbol symbol=BorrowedLifetime source="type BorrowedLifetime = LifetimeOf<BorrowOwned>" type=LifetimeOf<BorrowOwned> reduced="static"
/// @definition.type symbol=BorrowedLifetime source="type BorrowedLifetime = LifetimeOf<BorrowOwned>" value=LifetimeOf<BorrowOwned> reduced="static"
/// @resolution.name source=LifetimeOf target=memory.type.LifetimeOf
/// @resolution.name source=BorrowOwned target=BorrowOwned

type BorrowedAccess = AccessOf<BorrowOwned>;
/// @type.symbol symbol=BorrowedAccess source="type BorrowedAccess = AccessOf<BorrowOwned>" type=AccessOf<BorrowOwned> reduced="mutable"
/// @definition.type symbol=BorrowedAccess source="type BorrowedAccess = AccessOf<BorrowOwned>" value=AccessOf<BorrowOwned> reduced="mutable"
/// @resolution.name source=AccessOf target=memory.type.AccessOf
/// @resolution.name source=BorrowOwned target=BorrowOwned

declare const borrowedLifetime: BorrowedLifetime;
/// @type.symbol symbol=borrowedLifetime source=borrowedLifetime type=BorrowedLifetime reduced="static"
/// @resolution.name source=BorrowedLifetime target=BorrowedLifetime

declare const borrowedAccess: BorrowedAccess;
/// @type.symbol symbol=borrowedAccess source=borrowedAccess type=BorrowedAccess reduced="mutable"
/// @resolution.name source=BorrowedAccess target=BorrowedAccess

borrowedLifetime satisfies "static";
/// @resolution.name source=borrowedLifetime target=borrowedLifetime

borrowedAccess satisfies "mutable";
/// @resolution.name source=borrowedAccess target=borrowedAccess

/// @generic.instance id="Borrowed<Owned<Cell>, \"static\", \"mutable\">" template=memory.borrow.Borrowed arguments=(Owned<Cell>, "static", "mutable")
/// @generic.instance id=AccessOf<BorrowOwned> template=memory.type.AccessOf arguments=(BorrowOwned)
/// @generic.instance id=LifetimeOf<BorrowOwned> template=memory.type.LifetimeOf arguments=(BorrowOwned)
/// @generic.instance id=Owned<Cell> template=memory.owned.Owned arguments=(Cell)
"#,
    );
}

#[test]
fn test_static_value_default_uses_type_expression_bridge() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type Reborrow<Q, comptime L: Lifetime = type(LifetimeOr<Q, "static">)> = Borrowed<Q, L>;
type StaticCell = Reborrow<Cell>;

declare const cell: StaticCell;

cell satisfies Borrowed<Cell, "static">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type Reborrow<Q, comptime L: Lifetime = LifetimeOr<Q, "static">> = Borrowed<Q, L>;
type StaticCell = Reborrow<Cell>;

declare const cell: StaticCell;

cell satisfies Borrowed<Cell, "static">;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type Reborrow<Q, comptime L: Lifetime = type(LifetimeOr<Q, "static">)> = Borrowed<Q, L>;
/// @generic.template symbol=Reborrow parameters=(Q, comptime L: Lifetime = LifetimeOr<Q, "static">)
/// @type.symbol symbol=Reborrow type=Borrowed<Q, L, "mutable">
/// @definition.type symbol=Reborrow template=(Q, comptime L: Lifetime = LifetimeOr<Q, "static">) value=Borrowed<Q, L, "mutable">
/// @type.symbol symbol=Reborrow.Q source=Q type=Q
/// @type.symbol symbol=Reborrow.L source="comptime L: Lifetime = type(LifetimeOr<Q, \"static\">)" type=L
/// @resolution.name source=Lifetime target=memory.lifetime.Lifetime
/// @resolution.name source=LifetimeOr target=memory.type.LifetimeOr
/// @resolution.name source=Q target=Reborrow.Q
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Q target=Reborrow.Q
/// @resolution.name source=L target=Reborrow.L

type StaticCell = Reborrow<Cell>;
/// @type.symbol symbol=StaticCell source="type StaticCell = Reborrow<Cell>" type=Reborrow<Cell, LifetimeOr<Cell, "static">> reduced=Borrowed<Cell, "static", "mutable">
/// @definition.type symbol=StaticCell source="type StaticCell = Reborrow<Cell>" value=Reborrow<Cell, LifetimeOr<Cell, "static">> reduced=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=Reborrow target=Reborrow
/// @resolution.name source=Cell target=Cell

declare const cell: StaticCell;
/// @type.symbol symbol=cell source=cell type=StaticCell reduced=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=StaticCell target=StaticCell

cell satisfies Borrowed<Cell, "static">;
/// @resolution.name source=cell target=cell
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="Borrowed<Q, L, \"mutable\">" template=memory.borrow.Borrowed arguments=(Q, L, "mutable")
/// @generic.instance id="LifetimeOr<Cell, \"static\">" template=memory.type.LifetimeOr arguments=(Cell, "static")
/// @generic.instance id="Reborrow<Cell, LifetimeOr<Cell, \"static\">>" template=Reborrow arguments=(Cell, LifetimeOr<Cell, "static">)
"#,
    );
}

#[test]
fn test_read_default_axes_from_plain_type() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type OwnershipDefault = OwnershipOr<Cell, "raw">;
type AccessDefault = AccessOr<Cell, "readonly">;
type PlaceDefault = PlaceOr<Cell, "shared">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type OwnershipDefault = OwnershipOr<Cell, "raw">;
type AccessDefault = AccessOr<Cell, "readonly">;
type PlaceDefault = PlaceOr<Cell, "shared">;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type OwnershipDefault = OwnershipOr<Cell, "raw">;
/// @type.symbol symbol=OwnershipDefault source="type OwnershipDefault = OwnershipOr<Cell, \"raw\">" type=OwnershipOr<Cell, "raw"> reduced="owned"
/// @definition.type symbol=OwnershipDefault source="type OwnershipDefault = OwnershipOr<Cell, \"raw\">" value=OwnershipOr<Cell, "raw"> reduced="owned"
/// @resolution.name source=OwnershipOr target=memory.type.OwnershipOr
/// @resolution.name source=Cell target=Cell

type AccessDefault = AccessOr<Cell, "readonly">;
/// @type.symbol symbol=AccessDefault source="type AccessDefault = AccessOr<Cell, \"readonly\">" type=AccessOr<Cell, "readonly"> reduced="mutable"
/// @definition.type symbol=AccessDefault source="type AccessDefault = AccessOr<Cell, \"readonly\">" value=AccessOr<Cell, "readonly"> reduced="mutable"
/// @resolution.name source=AccessOr target=memory.type.AccessOr
/// @resolution.name source=Cell target=Cell

type PlaceDefault = PlaceOr<Cell, "shared">;
/// @type.symbol symbol=PlaceDefault source="type PlaceDefault = PlaceOr<Cell, \"shared\">" type=PlaceOr<Cell, "shared"> reduced="ambient"
/// @definition.type symbol=PlaceDefault source="type PlaceDefault = PlaceOr<Cell, \"shared\">" value=PlaceOr<Cell, "shared"> reduced="ambient"
/// @resolution.name source=PlaceOr target=memory.type.PlaceOr
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="AccessOr<Cell, \"readonly\">" template=memory.type.AccessOr arguments=(Cell, "readonly")
/// @generic.instance id="OwnershipOr<Cell, \"raw\">" template=memory.type.OwnershipOr arguments=(Cell, "raw")
/// @generic.instance id="PlaceOr<Cell, \"shared\">" template=memory.type.PlaceOr arguments=(Cell, "shared")
"#,
    );
}

#[test]
fn test_lifetime_and_space_defaults_apply_to_plain_type() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type LifetimeFallback = LifetimeOr<Cell, "static">;
type SpaceFallback = SpaceOr<Cell, "shared">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type LifetimeFallback = LifetimeOr<Cell, "static">;
type SpaceFallback = SpaceOr<Cell, "shared">;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type LifetimeFallback = LifetimeOr<Cell, "static">;
/// @type.symbol symbol=LifetimeFallback source="type LifetimeFallback = LifetimeOr<Cell, \"static\">" type=LifetimeOr<Cell, "static"> reduced="static"
/// @definition.type symbol=LifetimeFallback source="type LifetimeFallback = LifetimeOr<Cell, \"static\">" value=LifetimeOr<Cell, "static"> reduced="static"
/// @resolution.name source=LifetimeOr target=memory.type.LifetimeOr
/// @resolution.name source=Cell target=Cell

type SpaceFallback = SpaceOr<Cell, "shared">;
/// @type.symbol symbol=SpaceFallback source="type SpaceFallback = SpaceOr<Cell, \"shared\">" type=SpaceOr<Cell, "shared"> reduced="shared"
/// @definition.type symbol=SpaceFallback source="type SpaceFallback = SpaceOr<Cell, \"shared\">" value=SpaceOr<Cell, "shared"> reduced="shared"
/// @resolution.name source=SpaceOr target=memory.type.SpaceOr
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="LifetimeOr<Cell, \"static\">" template=memory.type.LifetimeOr arguments=(Cell, "static")
/// @generic.instance id="SpaceOr<Cell, \"shared\">" template=memory.type.SpaceOr arguments=(Cell, "shared")
"#,
    );
}

#[test]
fn test_place_in_maps_ambient_place_to_requested_space() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type AmbientInShared = PlaceIn<Cell, "shared">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type AmbientInShared = PlaceIn<Cell, "shared">;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type AmbientInShared = PlaceIn<Cell, "shared">;
/// @type.symbol symbol=AmbientInShared source="type AmbientInShared = PlaceIn<Cell, \"shared\">" type=PlaceIn<Cell, "shared"> reduced="shared"
/// @definition.type symbol=AmbientInShared source="type AmbientInShared = PlaceIn<Cell, \"shared\">" value=PlaceIn<Cell, "shared"> reduced="shared"
/// @resolution.name source=PlaceIn target=memory.type.PlaceIn
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="PlaceIn<Cell, \"shared\">" template=memory.type.PlaceIn arguments=(Cell, "shared")
"#,
    );
}

#[test]
fn test_place_in_preserves_explicit_place() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type LocalInShared = PlaceIn<local Cell, "shared">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type LocalInShared = PlaceIn<local Cell, "shared">;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type LocalInShared = PlaceIn<local Cell, "shared">;
/// @type.symbol symbol=LocalInShared source="type LocalInShared = PlaceIn<local Cell, \"shared\">" type=PlaceIn<Placed<Cell, "local">, "shared"> reduced="local"
/// @definition.type symbol=LocalInShared source="type LocalInShared = PlaceIn<local Cell, \"shared\">" value=PlaceIn<Placed<Cell, "local">, "shared"> reduced="local"
/// @resolution.name source=PlaceIn target=memory.type.PlaceIn
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="PlaceIn<Placed<Cell, \"local\">, \"shared\">" template=memory.type.PlaceIn arguments=(Placed<Cell, "local">, "shared")
"#,
    );
}

#[test]
fn test_read_ownership_axis() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type ManagedKind = OwnershipOf<Managed<Cell>>;
type OwnedKind = OwnershipOf<Owned<Cell>>;
type BorrowedKind = OwnershipOf<Borrowed<Cell, "static">>;
type RawKind = OwnershipOf<Raw<Cell>>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type ManagedKind = OwnershipOf<Managed<Cell>>;
type OwnedKind = OwnershipOf<Owned<Cell>>;
type BorrowedKind = OwnershipOf<Borrowed<Cell, "static">>;
type RawKind = OwnershipOf<Raw<Cell>>;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type ManagedKind = OwnershipOf<Managed<Cell>>;
/// @type.symbol symbol=ManagedKind source="type ManagedKind = OwnershipOf<Managed<Cell>>" type=OwnershipOf<Managed<Cell>> reduced="managed"
/// @definition.type symbol=ManagedKind source="type ManagedKind = OwnershipOf<Managed<Cell>>" value=OwnershipOf<Managed<Cell>> reduced="managed"
/// @resolution.name source=OwnershipOf target=memory.type.OwnershipOf
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Cell target=Cell

type OwnedKind = OwnershipOf<Owned<Cell>>;
/// @type.symbol symbol=OwnedKind source="type OwnedKind = OwnershipOf<Owned<Cell>>" type=OwnershipOf<Owned<Cell>> reduced="owned"
/// @definition.type symbol=OwnedKind source="type OwnedKind = OwnershipOf<Owned<Cell>>" value=OwnershipOf<Owned<Cell>> reduced="owned"
/// @resolution.name source=OwnershipOf target=memory.type.OwnershipOf
/// @resolution.name source=Owned target=memory.owned.Owned
/// @resolution.name source=Cell target=Cell

type BorrowedKind = OwnershipOf<Borrowed<Cell, "static">>;
/// @type.symbol symbol=BorrowedKind source="type BorrowedKind = OwnershipOf<Borrowed<Cell, \"static\">>" type=OwnershipOf<Borrowed<Cell, "static", "mutable">> reduced="borrowed"
/// @definition.type symbol=BorrowedKind source="type BorrowedKind = OwnershipOf<Borrowed<Cell, \"static\">>" value=OwnershipOf<Borrowed<Cell, "static", "mutable">> reduced="borrowed"
/// @resolution.name source=OwnershipOf target=memory.type.OwnershipOf
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

type RawKind = OwnershipOf<Raw<Cell>>;
/// @type.symbol symbol=RawKind source="type RawKind = OwnershipOf<Raw<Cell>>" type=OwnershipOf<Raw<Cell>> reduced="raw"
/// @definition.type symbol=RawKind source="type RawKind = OwnershipOf<Raw<Cell>>" value=OwnershipOf<Raw<Cell>> reduced="raw"
/// @resolution.name source=OwnershipOf target=memory.type.OwnershipOf
/// @resolution.name source=Raw target=memory.raw.Raw
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="Borrowed<Cell, \"static\", \"mutable\">" template=memory.borrow.Borrowed arguments=(Cell, "static", "mutable")
/// @generic.instance id="OwnershipOf<Borrowed<Cell, \"static\", \"mutable\">>" template=memory.type.OwnershipOf arguments=(Borrowed<Cell, "static", "mutable">)
/// @generic.instance id=Managed<Cell> template=memory.managed.Managed arguments=(Cell)
/// @generic.instance id=Owned<Cell> template=memory.owned.Owned arguments=(Cell)
/// @generic.instance id=OwnershipOf<Managed<Cell>> template=memory.type.OwnershipOf arguments=(Managed<Cell>)
/// @generic.instance id=OwnershipOf<Owned<Cell>> template=memory.type.OwnershipOf arguments=(Owned<Cell>)
/// @generic.instance id=OwnershipOf<Raw<Cell>> template=memory.type.OwnershipOf arguments=(Raw<Cell>)
/// @generic.instance id=Raw<Cell> template=memory.raw.Raw arguments=(Cell)
"#,
    );
}

#[test]
fn test_match_ownership_predicates_to_ownership_axis() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type ManagedCheck = IsManaged<Managed<Cell>>;
type OwnedCheck = IsOwned<Owned<Cell>>;
type BorrowedCheck = IsBorrowed<Borrowed<Cell, "static">>;
type RawCheck = IsRaw<Raw<Cell>>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type ManagedCheck = IsManaged<Managed<Cell>>;
type OwnedCheck = IsOwned<Owned<Cell>>;
type BorrowedCheck = IsBorrowed<Borrowed<Cell, "static">>;
type RawCheck = IsRaw<Raw<Cell>>;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type ManagedCheck = IsManaged<Managed<Cell>>;
/// @type.symbol symbol=ManagedCheck source="type ManagedCheck = IsManaged<Managed<Cell>>" type=IsManaged<Managed<Cell>> reduced=true
/// @definition.type symbol=ManagedCheck source="type ManagedCheck = IsManaged<Managed<Cell>>" value=IsManaged<Managed<Cell>> reduced=true
/// @resolution.name source=IsManaged target=memory.type.IsManaged
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Cell target=Cell

type OwnedCheck = IsOwned<Owned<Cell>>;
/// @type.symbol symbol=OwnedCheck source="type OwnedCheck = IsOwned<Owned<Cell>>" type=IsOwned<Owned<Cell>> reduced=true
/// @definition.type symbol=OwnedCheck source="type OwnedCheck = IsOwned<Owned<Cell>>" value=IsOwned<Owned<Cell>> reduced=true
/// @resolution.name source=IsOwned target=memory.type.IsOwned
/// @resolution.name source=Owned target=memory.owned.Owned
/// @resolution.name source=Cell target=Cell

type BorrowedCheck = IsBorrowed<Borrowed<Cell, "static">>;
/// @type.symbol symbol=BorrowedCheck source="type BorrowedCheck = IsBorrowed<Borrowed<Cell, \"static\">>" type=IsBorrowed<Borrowed<Cell, "static", "mutable">> reduced=true
/// @definition.type symbol=BorrowedCheck source="type BorrowedCheck = IsBorrowed<Borrowed<Cell, \"static\">>" value=IsBorrowed<Borrowed<Cell, "static", "mutable">> reduced=true
/// @resolution.name source=IsBorrowed target=memory.type.IsBorrowed
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

type RawCheck = IsRaw<Raw<Cell>>;
/// @type.symbol symbol=RawCheck source="type RawCheck = IsRaw<Raw<Cell>>" type=IsRaw<Raw<Cell>> reduced=true
/// @definition.type symbol=RawCheck source="type RawCheck = IsRaw<Raw<Cell>>" value=IsRaw<Raw<Cell>> reduced=true
/// @resolution.name source=IsRaw target=memory.type.IsRaw
/// @resolution.name source=Raw target=memory.raw.Raw
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="Borrowed<Cell, \"static\", \"mutable\">" template=memory.borrow.Borrowed arguments=(Cell, "static", "mutable")
/// @generic.instance id="IsBorrowed<Borrowed<Cell, \"static\", \"mutable\">>" template=memory.type.IsBorrowed arguments=(Borrowed<Cell, "static", "mutable">)
/// @generic.instance id=IsManaged<Managed<Cell>> template=memory.type.IsManaged arguments=(Managed<Cell>)
/// @generic.instance id=IsOwned<Owned<Cell>> template=memory.type.IsOwned arguments=(Owned<Cell>)
/// @generic.instance id=IsRaw<Raw<Cell>> template=memory.type.IsRaw arguments=(Raw<Cell>)
/// @generic.instance id=Managed<Cell> template=memory.managed.Managed arguments=(Cell)
/// @generic.instance id=Owned<Cell> template=memory.owned.Owned arguments=(Cell)
/// @generic.instance id=Raw<Cell> template=memory.raw.Raw arguments=(Cell)
"#,
    );
}

#[test]
fn test_match_space_predicates_to_place_axis() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type SharedCheck = IsShared<shared Cell>;
type AmbientSharedCheck = IsSharedIn<Cell, "shared">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type SharedCheck = IsShared<shared Cell>;
type AmbientSharedCheck = IsSharedIn<Cell, "shared">;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type SharedCheck = IsShared<shared Cell>;
/// @type.symbol symbol=SharedCheck source="type SharedCheck = IsShared<shared Cell>" type=IsShared<Placed<Cell, "shared">> reduced=true
/// @definition.type symbol=SharedCheck source="type SharedCheck = IsShared<shared Cell>" value=IsShared<Placed<Cell, "shared">> reduced=true
/// @resolution.name source=IsShared target=memory.type.IsShared
/// @resolution.name source=Cell target=Cell

type AmbientSharedCheck = IsSharedIn<Cell, "shared">;
/// @type.symbol symbol=AmbientSharedCheck source="type AmbientSharedCheck = IsSharedIn<Cell, \"shared\">" type=IsSharedIn<Cell, "shared"> reduced=true
/// @definition.type symbol=AmbientSharedCheck source="type AmbientSharedCheck = IsSharedIn<Cell, \"shared\">" value=IsSharedIn<Cell, "shared"> reduced=true
/// @resolution.name source=IsSharedIn target=memory.type.IsSharedIn
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="IsShared<Placed<Cell, \"shared\">>" template=memory.type.IsShared arguments=(Placed<Cell, "shared">)
/// @generic.instance id="IsSharedIn<Cell, \"shared\">" template=memory.type.IsSharedIn arguments=(Cell, "shared")
"#,
    );
}

#[test]
fn test_with_ownership_builds_owned_and_borrowed_forms() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type OwnedCell = WithOwnership<Cell, "owned", "static">;
type BorrowedCell = WithOwnership<Cell, "borrowed", "static">;

declare const ownedCell: OwnedCell;
declare const borrowedCell: BorrowedCell;

ownedCell satisfies ^Cell;
borrowedCell satisfies Borrowed<Cell, "static">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type OwnedCell = WithOwnership<Cell, "owned", "static">;
type BorrowedCell = WithOwnership<Cell, "borrowed", "static">;

declare const ownedCell: OwnedCell;
declare const borrowedCell: BorrowedCell;

ownedCell satisfies ^Cell;
borrowedCell satisfies Borrowed<Cell, "static">;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type OwnedCell = WithOwnership<Cell, "owned", "static">;
/// @type.symbol symbol=OwnedCell source="type OwnedCell = WithOwnership<Cell, \"owned\", \"static\">" type=WithOwnership<Cell, "owned", "static"> reduced=Cell
/// @definition.type symbol=OwnedCell source="type OwnedCell = WithOwnership<Cell, \"owned\", \"static\">" value=WithOwnership<Cell, "owned", "static"> reduced=Cell
/// @resolution.name source=WithOwnership target=memory.type.WithOwnership
/// @resolution.name source=Cell target=Cell

type BorrowedCell = WithOwnership<Cell, "borrowed", "static">;
/// @type.symbol symbol=BorrowedCell source="type BorrowedCell = WithOwnership<Cell, \"borrowed\", \"static\">" type=WithOwnership<Cell, "borrowed", "static"> reduced=Borrowed<Cell, "static", "mutable">
/// @definition.type symbol=BorrowedCell source="type BorrowedCell = WithOwnership<Cell, \"borrowed\", \"static\">" value=WithOwnership<Cell, "borrowed", "static"> reduced=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=WithOwnership target=memory.type.WithOwnership
/// @resolution.name source=Cell target=Cell

declare const ownedCell: OwnedCell;
/// @type.symbol symbol=ownedCell source=ownedCell type=OwnedCell reduced=Cell
/// @resolution.name source=OwnedCell target=OwnedCell

declare const borrowedCell: BorrowedCell;
/// @type.symbol symbol=borrowedCell source=borrowedCell type=BorrowedCell reduced=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=BorrowedCell target=BorrowedCell

ownedCell satisfies ^Cell;
/// @resolution.name source=ownedCell target=ownedCell
/// @resolution.name source=Cell target=Cell

borrowedCell satisfies Borrowed<Cell, "static">;
/// @resolution.name source=borrowedCell target=borrowedCell
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="WithOwnership<Cell, \"borrowed\", \"static\">" template=memory.type.WithOwnership arguments=(Cell, "borrowed", "static")
/// @generic.instance id="WithOwnership<Cell, \"owned\", \"static\">" template=memory.type.WithOwnership arguments=(Cell, "owned", "static")
"#,
    );
}

#[test]
fn test_with_place_sets_explicit_place() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type SharedOwned = WithPlace<^Cell, "shared">;

declare const sharedOwned: SharedOwned;

sharedOwned satisfies shared ^Cell;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type SharedOwned = WithPlace<^Cell, "shared">;

declare const sharedOwned: SharedOwned;

sharedOwned satisfies shared ^Cell;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type SharedOwned = WithPlace<^Cell, "shared">;
/// @type.symbol symbol=SharedOwned source="type SharedOwned = WithPlace<^Cell, \"shared\">" type=WithPlace<Owned<Cell>, "shared"> reduced=Placed<Cell, "shared">
/// @definition.type symbol=SharedOwned source="type SharedOwned = WithPlace<^Cell, \"shared\">" value=WithPlace<Owned<Cell>, "shared"> reduced=Placed<Cell, "shared">
/// @resolution.name source=WithPlace target=memory.type.WithPlace
/// @resolution.name source=Cell target=Cell

declare const sharedOwned: SharedOwned;
/// @type.symbol symbol=sharedOwned source=sharedOwned type=SharedOwned reduced=Placed<Cell, "shared">
/// @resolution.name source=SharedOwned target=SharedOwned

sharedOwned satisfies shared ^Cell;
/// @resolution.name source=sharedOwned target=sharedOwned
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="WithPlace<Owned<Cell>, \"shared\">" template=memory.type.WithPlace arguments=(Owned<Cell>, "shared")
"#,
    );
}

#[test]
fn test_with_lifetime_sets_borrow_lifetime() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type StaticBorrow = WithLifetime<Borrowed<Cell, "static">, "static">;

declare const staticBorrow: StaticBorrow;

staticBorrow satisfies Borrowed<Cell, "static">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type StaticBorrow = WithLifetime<Borrowed<Cell, "static">, "static">;

declare const staticBorrow: StaticBorrow;

staticBorrow satisfies Borrowed<Cell, "static">;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type StaticBorrow = WithLifetime<Borrowed<Cell, "static">, "static">;
/// @type.symbol symbol=StaticBorrow source="type StaticBorrow = WithLifetime<Borrowed<Cell, \"static\">, \"static\">" type=WithLifetime<Borrowed<Cell, "static", "mutable">, "static"> reduced=Borrowed<Cell, "static", "mutable">
/// @definition.type symbol=StaticBorrow source="type StaticBorrow = WithLifetime<Borrowed<Cell, \"static\">, \"static\">" value=WithLifetime<Borrowed<Cell, "static", "mutable">, "static"> reduced=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=WithLifetime target=memory.type.WithLifetime
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

declare const staticBorrow: StaticBorrow;
/// @type.symbol symbol=staticBorrow source=staticBorrow type=StaticBorrow reduced=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=StaticBorrow target=StaticBorrow

staticBorrow satisfies Borrowed<Cell, "static">;
/// @resolution.name source=staticBorrow target=staticBorrow
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="Borrowed<Cell, \"static\", \"mutable\">" template=memory.borrow.Borrowed arguments=(Cell, "static", "mutable")
/// @generic.instance id="WithLifetime<Borrowed<Cell, \"static\", \"mutable\">, \"static\">" template=memory.type.WithLifetime arguments=(Borrowed<Cell, "static", "mutable">, "static")
"#,
    );
}

#[test]
fn test_with_space_resolves_ambient_space() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type SharedOwned = WithSpace<^Cell, "shared">;
type LocalSharedOwned = WithSpace<shared ^Cell, "local">;

declare const sharedOwned: SharedOwned;
declare const localSharedOwned: LocalSharedOwned;

sharedOwned satisfies shared ^Cell;
localSharedOwned satisfies local ^Cell;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type SharedOwned = WithSpace<^Cell, "shared">;
type LocalSharedOwned = WithSpace<shared ^Cell, "local">;

declare const sharedOwned: SharedOwned;
declare const localSharedOwned: LocalSharedOwned;

sharedOwned satisfies shared ^Cell;
localSharedOwned satisfies local ^Cell;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type SharedOwned = WithSpace<^Cell, "shared">;
/// @type.symbol symbol=SharedOwned source="type SharedOwned = WithSpace<^Cell, \"shared\">" type=WithSpace<Owned<Cell>, "shared"> reduced=Placed<Cell, "shared">
/// @definition.type symbol=SharedOwned source="type SharedOwned = WithSpace<^Cell, \"shared\">" value=WithSpace<Owned<Cell>, "shared"> reduced=Placed<Cell, "shared">
/// @resolution.name source=WithSpace target=memory.type.WithSpace
/// @resolution.name source=Cell target=Cell

type LocalSharedOwned = WithSpace<shared ^Cell, "local">;
/// @type.symbol symbol=LocalSharedOwned source="type LocalSharedOwned = WithSpace<shared ^Cell, \"local\">" type=WithSpace<Placed<Owned<Cell>, "shared">, "local"> reduced=Placed<Cell, "shared">
/// @definition.type symbol=LocalSharedOwned source="type LocalSharedOwned = WithSpace<shared ^Cell, \"local\">" value=WithSpace<Placed<Owned<Cell>, "shared">, "local"> reduced=Placed<Cell, "shared">
/// @resolution.name source=WithSpace target=memory.type.WithSpace
/// @resolution.name source=Cell target=Cell

declare const sharedOwned: SharedOwned;
/// @type.symbol symbol=sharedOwned source=sharedOwned type=SharedOwned reduced=Placed<Cell, "shared">
/// @resolution.name source=SharedOwned target=SharedOwned

declare const localSharedOwned: LocalSharedOwned;
/// @type.symbol symbol=localSharedOwned source=localSharedOwned type=LocalSharedOwned reduced=Placed<Cell, "shared">
/// @resolution.name source=LocalSharedOwned target=LocalSharedOwned

sharedOwned satisfies shared ^Cell;
/// @resolution.name source=sharedOwned target=sharedOwned
/// @resolution.name source=Cell target=Cell

localSharedOwned satisfies local ^Cell;
/// @resolution.name source=localSharedOwned target=localSharedOwned
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="WithSpace<Owned<Cell>, \"shared\">" template=memory.type.WithSpace arguments=(Owned<Cell>, "shared")
/// @generic.instance id="WithSpace<Placed<Owned<Cell>, \"shared\">, \"local\">" template=memory.type.WithSpace arguments=(Placed<Owned<Cell>, "shared">, "local")
"#,
        r#"
/// @diagnostic.error code=EC201 message="type 'LocalSharedOwned' does not satisfy 'local ^Cell'"
/// @diagnostic.label line=13 column=18 span="satisfies" line_source="localSharedOwned satisfies local ^Cell;"
"#,
    );
}

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type ReadonlyOwned = WithAccess<^Cell, "readonly">;
type ExclusiveBorrow = WithAccess<Borrowed<Cell, "static">, "exclusive">;

declare const readonlyOwned: ReadonlyOwned;
declare const exclusiveBorrow: ExclusiveBorrow;

readonlyOwned satisfies ^readonly Cell;
exclusiveBorrow satisfies Borrowed<Cell, "static", "exclusive">;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type ReadonlyOwned = WithAccess<^Cell, "readonly">;
/// @type.symbol symbol=ReadonlyOwned source="type ReadonlyOwned = WithAccess<^Cell, \"readonly\">" type=WithAccess<Owned<Cell>, "readonly"> reduced=Readonly<Cell>
/// @definition.type symbol=ReadonlyOwned source="type ReadonlyOwned = WithAccess<^Cell, \"readonly\">" value=WithAccess<Owned<Cell>, "readonly"> reduced=Readonly<Cell>
/// @resolution.name source=WithAccess target=memory.type.WithAccess
/// @resolution.name source=Cell target=Cell

type ExclusiveBorrow = WithAccess<Borrowed<Cell, "static">, "exclusive">;
/// @type.symbol symbol=ExclusiveBorrow source="type ExclusiveBorrow = WithAccess<Borrowed<Cell, \"static\">, \"exclusive\">" type=WithAccess<Borrowed<Cell, "static", "mutable">, "exclusive"> reduced=Borrowed<Cell, "static", "exclusive">
/// @definition.type symbol=ExclusiveBorrow source="type ExclusiveBorrow = WithAccess<Borrowed<Cell, \"static\">, \"exclusive\">" value=WithAccess<Borrowed<Cell, "static", "mutable">, "exclusive"> reduced=Borrowed<Cell, "static", "exclusive">
/// @resolution.name source=WithAccess target=memory.type.WithAccess
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

declare const readonlyOwned: ReadonlyOwned;
/// @type.symbol symbol=readonlyOwned source=readonlyOwned type=ReadonlyOwned reduced=Readonly<Cell>
/// @resolution.name source=ReadonlyOwned target=ReadonlyOwned

declare const exclusiveBorrow: ExclusiveBorrow;
/// @type.symbol symbol=exclusiveBorrow source=exclusiveBorrow type=ExclusiveBorrow reduced=Borrowed<Cell, "static", "exclusive">
/// @resolution.name source=ExclusiveBorrow target=ExclusiveBorrow

readonlyOwned satisfies ^readonly Cell;
/// @resolution.name source=readonlyOwned target=readonlyOwned
/// @resolution.name source=Cell target=Cell

exclusiveBorrow satisfies Borrowed<Cell, "static", "exclusive">;
/// @resolution.name source=exclusiveBorrow target=exclusiveBorrow
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="Borrowed<Cell, \"static\", \"mutable\">" template=memory.borrow.Borrowed arguments=(Cell, "static", "mutable")
/// @generic.instance id="WithAccess<Borrowed<Cell, \"static\", \"mutable\">, \"exclusive\">" template=memory.type.WithAccess arguments=(Borrowed<Cell, "static", "mutable">, "exclusive")
/// @generic.instance id="WithAccess<Owned<Cell>, \"readonly\">" template=memory.type.WithAccess arguments=(Owned<Cell>, "readonly")
"#,
    );
}

#[test]
fn test_with_base_replaces_memory_payload() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

struct Payload {
    value: int32;
}

type Rebased = WithBase<shared ^readonly Cell, Payload>;

declare const rebased: Rebased;

rebased satisfies shared ^readonly Payload;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

struct Payload {
    value: int32;
}

type Rebased = WithBase<shared ^readonly Cell, Payload>;

declare const rebased: Rebased;

rebased satisfies shared ^readonly Payload;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

struct Payload {
/// @type.symbol symbol=Payload type=Payload
/// @definition.struct symbol=Payload
/// @definition.field symbol=Payload.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Payload.value source="value: int32" type=int32

}

type Rebased = WithBase<shared ^readonly Cell, Payload>;
/// @type.symbol symbol=Rebased source="type Rebased = WithBase<shared ^readonly Cell, Payload>" type=WithBase<Placed<Owned<Readonly<Cell>>, "shared">, Payload> reduced=Placed<Readonly<Payload>, "shared">
/// @definition.type symbol=Rebased source="type Rebased = WithBase<shared ^readonly Cell, Payload>" value=WithBase<Placed<Owned<Readonly<Cell>>, "shared">, Payload> reduced=Placed<Readonly<Payload>, "shared">
/// @resolution.name source=WithBase target=memory.type.WithBase
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=Payload target=Payload

declare const rebased: Rebased;
/// @type.symbol symbol=rebased source=rebased type=Rebased reduced=Placed<Readonly<Payload>, "shared">
/// @resolution.name source=Rebased target=Rebased

rebased satisfies shared ^readonly Payload;
/// @resolution.name source=rebased target=rebased
/// @resolution.name source=Payload target=Payload

/// @generic.instance id="WithBase<Placed<Owned<Readonly<Cell>>, \"shared\">, Payload>" template=memory.type.WithBase arguments=(Placed<Owned<Readonly<Cell>>, "shared">, Payload)
"#,
    );
}

#[test]
fn test_borrowed_readonly_payload_clamps_access() {
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

    session.assert_dir_checked(
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

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type ReadonlyBorrow = Borrowed<Readonly<Cell>, "static">;
/// @type.symbol symbol=ReadonlyBorrow source="type ReadonlyBorrow = Borrowed<Readonly<Cell>, \"static\">" type=Borrowed<Readonly<Cell>, "static", "mutable"> reduced=Borrowed<Cell, "static", "readonly">
/// @definition.type symbol=ReadonlyBorrow source="type ReadonlyBorrow = Borrowed<Readonly<Cell>, \"static\">" value=Borrowed<Readonly<Cell>, "static", "mutable"> reduced=Borrowed<Cell, "static", "readonly">
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Readonly target=types.object.Readonly
/// @resolution.name source=Cell target=Cell

declare const borrow: ReadonlyBorrow;
/// @type.symbol symbol=borrow source=borrow type=ReadonlyBorrow reduced=Borrowed<Cell, "static", "readonly">
/// @resolution.name source=ReadonlyBorrow target=ReadonlyBorrow

borrow satisfies Borrowed<Cell, "static", "readonly">;
/// @resolution.name source=borrow target=borrow
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

/// @generic.instance id="Borrowed<Readonly<Cell>, \"static\", \"mutable\">" template=memory.borrow.Borrowed arguments=(Readonly<Cell>, "static", "mutable")
/// @generic.instance id=Readonly<Cell> template=types.object.Readonly arguments=(Cell)
"#,
    );
}

#[test]
fn test_default_trait_returns_this_type() {
    let session = TestSession::single(
        r#"
import { Phantom } from "destack:memory";

const marker: Phantom<int32> = Phantom<int32>.default();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Phantom } from "destack:memory";

const marker: Phantom<int32> = Phantom<int32>.default<int32>();

=== checked ===
import { Phantom } from "destack:memory";

const marker: Phantom<int32> = Phantom<int32>.default();
/// @type.symbol symbol=marker source=marker type=memory.phantom.Phantom<int32>
/// @resolution.name source=Phantom target=memory.phantom.Phantom
/// @type.node source=Phantom<int32> type=memory.phantom.Phantom<int32>
/// @type.node source=Phantom<int32>.default type=() => memory.phantom.Phantom<int32>
/// @type.node source=Phantom<int32>.default() type=memory.phantom.Phantom<int32>
/// @resolution.name source=Phantom target=memory.phantom.Phantom
/// @resolution.member source=Phantom<int32>.default receiver=memory.phantom.Phantom<int32> kind=symbol target=memory.phantom.default
/// @resolution.call source=Phantom<int32>.default() parameters=() return=memory.phantom.Phantom<int32> kind=symbol target=memory.phantom.default receiver=memory.phantom.Phantom<int32> instance=memory.phantom.Phantom<int32>.<extension#1>.default
/// @resolution.instantiation source=Phantom<int32> target=memory.phantom.Phantom instance=memory.phantom.Phantom<int32>
/// @generic.instance source=Phantom<int32> id=memory.phantom.Phantom<int32>
/// @generic.instance source=Phantom<int32>.default id=memory.phantom.Phantom<int32>
/// @generic.instance source=Phantom<int32>.default() id=memory.phantom.Phantom<int32>
/// @generic.instance source=Phantom<int32>.default() id=memory.phantom.Phantom<int32>.<extension#1>.default

/// @generic.instance id=memory.phantom.Phantom<int32> template=memory.phantom.Phantom arguments=(int32)
/// @generic.instance id=memory.phantom.Phantom<int32>.<extension#1>.default template=memory.phantom.default arguments=(int32)
"#,
    );
}

#[test]
fn test_placement_commutes_with_ownership() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

declare const outer: shared ^Cell;
declare const inner: ^shared Cell;

outer satisfies ^shared Cell;
inner satisfies shared ^Cell;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

declare const outer: shared ^Cell;
declare const inner: shared ^Cell;

outer satisfies ^shared Cell;
inner satisfies shared ^Cell;

=== checked ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

declare const outer: shared ^Cell;
/// @type.symbol symbol=outer source=outer type=Placed<Owned<Cell>, "shared"> reduced=Placed<Cell, "shared">
/// @resolution.name source=Cell target=Cell

declare const inner: ^shared Cell;
/// @type.symbol symbol=inner source=inner type=Placed<Owned<Cell>, "shared"> reduced=Placed<Cell, "shared">
/// @resolution.name source=Cell target=Cell

outer satisfies ^shared Cell;
/// @resolution.name source=outer target=outer
/// @resolution.name source=Cell target=Cell

inner satisfies shared ^Cell;
/// @resolution.name source=inner target=inner
/// @resolution.name source=Cell target=Cell
"#,
    );
}
