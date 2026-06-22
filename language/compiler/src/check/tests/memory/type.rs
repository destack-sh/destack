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
/// @type.symbol symbol=Base source="type Base = BaseOf<OwnedBorrow>" type=Cell
/// @definition.type symbol=Base source="type Base = BaseOf<OwnedBorrow>" value=Cell
/// @resolution.name source=BaseOf target=memory.type.BaseOf
/// @resolution.name source=OwnedBorrow target=OwnedBorrow

type Payload = PayloadOf<OwnedBorrow>;
/// @type.symbol symbol=Payload source="type Payload = PayloadOf<OwnedBorrow>" type=Borrowed<Cell, "static", "mutable">
/// @definition.type symbol=Payload source="type Payload = PayloadOf<OwnedBorrow>" value=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=PayloadOf target=memory.type.PayloadOf
/// @resolution.name source=OwnedBorrow target=OwnedBorrow

declare const base: Base;
/// @type.symbol symbol=base source=base type=memory.type.BaseOf<OwnedBorrow>
/// @resolution.name source=Base target=Base

declare const payload: Payload;
/// @type.symbol symbol=payload source=payload type=memory.type.PayloadOf<OwnedBorrow>
/// @resolution.name source=Payload target=Payload

base satisfies Cell;
/// @resolution.name source=base target=base
/// @resolution.name source=Cell target=Cell

payload satisfies Borrowed<Cell, "static">;
/// @resolution.name source=payload target=payload
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=BorrowOwned source="type BorrowOwned = Borrowed<Owned<Cell>, \"static\">" type=Borrowed<Cell, "static", "mutable">
/// @definition.type symbol=BorrowOwned source="type BorrowOwned = Borrowed<Owned<Cell>, \"static\">" value=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Owned target=memory.owned.Owned
/// @resolution.name source=Cell target=Cell

type BorrowedLifetime = LifetimeOf<BorrowOwned>;
/// @type.symbol symbol=BorrowedLifetime source="type BorrowedLifetime = LifetimeOf<BorrowOwned>" type="static"
/// @definition.type symbol=BorrowedLifetime source="type BorrowedLifetime = LifetimeOf<BorrowOwned>" value="static"
/// @resolution.name source=LifetimeOf target=memory.type.LifetimeOf
/// @resolution.name source=BorrowOwned target=BorrowOwned

type BorrowedAccess = AccessOf<BorrowOwned>;
/// @type.symbol symbol=BorrowedAccess source="type BorrowedAccess = AccessOf<BorrowOwned>" type="mutable"
/// @definition.type symbol=BorrowedAccess source="type BorrowedAccess = AccessOf<BorrowOwned>" value="mutable"
/// @resolution.name source=AccessOf target=memory.type.AccessOf
/// @resolution.name source=BorrowOwned target=BorrowOwned

declare const borrowedLifetime: BorrowedLifetime;
/// @type.symbol symbol=borrowedLifetime source=borrowedLifetime type=memory.type.LifetimeOf<BorrowOwned>
/// @resolution.name source=BorrowedLifetime target=BorrowedLifetime

declare const borrowedAccess: BorrowedAccess;
/// @type.symbol symbol=borrowedAccess source=borrowedAccess type=memory.type.AccessOf<BorrowOwned>
/// @resolution.name source=BorrowedAccess target=BorrowedAccess

borrowedLifetime satisfies "static";
/// @resolution.name source=borrowedLifetime target=borrowedLifetime

borrowedAccess satisfies "mutable";
/// @resolution.name source=borrowedAccess target=borrowedAccess
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
/// @type.symbol symbol=OwnershipDefault source="type OwnershipDefault = OwnershipOr<Cell, \"raw\">" type="managed"
/// @definition.type symbol=OwnershipDefault source="type OwnershipDefault = OwnershipOr<Cell, \"raw\">" value="managed"
/// @resolution.name source=OwnershipOr target=memory.type.OwnershipOr
/// @resolution.name source=Cell target=Cell

type AccessDefault = AccessOr<Cell, "readonly">;
/// @type.symbol symbol=AccessDefault source="type AccessDefault = AccessOr<Cell, \"readonly\">" type="mutable"
/// @definition.type symbol=AccessDefault source="type AccessDefault = AccessOr<Cell, \"readonly\">" value="mutable"
/// @resolution.name source=AccessOr target=memory.type.AccessOr
/// @resolution.name source=Cell target=Cell

type PlaceDefault = PlaceOr<Cell, "shared">;
/// @type.symbol symbol=PlaceDefault source="type PlaceDefault = PlaceOr<Cell, \"shared\">" type="ambient"
/// @definition.type symbol=PlaceDefault source="type PlaceDefault = PlaceOr<Cell, \"shared\">" value="ambient"
/// @resolution.name source=PlaceOr target=memory.type.PlaceOr
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=LifetimeFallback source="type LifetimeFallback = LifetimeOr<Cell, \"static\">" type="static"
/// @definition.type symbol=LifetimeFallback source="type LifetimeFallback = LifetimeOr<Cell, \"static\">" value="static"
/// @resolution.name source=LifetimeOr target=memory.type.LifetimeOr
/// @resolution.name source=Cell target=Cell

type SpaceFallback = SpaceOr<Cell, "shared">;
/// @type.symbol symbol=SpaceFallback source="type SpaceFallback = SpaceOr<Cell, \"shared\">" type="shared"
/// @definition.type symbol=SpaceFallback source="type SpaceFallback = SpaceOr<Cell, \"shared\">" value="shared"
/// @resolution.name source=SpaceOr target=memory.type.SpaceOr
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=AmbientInShared source="type AmbientInShared = PlaceIn<Cell, \"shared\">" type="shared"
/// @definition.type symbol=AmbientInShared source="type AmbientInShared = PlaceIn<Cell, \"shared\">" value="shared"
/// @resolution.name source=PlaceIn target=memory.type.PlaceIn
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=LocalInShared source="type LocalInShared = PlaceIn<local Cell, \"shared\">" type="local"
/// @definition.type symbol=LocalInShared source="type LocalInShared = PlaceIn<local Cell, \"shared\">" value="local"
/// @resolution.name source=PlaceIn target=memory.type.PlaceIn
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=ManagedKind source="type ManagedKind = OwnershipOf<Managed<Cell>>" type="managed"
/// @definition.type symbol=ManagedKind source="type ManagedKind = OwnershipOf<Managed<Cell>>" value="managed"
/// @resolution.name source=OwnershipOf target=memory.type.OwnershipOf
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Cell target=Cell

type OwnedKind = OwnershipOf<Owned<Cell>>;
/// @type.symbol symbol=OwnedKind source="type OwnedKind = OwnershipOf<Owned<Cell>>" type="owned"
/// @definition.type symbol=OwnedKind source="type OwnedKind = OwnershipOf<Owned<Cell>>" value="owned"
/// @resolution.name source=OwnershipOf target=memory.type.OwnershipOf
/// @resolution.name source=Owned target=memory.owned.Owned
/// @resolution.name source=Cell target=Cell

type BorrowedKind = OwnershipOf<Borrowed<Cell, "static">>;
/// @type.symbol symbol=BorrowedKind source="type BorrowedKind = OwnershipOf<Borrowed<Cell, \"static\">>" type="borrowed"
/// @definition.type symbol=BorrowedKind source="type BorrowedKind = OwnershipOf<Borrowed<Cell, \"static\">>" value="borrowed"
/// @resolution.name source=OwnershipOf target=memory.type.OwnershipOf
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

type RawKind = OwnershipOf<Raw<Cell>>;
/// @type.symbol symbol=RawKind source="type RawKind = OwnershipOf<Raw<Cell>>" type="raw"
/// @definition.type symbol=RawKind source="type RawKind = OwnershipOf<Raw<Cell>>" value="raw"
/// @resolution.name source=OwnershipOf target=memory.type.OwnershipOf
/// @resolution.name source=Raw target=memory.raw.Raw
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=ManagedCheck source="type ManagedCheck = IsManaged<Managed<Cell>>" type=true
/// @definition.type symbol=ManagedCheck source="type ManagedCheck = IsManaged<Managed<Cell>>" value=true
/// @resolution.name source=IsManaged target=memory.type.IsManaged
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Cell target=Cell

type OwnedCheck = IsOwned<Owned<Cell>>;
/// @type.symbol symbol=OwnedCheck source="type OwnedCheck = IsOwned<Owned<Cell>>" type=true
/// @definition.type symbol=OwnedCheck source="type OwnedCheck = IsOwned<Owned<Cell>>" value=true
/// @resolution.name source=IsOwned target=memory.type.IsOwned
/// @resolution.name source=Owned target=memory.owned.Owned
/// @resolution.name source=Cell target=Cell

type BorrowedCheck = IsBorrowed<Borrowed<Cell, "static">>;
/// @type.symbol symbol=BorrowedCheck source="type BorrowedCheck = IsBorrowed<Borrowed<Cell, \"static\">>" type=true
/// @definition.type symbol=BorrowedCheck source="type BorrowedCheck = IsBorrowed<Borrowed<Cell, \"static\">>" value=true
/// @resolution.name source=IsBorrowed target=memory.type.IsBorrowed
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

type RawCheck = IsRaw<Raw<Cell>>;
/// @type.symbol symbol=RawCheck source="type RawCheck = IsRaw<Raw<Cell>>" type=true
/// @definition.type symbol=RawCheck source="type RawCheck = IsRaw<Raw<Cell>>" value=true
/// @resolution.name source=IsRaw target=memory.type.IsRaw
/// @resolution.name source=Raw target=memory.raw.Raw
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=SharedCheck source="type SharedCheck = IsShared<shared Cell>" type=true
/// @definition.type symbol=SharedCheck source="type SharedCheck = IsShared<shared Cell>" value=true
/// @resolution.name source=IsShared target=memory.type.IsShared
/// @resolution.name source=Cell target=Cell

type AmbientSharedCheck = IsSharedIn<Cell, "shared">;
/// @type.symbol symbol=AmbientSharedCheck source="type AmbientSharedCheck = IsSharedIn<Cell, \"shared\">" type=true
/// @definition.type symbol=AmbientSharedCheck source="type AmbientSharedCheck = IsSharedIn<Cell, \"shared\">" value=true
/// @resolution.name source=IsSharedIn target=memory.type.IsSharedIn
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=OwnedCell source="type OwnedCell = WithOwnership<Cell, \"owned\", \"static\">" type=Owned<Cell>
/// @definition.type symbol=OwnedCell source="type OwnedCell = WithOwnership<Cell, \"owned\", \"static\">" value=Owned<Cell>
/// @resolution.name source=WithOwnership target=memory.type.WithOwnership
/// @resolution.name source=Cell target=Cell

type BorrowedCell = WithOwnership<Cell, "borrowed", "static">;
/// @type.symbol symbol=BorrowedCell source="type BorrowedCell = WithOwnership<Cell, \"borrowed\", \"static\">" type=Borrowed<Cell, "static", "mutable">
/// @definition.type symbol=BorrowedCell source="type BorrowedCell = WithOwnership<Cell, \"borrowed\", \"static\">" value=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=WithOwnership target=memory.type.WithOwnership
/// @resolution.name source=Cell target=Cell

declare const ownedCell: OwnedCell;
/// @type.symbol symbol=ownedCell source=ownedCell type=memory.type.WithOwnership<Cell, "owned", "static">
/// @resolution.name source=OwnedCell target=OwnedCell

declare const borrowedCell: BorrowedCell;
/// @type.symbol symbol=borrowedCell source=borrowedCell type=memory.type.WithOwnership<Cell, "borrowed", "static">
/// @resolution.name source=BorrowedCell target=BorrowedCell

ownedCell satisfies ^Cell;
/// @resolution.name source=ownedCell target=ownedCell
/// @resolution.name source=Cell target=Cell

borrowedCell satisfies Borrowed<Cell, "static">;
/// @resolution.name source=borrowedCell target=borrowedCell
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=SharedOwned source="type SharedOwned = WithPlace<^Cell, \"shared\">" type=Placed<Owned<Cell>, "shared">
/// @definition.type symbol=SharedOwned source="type SharedOwned = WithPlace<^Cell, \"shared\">" value=Placed<Owned<Cell>, "shared">
/// @resolution.name source=WithPlace target=memory.type.WithPlace
/// @resolution.name source=Cell target=Cell

declare const sharedOwned: SharedOwned;
/// @type.symbol symbol=sharedOwned source=sharedOwned type=memory.type.WithPlace<Owned<Cell>, "shared">
/// @resolution.name source=SharedOwned target=SharedOwned

sharedOwned satisfies shared ^Cell;
/// @resolution.name source=sharedOwned target=sharedOwned
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=StaticBorrow source="type StaticBorrow = WithLifetime<Borrowed<Cell, \"static\">, \"static\">" type=Borrowed<Cell, "static", "mutable">
/// @definition.type symbol=StaticBorrow source="type StaticBorrow = WithLifetime<Borrowed<Cell, \"static\">, \"static\">" value=Borrowed<Cell, "static", "mutable">
/// @resolution.name source=WithLifetime target=memory.type.WithLifetime
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

declare const staticBorrow: StaticBorrow;
/// @type.symbol symbol=staticBorrow source=staticBorrow type=memory.type.WithLifetime<Borrowed<Cell, "static">, "static">
/// @resolution.name source=StaticBorrow target=StaticBorrow

staticBorrow satisfies Borrowed<Cell, "static">;
/// @resolution.name source=staticBorrow target=staticBorrow
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell
"#,
    );
}

#[test]
fn test_with_space_sets_storage_space() {
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

    session.assert_dir_checked(
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
/// @type.symbol symbol=SharedOwned source="type SharedOwned = WithSpace<^Cell, \"shared\">" type=Placed<Owned<Cell>, "shared">
/// @definition.type symbol=SharedOwned source="type SharedOwned = WithSpace<^Cell, \"shared\">" value=Placed<Owned<Cell>, "shared">
/// @resolution.name source=WithSpace target=memory.type.WithSpace
/// @resolution.name source=Cell target=Cell

type LocalSharedOwned = WithSpace<shared ^Cell, "local">;
/// @type.symbol symbol=LocalSharedOwned source="type LocalSharedOwned = WithSpace<shared ^Cell, \"local\">" type=Placed<Owned<Cell>, "local">
/// @definition.type symbol=LocalSharedOwned source="type LocalSharedOwned = WithSpace<shared ^Cell, \"local\">" value=Placed<Owned<Cell>, "local">
/// @resolution.name source=WithSpace target=memory.type.WithSpace
/// @resolution.name source=Cell target=Cell

declare const sharedOwned: SharedOwned;
/// @type.symbol symbol=sharedOwned source=sharedOwned type=memory.type.WithSpace<Owned<Cell>, "shared">
/// @resolution.name source=SharedOwned target=SharedOwned

declare const localSharedOwned: LocalSharedOwned;
/// @type.symbol symbol=localSharedOwned source=localSharedOwned type=memory.type.WithSpace<Placed<Owned<Cell>, "shared">, "local">
/// @resolution.name source=LocalSharedOwned target=LocalSharedOwned

sharedOwned satisfies shared ^Cell;
/// @resolution.name source=sharedOwned target=sharedOwned
/// @resolution.name source=Cell target=Cell

localSharedOwned satisfies local ^Cell;
/// @resolution.name source=localSharedOwned target=localSharedOwned
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=ReadonlyOwned source="type ReadonlyOwned = WithAccess<^Cell, \"readonly\">" type=Owned<readonly Cell>
/// @definition.type symbol=ReadonlyOwned source="type ReadonlyOwned = WithAccess<^Cell, \"readonly\">" value=Owned<readonly Cell>
/// @resolution.name source=WithAccess target=memory.type.WithAccess
/// @resolution.name source=Cell target=Cell

type ExclusiveBorrow = WithAccess<Borrowed<Cell, "static">, "exclusive">;
/// @type.symbol symbol=ExclusiveBorrow source="type ExclusiveBorrow = WithAccess<Borrowed<Cell, \"static\">, \"exclusive\">" type=Borrowed<Cell, "static", "exclusive">
/// @definition.type symbol=ExclusiveBorrow source="type ExclusiveBorrow = WithAccess<Borrowed<Cell, \"static\">, \"exclusive\">" value=Borrowed<Cell, "static", "exclusive">
/// @resolution.name source=WithAccess target=memory.type.WithAccess
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

declare const readonlyOwned: ReadonlyOwned;
/// @type.symbol symbol=readonlyOwned source=readonlyOwned type=memory.type.WithAccess<Owned<Cell>, "readonly">
/// @resolution.name source=ReadonlyOwned target=ReadonlyOwned

declare const exclusiveBorrow: ExclusiveBorrow;
/// @type.symbol symbol=exclusiveBorrow source=exclusiveBorrow type=memory.type.WithAccess<Borrowed<Cell, "static">, "exclusive">
/// @resolution.name source=ExclusiveBorrow target=ExclusiveBorrow

readonlyOwned satisfies ^readonly Cell;
/// @resolution.name source=readonlyOwned target=readonlyOwned
/// @resolution.name source=Cell target=Cell

exclusiveBorrow satisfies Borrowed<Cell, "static", "exclusive">;
/// @resolution.name source=exclusiveBorrow target=exclusiveBorrow
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell
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
/// @type.symbol symbol=Rebased source="type Rebased = WithBase<shared ^readonly Cell, Payload>" type=Placed<Owned<readonly Payload>, "shared">
/// @definition.type symbol=Rebased source="type Rebased = WithBase<shared ^readonly Cell, Payload>" value=Placed<Owned<readonly Payload>, "shared">
/// @resolution.name source=WithBase target=memory.type.WithBase
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=Payload target=Payload

declare const rebased: Rebased;
/// @type.symbol symbol=rebased source=rebased type=memory.type.WithBase<Placed<Owned<readonly Cell>, "shared">, Payload>
/// @resolution.name source=Rebased target=Rebased

rebased satisfies shared ^readonly Payload;
/// @resolution.name source=rebased target=rebased
/// @resolution.name source=Payload target=Payload
"#,
    );
}
