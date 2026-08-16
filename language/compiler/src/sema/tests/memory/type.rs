use crate::tests::{DirRows, TestSession};

#[test]
fn test_reject_lifetime_roots_as_placement_spaces() {
    let session = TestSession::single(
        r#"
struct Cell {}

type StaticCell = WithSpace<Cell, "static">;
type FrameCell = WithSpace<Cell, "frame">;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {}

type StaticCell = WithSpace<Cell, "static">;
type FrameCell = WithSpace<Cell, "frame">;

=== dir ===
struct Cell {}
/// @type.symbol symbol=Cell source="struct Cell {}" type=Cell
/// @definition.struct symbol=Cell source="struct Cell {}"

type StaticCell = WithSpace<Cell, "static">;
/// @type.symbol symbol=StaticCell source="type StaticCell = WithSpace<Cell, \"static\">" type=Placed<Cell, "static">
/// @definition.type symbol=StaticCell source="type StaticCell = WithSpace<Cell, \"static\">" value=Placed<Cell, "static">
/// @resolution.name source=WithSpace target=memory.type.WithSpace
/// @resolution.name source=Cell target=Cell

type FrameCell = WithSpace<Cell, "frame">;
/// @type.symbol symbol=FrameCell source="type FrameCell = WithSpace<Cell, \"frame\">" type=Placed<Cell, "frame">
/// @definition.type symbol=FrameCell source="type FrameCell = WithSpace<Cell, \"frame\">" value=Placed<Cell, "frame">
/// @resolution.name source=WithSpace target=memory.type.WithSpace
/// @resolution.name source=Cell target=Cell
"#,
        r#"
"#,
    );
}

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

    session.assert_dir(
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

declare const base: Cell;
declare const payload: &'static Cell;

base satisfies Cell;
payload satisfies Borrowed<Cell, "static">;

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type OwnedBorrow = Owned<Borrowed<Cell, "static">>;
/// @type.symbol symbol=OwnedBorrow source="type OwnedBorrow = Owned<Borrowed<Cell, \"static\">>" type=Owned<&'static Cell>
/// @definition.type symbol=OwnedBorrow source="type OwnedBorrow = Owned<Borrowed<Cell, \"static\">>" value=Owned<&'static Cell>
/// @resolution.name source=Owned target=memory.owned.Owned
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

type Base = BaseOf<OwnedBorrow>;
/// @type.symbol symbol=Base source="type Base = BaseOf<OwnedBorrow>" type=Cell
/// @definition.type symbol=Base source="type Base = BaseOf<OwnedBorrow>" value=Cell
/// @resolution.name source=BaseOf target=memory.type.BaseOf
/// @resolution.name source=OwnedBorrow target=OwnedBorrow

type Payload = PayloadOf<OwnedBorrow>;
/// @type.symbol symbol=Payload source="type Payload = PayloadOf<OwnedBorrow>" type=&'static Cell
/// @definition.type symbol=Payload source="type Payload = PayloadOf<OwnedBorrow>" value=&'static Cell
/// @resolution.name source=PayloadOf target=memory.type.PayloadOf
/// @resolution.name source=OwnedBorrow target=OwnedBorrow

declare const base: Base;
/// @type.symbol symbol=base source=base type=Cell
/// @resolution.pattern source=base kind=binding target=base
/// @resolution.name source=Base target=Base

declare const payload: Payload;
/// @type.symbol symbol=payload source=payload type=&'static Cell
/// @resolution.pattern source=payload kind=binding target=payload
/// @resolution.name source=Payload target=Payload

base satisfies Cell;
/// @resolution.name source=base target=base
/// @resolution.place source=base placement="local" lifetime="static" access="readonly"
/// @resolution.access source=base root=base
/// @resolution.name source=Cell target=Cell

payload satisfies Borrowed<Cell, "static">;
/// @resolution.name source=payload target=payload
/// @resolution.place source=payload placement="local" lifetime="static" access="mutable"
/// @resolution.access source=payload root=payload
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

    session.assert_dir(
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

declare const borrowedLifetime: "static";
declare const borrowedAccess: "mutable";

borrowedLifetime satisfies "static";
borrowedAccess satisfies "mutable";

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type BorrowOwned = Borrowed<Owned<Cell>, "static">;
/// @type.symbol symbol=BorrowOwned source="type BorrowOwned = Borrowed<Owned<Cell>, \"static\">" type=&'static Cell
/// @definition.type symbol=BorrowOwned source="type BorrowOwned = Borrowed<Owned<Cell>, \"static\">" value=&'static Cell
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
/// @type.symbol symbol=borrowedLifetime source=borrowedLifetime type="static"
/// @resolution.pattern source=borrowedLifetime kind=binding target=borrowedLifetime
/// @resolution.name source=BorrowedLifetime target=BorrowedLifetime

declare const borrowedAccess: BorrowedAccess;
/// @type.symbol symbol=borrowedAccess source=borrowedAccess type="mutable"
/// @resolution.pattern source=borrowedAccess kind=binding target=borrowedAccess
/// @resolution.name source=BorrowedAccess target=BorrowedAccess

borrowedLifetime satisfies "static";
/// @resolution.name source=borrowedLifetime target=borrowedLifetime
/// @resolution.place source=borrowedLifetime placement="local" lifetime="static" access="readonly"
/// @resolution.access source=borrowedLifetime root=borrowedLifetime

borrowedAccess satisfies "mutable";
/// @resolution.name source=borrowedAccess target=borrowedAccess
/// @resolution.place source=borrowedAccess placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=borrowedAccess root=borrowedAccess
"#,
    );
}

#[test]
fn test_read_lifetime_only_from_borrowed_form() {
    let session = TestSession::single(
        r#"
class User {}

type ManagedLifetime = LifetimeOf<Managed<User>>;
type ManagedLifetimeFallback = LifetimeOr<Managed<User>, "static">;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

type ManagedLifetime = LifetimeOf<Managed<User>>;
type ManagedLifetimeFallback = LifetimeOr<Managed<User>, "static">;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

type ManagedLifetime = LifetimeOf<Managed<User>>;
/// @type.symbol symbol=ManagedLifetime source="type ManagedLifetime = LifetimeOf<Managed<User>>" type=never
/// @definition.type symbol=ManagedLifetime source="type ManagedLifetime = LifetimeOf<Managed<User>>" value=never
/// @resolution.name source=LifetimeOf target=memory.type.LifetimeOf
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=User target=User

type ManagedLifetimeFallback = LifetimeOr<Managed<User>, "static">;
/// @type.symbol symbol=ManagedLifetimeFallback source="type ManagedLifetimeFallback = LifetimeOr<Managed<User>, \"static\">" type="static"
/// @definition.type symbol=ManagedLifetimeFallback source="type ManagedLifetimeFallback = LifetimeOr<Managed<User>, \"static\">" value="static"
/// @resolution.name source=LifetimeOr target=memory.type.LifetimeOr
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_use_lifetime_query_as_const_default() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type Reborrow<Q, const L: Lifetime = type LifetimeOr<Q, "static">> = Borrowed<Q, L>;
type StaticCell = Reborrow<Cell>;

declare const cell: StaticCell;

cell satisfies Borrowed<Cell, "static">;
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

type Reborrow<Q, const L: Lifetime = LifetimeOr<Q, "static">> = Borrowed<Q, L>;
type StaticCell = Reborrow<Cell>;

declare const cell: &'static Cell;

cell satisfies Borrowed<Cell, "static">;

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type Reborrow<Q, const L: Lifetime = type LifetimeOr<Q, "static">> = Borrowed<Q, L>;
/// @generic.template symbol=Reborrow parameters=(Q, const L: Lifetime = LifetimeOr<Q, "static">)
/// @type.symbol symbol=Reborrow type=Borrowed<Q, L, "mutable">
/// @definition.type symbol=Reborrow template=(Q, const L: Lifetime = LifetimeOr<Q, "static">) value=Borrowed<Q, L, "mutable">
/// @type.symbol symbol=Reborrow.Q source=Q type=Q
/// @type.symbol symbol=Reborrow.L source="const L: Lifetime = type LifetimeOr<Q, \"static\">" type=L
/// @resolution.name source=Lifetime target=memory.lifetime.Lifetime
/// @resolution.name source=LifetimeOr target=memory.type.LifetimeOr
/// @resolution.name source=Q target=Reborrow.Q
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Q target=Reborrow.Q
/// @resolution.name source=L target=Reborrow.L

type StaticCell = Reborrow<Cell>;
/// @type.symbol symbol=StaticCell source="type StaticCell = Reborrow<Cell>" type=&'static Cell
/// @definition.type symbol=StaticCell source="type StaticCell = Reborrow<Cell>" value=&'static Cell
/// @resolution.name source=Reborrow target=Reborrow
/// @resolution.name source=Cell target=Cell

declare const cell: StaticCell;
/// @type.symbol symbol=cell source=cell type=&'static Cell
/// @resolution.pattern source=cell kind=binding target=cell
/// @resolution.name source=StaticCell target=StaticCell

cell satisfies Borrowed<Cell, "static">;
/// @resolution.name source=cell target=cell
/// @resolution.place source=cell placement="local" lifetime="static" access="mutable"
/// @resolution.access source=cell root=cell
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell
"#,
    );
}

#[test]
fn test_use_place_query_as_const_default() {
    let session = TestSession::single(
        r#"
struct Cell { value: int32; }

type PreservePlace<Q, const P: Place = type PlaceOf<Q>> = WithPlace<BaseOf<Q>, P>;

declare const localCell: PreservePlace<local Cell>;
declare const sharedCell: PreservePlace<shared Cell>;

localCell satisfies local Cell;
sharedCell satisfies shared Cell;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type PreservePlace<Q, const P: Place = PlaceOf<Q>> = WithPlace<BaseOf<Q>, P>;

declare const localCell: local Cell;
declare const sharedCell: shared Cell;

localCell satisfies local Cell;
sharedCell satisfies shared Cell;

=== dir ===
struct Cell { value: int32; }
/// @type.symbol symbol=Cell source="struct Cell { value: int32; }" type=Cell
/// @definition.struct symbol=Cell source="struct Cell { value: int32; }"
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32
/// @type.symbol symbol=Cell.value source="value: int32" type=int32

type PreservePlace<Q, const P: Place = type PlaceOf<Q>> = WithPlace<BaseOf<Q>, P>;
/// @generic.template symbol=PreservePlace parameters=(Q, const P: Place = PlaceOf<Q>)
/// @type.symbol symbol=PreservePlace type=WithPlace<BaseOf<Q>, P>
/// @definition.type symbol=PreservePlace template=(Q, const P: Place = PlaceOf<Q>) value=WithPlace<BaseOf<Q>, P>
/// @type.symbol symbol=PreservePlace.Q source=Q type=Q
/// @type.symbol symbol=PreservePlace.P source="const P: Place = type PlaceOf<Q>" type=P
/// @resolution.name source=Place target=memory.place.Place
/// @resolution.name source=PlaceOf target=memory.type.PlaceOf
/// @resolution.name source=Q target=PreservePlace.Q
/// @resolution.name source=WithPlace target=memory.type.WithPlace
/// @resolution.name source=BaseOf target=memory.type.BaseOf
/// @resolution.name source=Q target=PreservePlace.Q
/// @resolution.name source=P target=PreservePlace.P

declare const localCell: PreservePlace<local Cell>;
/// @type.symbol symbol=localCell source=localCell type=Placed<Cell, "local">
/// @resolution.pattern source=localCell kind=binding target=localCell
/// @resolution.name source=PreservePlace target=PreservePlace
/// @resolution.name source=Cell target=Cell

declare const sharedCell: PreservePlace<shared Cell>;
/// @type.symbol symbol=sharedCell source=sharedCell type=Placed<Cell, "shared">
/// @resolution.pattern source=sharedCell kind=binding target=sharedCell
/// @resolution.name source=PreservePlace target=PreservePlace
/// @resolution.name source=Cell target=Cell

localCell satisfies local Cell;
/// @resolution.name source=localCell target=localCell
/// @resolution.place source=localCell placement="local" lifetime="static" access="readonly"
/// @resolution.access source=localCell root=localCell
/// @resolution.name source=Cell target=Cell

sharedCell satisfies shared Cell;
/// @resolution.name source=sharedCell target=sharedCell
/// @resolution.place source=sharedCell placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=sharedCell root=sharedCell
/// @resolution.name source=Cell target=Cell
"#,
        r#"

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

    session.assert_dir(
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

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type OwnershipDefault = OwnershipOr<Cell, "raw">;
/// @type.symbol symbol=OwnershipDefault source="type OwnershipDefault = OwnershipOr<Cell, \"raw\">" type="owned"
/// @definition.type symbol=OwnershipDefault source="type OwnershipDefault = OwnershipOr<Cell, \"raw\">" value="owned"
/// @resolution.name source=OwnershipOr target=memory.type.OwnershipOr
/// @resolution.name source=Cell target=Cell

type AccessDefault = AccessOr<Cell, "readonly">;
/// @type.symbol symbol=AccessDefault source="type AccessDefault = AccessOr<Cell, \"readonly\">" type="mutable"
/// @definition.type symbol=AccessDefault source="type AccessDefault = AccessOr<Cell, \"readonly\">" value="mutable"
/// @resolution.name source=AccessOr target=memory.type.AccessOr
/// @resolution.name source=Cell target=Cell

type PlaceDefault = PlaceOr<Cell, "shared">;
/// @type.symbol symbol=PlaceDefault source="type PlaceDefault = PlaceOr<Cell, \"shared\">" type="relative"
/// @definition.type symbol=PlaceDefault source="type PlaceDefault = PlaceOr<Cell, \"shared\">" value="relative"
/// @resolution.name source=PlaceOr target=memory.type.PlaceOr
/// @resolution.name source=Cell target=Cell
"#,
    );
}

#[test]
fn test_apply_lifetime_and_space_defaults_to_plain_type() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type LifetimeFallback = LifetimeOr<Cell, "static">;
type SpaceFallback = SpaceOr<Cell, "shared">;
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

type LifetimeFallback = LifetimeOr<Cell, "static">;
type SpaceFallback = SpaceOr<Cell, "shared">;

=== dir ===
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
fn test_place_in_resolves_relative_place_to_requested_space() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type RelativeInShared = PlaceIn<Cell, "shared">;
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

type RelativeInShared = PlaceIn<Cell, "shared">;

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type RelativeInShared = PlaceIn<Cell, "shared">;
/// @type.symbol symbol=RelativeInShared source="type RelativeInShared = PlaceIn<Cell, \"shared\">" type="shared"
/// @definition.type symbol=RelativeInShared source="type RelativeInShared = PlaceIn<Cell, \"shared\">" value="shared"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type LocalInShared = PlaceIn<local Cell, "shared">;

=== dir ===
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

    session.assert_dir(
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

=== dir ===
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

    session.assert_dir(
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

=== dir ===
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
type RelativeSharedCheck = IsSharedIn<Cell, "shared">;
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

type SharedCheck = IsShared<shared Cell>;
type RelativeSharedCheck = IsSharedIn<Cell, "shared">;

=== dir ===
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

type RelativeSharedCheck = IsSharedIn<Cell, "shared">;
/// @type.symbol symbol=RelativeSharedCheck source="type RelativeSharedCheck = IsSharedIn<Cell, \"shared\">" type=true
/// @definition.type symbol=RelativeSharedCheck source="type RelativeSharedCheck = IsSharedIn<Cell, \"shared\">" value=true
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type OwnedCell = WithOwnership<Cell, "owned", "static">;
type BorrowedCell = WithOwnership<Cell, "borrowed", "static">;

declare const ownedCell: ^Cell;
declare const borrowedCell: &'static Cell;

ownedCell satisfies ^Cell;
borrowedCell satisfies Borrowed<Cell, "static">;

=== dir ===
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
/// @type.symbol symbol=BorrowedCell source="type BorrowedCell = WithOwnership<Cell, \"borrowed\", \"static\">" type=&'static Cell
/// @definition.type symbol=BorrowedCell source="type BorrowedCell = WithOwnership<Cell, \"borrowed\", \"static\">" value=&'static Cell
/// @resolution.name source=WithOwnership target=memory.type.WithOwnership
/// @resolution.name source=Cell target=Cell

declare const ownedCell: OwnedCell;
/// @type.symbol symbol=ownedCell source=ownedCell type=Owned<Cell>
/// @resolution.pattern source=ownedCell kind=binding target=ownedCell
/// @resolution.name source=OwnedCell target=OwnedCell

declare const borrowedCell: BorrowedCell;
/// @type.symbol symbol=borrowedCell source=borrowedCell type=&'static Cell
/// @resolution.pattern source=borrowedCell kind=binding target=borrowedCell
/// @resolution.name source=BorrowedCell target=BorrowedCell

ownedCell satisfies ^Cell;
/// @resolution.name source=ownedCell target=ownedCell
/// @resolution.place source=ownedCell placement="local" lifetime="static" access="readonly"
/// @resolution.access source=ownedCell root=ownedCell
/// @resolution.name source=Cell target=Cell

borrowedCell satisfies Borrowed<Cell, "static">;
/// @resolution.name source=borrowedCell target=borrowedCell
/// @resolution.place source=borrowedCell placement="local" lifetime="static" access="mutable"
/// @resolution.access source=borrowedCell root=borrowedCell
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell
"#,
    );
}

#[test]
fn test_with_place_replaces_explicit_place() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type SharedOwned = WithPlace<local ^Cell, "shared">;

declare const sharedOwned: SharedOwned;

sharedOwned satisfies shared ^Cell;
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

type SharedOwned = WithPlace<local ^Cell, "shared">;

declare const sharedOwned: shared ^Cell;

sharedOwned satisfies shared ^Cell;

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type SharedOwned = WithPlace<local ^Cell, "shared">;
/// @type.symbol symbol=SharedOwned source="type SharedOwned = WithPlace<local ^Cell, \"shared\">" type=Placed<Owned<Cell>, "shared">
/// @definition.type symbol=SharedOwned source="type SharedOwned = WithPlace<local ^Cell, \"shared\">" value=Placed<Owned<Cell>, "shared">
/// @resolution.name source=WithPlace target=memory.type.WithPlace
/// @resolution.name source=Cell target=Cell

declare const sharedOwned: SharedOwned;
/// @type.symbol symbol=sharedOwned source=sharedOwned type=Placed<Owned<Cell>, "shared">
/// @resolution.pattern source=sharedOwned kind=binding target=sharedOwned
/// @resolution.name source=SharedOwned target=SharedOwned

sharedOwned satisfies shared ^Cell;
/// @resolution.name source=sharedOwned target=sharedOwned
/// @resolution.place source=sharedOwned placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=sharedOwned root=sharedOwned
/// @resolution.name source=Cell target=Cell
"#,
    );
}

#[test]
fn test_with_lifetime_replaces_borrow_lifetime() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type Reborrow<const Source: Lifetime, const Target: Lifetime> = WithLifetime<
    Borrowed<Cell, Source>,
    Target
>;
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

type Reborrow<const Source: Lifetime, const Target: Lifetime> = WithLifetime<
    Borrowed<Cell, Source>,
    Target
>;

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type Reborrow<const Source: Lifetime, const Target: Lifetime> = WithLifetime<
/// @generic.template symbol=Reborrow parameters=(const Source: Lifetime, const Target: Lifetime)
/// @type.symbol symbol=Reborrow type=WithLifetime<Borrowed<Cell, Source, "mutable">, Target>
/// @generic.instance id="WithLifetime<Borrowed<Cell, Source, \"mutable\">>" template=memory.type.WithLifetime arguments=(Borrowed<Cell, Source, "mutable">)
/// @definition.type symbol=Reborrow template=(const Source: Lifetime, const Target: Lifetime) value=WithLifetime<Borrowed<Cell, Source, "mutable">, Target>
/// @type.symbol symbol=Reborrow.Source source="const Source: Lifetime" type=Source
/// @resolution.name source=Lifetime target=memory.lifetime.Lifetime
/// @type.symbol symbol=Reborrow.Target source="const Target: Lifetime" type=Target
/// @resolution.name source=Lifetime target=memory.lifetime.Lifetime
/// @resolution.name source=WithLifetime target=memory.type.WithLifetime

    Borrowed<Cell, Source>,
    /// @resolution.name source=Borrowed target=memory.borrow.Borrowed
    /// @resolution.name source=Cell target=Cell
    /// @resolution.name source=Source target=Reborrow.Source

    Target
    /// @resolution.name source=Target target=Reborrow.Target

>;
"#,
    );
}

#[test]
fn test_with_space_resolves_relative_space() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type SharedOwned = WithSpace<^Cell, "shared">;

declare const sharedOwned: SharedOwned;

sharedOwned satisfies shared ^Cell;
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

type SharedOwned = WithSpace<^Cell, "shared">;

declare const sharedOwned: shared ^Cell;

sharedOwned satisfies shared ^Cell;

=== dir ===
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

declare const sharedOwned: SharedOwned;
/// @type.symbol symbol=sharedOwned source=sharedOwned type=Placed<Owned<Cell>, "shared">
/// @resolution.pattern source=sharedOwned kind=binding target=sharedOwned
/// @resolution.name source=SharedOwned target=SharedOwned

sharedOwned satisfies shared ^Cell;
/// @resolution.name source=sharedOwned target=sharedOwned
/// @resolution.place source=sharedOwned placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=sharedOwned root=sharedOwned
/// @resolution.name source=Cell target=Cell
"#,
    );
}

#[test]
fn test_with_space_preserves_explicit_space() {
    let session = TestSession::single(
        r#"
struct Cell {
    value: int32;
}

type StillShared = WithSpace<shared ^Cell, "local">;
declare const value: StillShared;

value satisfies shared ^Cell;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

type StillShared = WithSpace<shared ^Cell, "local">;
declare const value: shared ^Cell;

value satisfies shared ^Cell;

=== dir ===
struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

type StillShared = WithSpace<shared ^Cell, "local">;
/// @type.symbol symbol=StillShared source="type StillShared = WithSpace<shared ^Cell, \"local\">" type=Placed<Owned<Cell>, "shared">
/// @definition.type symbol=StillShared source="type StillShared = WithSpace<shared ^Cell, \"local\">" value=Placed<Owned<Cell>, "shared">
/// @resolution.name source=WithSpace target=memory.type.WithSpace
/// @resolution.name source=Cell target=Cell

declare const value: StillShared;
/// @type.symbol symbol=value source=value type=Placed<Owned<Cell>, "shared">
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=StillShared target=StillShared

value satisfies shared ^Cell;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=value root=value
/// @resolution.name source=Cell target=Cell
"#,
        r#"

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

declare const readonlyOwned: ^readonly Cell;
declare const exclusiveBorrow: &'static exclusive Cell;

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
/// @type.symbol symbol=ReadonlyOwned source="type ReadonlyOwned = WithAccess<^Cell, \"readonly\">" type=Owned<Readonly<Cell>>
/// @definition.type symbol=ReadonlyOwned source="type ReadonlyOwned = WithAccess<^Cell, \"readonly\">" value=Owned<Readonly<Cell>>
/// @resolution.name source=WithAccess target=memory.type.WithAccess
/// @resolution.name source=Cell target=Cell

type ExclusiveBorrow = WithAccess<Borrowed<Cell, "static">, "exclusive">;
/// @type.symbol symbol=ExclusiveBorrow source="type ExclusiveBorrow = WithAccess<Borrowed<Cell, \"static\">, \"exclusive\">" type=&'static exclusive Cell
/// @definition.type symbol=ExclusiveBorrow source="type ExclusiveBorrow = WithAccess<Borrowed<Cell, \"static\">, \"exclusive\">" value=&'static exclusive Cell
/// @resolution.name source=WithAccess target=memory.type.WithAccess
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell

declare const readonlyOwned: ReadonlyOwned;
/// @type.symbol symbol=readonlyOwned source=readonlyOwned type=Owned<Readonly<Cell>>
/// @resolution.pattern source=readonlyOwned kind=binding target=readonlyOwned
/// @resolution.name source=ReadonlyOwned target=ReadonlyOwned

declare const exclusiveBorrow: ExclusiveBorrow;
/// @type.symbol symbol=exclusiveBorrow source=exclusiveBorrow type=&'static exclusive Cell
/// @resolution.pattern source=exclusiveBorrow kind=binding target=exclusiveBorrow
/// @resolution.name source=ExclusiveBorrow target=ExclusiveBorrow

readonlyOwned satisfies ^readonly Cell;
/// @resolution.name source=readonlyOwned target=readonlyOwned
/// @resolution.place source=readonlyOwned placement="local" lifetime="static" access="readonly"
/// @resolution.access source=readonlyOwned root=readonlyOwned
/// @resolution.name source=Cell target=Cell

exclusiveBorrow satisfies Borrowed<Cell, "static", "exclusive">;
/// @resolution.name source=exclusiveBorrow target=exclusiveBorrow
/// @resolution.place source=exclusiveBorrow placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=exclusiveBorrow root=exclusiveBorrow
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

    session.assert_dir(
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

declare const rebased: shared ^readonly Payload;

rebased satisfies shared ^readonly Payload;

=== dir ===
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
/// @type.symbol symbol=Rebased source="type Rebased = WithBase<shared ^readonly Cell, Payload>" type=Placed<Owned<Readonly<Payload>>, "shared">
/// @definition.type symbol=Rebased source="type Rebased = WithBase<shared ^readonly Cell, Payload>" value=Placed<Owned<Readonly<Payload>>, "shared">
/// @resolution.name source=WithBase target=memory.type.WithBase
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=Payload target=Payload

declare const rebased: Rebased;
/// @type.symbol symbol=rebased source=rebased type=Placed<Owned<Readonly<Payload>>, "shared">
/// @resolution.pattern source=rebased kind=binding target=rebased
/// @resolution.name source=Rebased target=Rebased

rebased satisfies shared ^readonly Payload;
/// @resolution.name source=rebased target=rebased
/// @resolution.place source=rebased placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=rebased root=rebased
/// @resolution.name source=Payload target=Payload
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

declare const borrow: &'static readonly Cell;

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
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Readonly target=types.object.Readonly
/// @resolution.name source=Cell target=Cell

declare const borrow: ReadonlyBorrow;
/// @type.symbol symbol=borrow source=borrow type=&'static readonly Cell
/// @resolution.pattern source=borrow kind=binding target=borrow
/// @resolution.name source=ReadonlyBorrow target=ReadonlyBorrow

borrow satisfies Borrowed<Cell, "static", "readonly">;
/// @resolution.name source=borrow target=borrow
/// @resolution.place source=borrow placement="local" lifetime="static" access="readonly"
/// @resolution.access source=borrow root=borrow
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=Cell target=Cell
"#,
    );
}
