use crate::tests::{DirRows, TestSession};

#[test]
fn test_placement_commutes_with_readonly_ownership_and_borrowing() {
    let session = TestSession::single(
        r#"
class User {}

type LocalReadonly = local readonly User;
type ReadonlyLocal = readonly local User;
type SharedOwned = shared ^User;
type OwnedShared = ^shared User;
type LocalBorrowed = local &readonly User;
type BorrowedLocal = &readonly local User;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

type LocalReadonly = local readonly User;
type ReadonlyLocal = readonly local User;
type SharedOwned = shared ^User;
type OwnedShared = ^shared User;
type LocalBorrowed<comptime L0: Lifetime> = local &readonly User;
type BorrowedLocal<comptime L0: Lifetime> = &readonly local User;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

type LocalReadonly = local readonly User;
/// @type.symbol symbol=LocalReadonly source="type LocalReadonly = local readonly User" type=Placed<Readonly<User>, "local">
/// @definition.type symbol=LocalReadonly source="type LocalReadonly = local readonly User" value=Placed<Readonly<User>, "local">
/// @resolution.name source=User target=User

type ReadonlyLocal = readonly local User;
/// @type.symbol symbol=ReadonlyLocal source="type ReadonlyLocal = readonly local User" type=Placed<Readonly<User>, "local">
/// @definition.type symbol=ReadonlyLocal source="type ReadonlyLocal = readonly local User" value=Placed<Readonly<User>, "local">
/// @resolution.name source=User target=User

type SharedOwned = shared ^User;
/// @type.symbol symbol=SharedOwned source="type SharedOwned = shared ^User" type=Placed<Owned<User>, "shared">
/// @definition.type symbol=SharedOwned source="type SharedOwned = shared ^User" value=Placed<Owned<User>, "shared">
/// @resolution.name source=User target=User

type OwnedShared = ^shared User;
/// @type.symbol symbol=OwnedShared source="type OwnedShared = ^shared User" type=Placed<Owned<User>, "shared">
/// @definition.type symbol=OwnedShared source="type OwnedShared = ^shared User" value=Placed<Owned<User>, "shared">
/// @resolution.name source=User target=User

type LocalBorrowed = local &readonly User;
/// @generic.template symbol=LocalBorrowed parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=LocalBorrowed source="type LocalBorrowed = local &readonly User" type=Placed<Borrowed<User, LocalBorrowed.L0, "readonly">, "local">
/// @definition.type symbol=LocalBorrowed source="type LocalBorrowed = local &readonly User" value=Placed<Borrowed<User, LocalBorrowed.L0, "readonly">, "local">
/// @resolution.name source=User target=User

type BorrowedLocal = &readonly local User;
/// @generic.template symbol=BorrowedLocal parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=BorrowedLocal source="type BorrowedLocal = &readonly local User" type=Placed<Borrowed<User, BorrowedLocal.L0, "readonly">, "local">
/// @definition.type symbol=BorrowedLocal source="type BorrowedLocal = &readonly local User" value=Placed<Borrowed<User, BorrowedLocal.L0, "readonly">, "local">
/// @resolution.name source=User target=User
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_relabeling_managed_values_across_spaces() {
    let session = TestSession::single(
        r#"
class User {}

declare const localUser: local User;
declare const sharedUser: shared User;

const localFromShared: local User = sharedUser;
const sharedFromLocal: shared User = localUser;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const localUser: local User;
declare const sharedUser: shared User;

const localFromShared: local User = sharedUser;
const sharedFromLocal: shared User = localUser;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=Placed<User, "local">
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<User, "shared">
/// @resolution.name source=User target=User

const localFromShared: local User = sharedUser;
/// @type.symbol symbol=localFromShared source=localFromShared type=Placed<User, "local">
/// @resolution.name source=User target=User
/// @resolution.name source=sharedUser target=sharedUser

const sharedFromLocal: shared User = localUser;
/// @type.symbol symbol=sharedFromLocal source=sharedFromLocal type=Placed<User, "shared">
/// @resolution.name source=User target=User
/// @resolution.name source=localUser target=localUser
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'shared User' is not assignable to type 'local User'"
/// @diagnostic.label line=7 column=37 span="sharedUser" line_source="const localFromShared: local User = sharedUser;"
/// @diagnostic.note message="a value never changes its space"
/// @diagnostic.help message="use a value in the destination placement or create a new value there"
/// @diagnostic.error code=EC200 message="type 'local User' is not assignable to type 'shared User'"
/// @diagnostic.label line=8 column=38 span="localUser" line_source="const sharedFromLocal: shared User = localUser;"
/// @diagnostic.note message="a value never changes its space"
/// @diagnostic.help message="use a value in the destination placement or create a new value there"
"#,
    );
}

#[test]
fn test_place_union_carrier_without_distributing() {
    let session = TestSession::single(
        r#"
class User {}
class Team {}

type LocalChoice = local (User | shared Team);
type CarrierPlace = PlaceOf<LocalChoice>;

declare const localChoice: LocalChoice;
declare const carrierPlace: CarrierPlace;

localChoice satisfies local (User | shared Team);
carrierPlace satisfies "local";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}
class Team {}

type LocalChoice = local (User | shared Team);
type CarrierPlace = PlaceOf<LocalChoice>;

declare const localChoice: LocalChoice;
declare const carrierPlace: CarrierPlace;

localChoice satisfies local (User | shared Team);
carrierPlace satisfies "local";

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

class Team {}
/// @type.symbol symbol=Team source="class Team {}" type=Team
/// @definition.class symbol=Team source="class Team {}"

type LocalChoice = local (User | shared Team);
/// @type.symbol symbol=LocalChoice source="type LocalChoice = local (User | shared Team)" type=Placed<User | Placed<Team, "shared">, "local">
/// @definition.type symbol=LocalChoice source="type LocalChoice = local (User | shared Team)" value=Placed<User | Placed<Team, "shared">, "local">
/// @resolution.name source=User target=User
/// @resolution.name source=Team target=Team

type CarrierPlace = PlaceOf<LocalChoice>;
/// @type.symbol symbol=CarrierPlace source="type CarrierPlace = PlaceOf<LocalChoice>" type=PlaceOf<LocalChoice> reduced="local"
/// @definition.type symbol=CarrierPlace source="type CarrierPlace = PlaceOf<LocalChoice>" value=PlaceOf<LocalChoice> reduced="local"
/// @resolution.name source=PlaceOf target=memory.type.PlaceOf
/// @resolution.name source=LocalChoice target=LocalChoice

declare const localChoice: LocalChoice;
/// @type.symbol symbol=localChoice source=localChoice type=LocalChoice reduced=Placed<User | Placed<Team, "shared">, "local">
/// @resolution.name source=LocalChoice target=LocalChoice

declare const carrierPlace: CarrierPlace;
/// @type.symbol symbol=carrierPlace source=carrierPlace type=CarrierPlace reduced="local"
/// @resolution.name source=CarrierPlace target=CarrierPlace

localChoice satisfies local (User | shared Team);
/// @resolution.name source=localChoice target=localChoice
/// @resolution.name source=User target=User
/// @resolution.name source=Team target=Team

carrierPlace satisfies "local";
/// @resolution.name source=carrierPlace target=carrierPlace

/// @generic.instance id=PlaceOf<LocalChoice> template=memory.type.PlaceOf arguments=(LocalChoice)
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_relabeling_owned_values_across_spaces() {
    let session = TestSession::single(
        r#"
class User {}

declare const localUser: local ^User;
declare const sharedUser: shared ^User;

const sharedFromLocal: shared ^User = localUser;
const localFromShared: local ^User = sharedUser;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const localUser: local ^User;
declare const sharedUser: shared ^User;

const sharedFromLocal: shared ^User = localUser;
const localFromShared: local ^User = sharedUser;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const localUser: local ^User;
/// @type.symbol symbol=localUser source=localUser type=Placed<Owned<User>, "local">
/// @resolution.name source=User target=User

declare const sharedUser: shared ^User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<Owned<User>, "shared">
/// @resolution.name source=User target=User

const sharedFromLocal: shared ^User = localUser;
/// @type.symbol symbol=sharedFromLocal source=sharedFromLocal type=Placed<Owned<User>, "shared">
/// @resolution.name source=User target=User
/// @resolution.name source=localUser target=localUser

const localFromShared: local ^User = sharedUser;
/// @type.symbol symbol=localFromShared source=localFromShared type=Placed<Owned<User>, "local">
/// @resolution.name source=User target=User
/// @resolution.name source=sharedUser target=sharedUser
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'local ^User' is not assignable to type 'shared ^User'"
/// @diagnostic.label line=7 column=39 span="localUser" line_source="const sharedFromLocal: shared ^User = localUser;"
/// @diagnostic.note message="a value never changes its space"
/// @diagnostic.help message="use a value in the destination placement or create a new value there"
/// @diagnostic.error code=EC200 message="type 'shared ^User' is not assignable to type 'local ^User'"
/// @diagnostic.label line=8 column=38 span="sharedUser" line_source="const localFromShared: local ^User = sharedUser;"
/// @diagnostic.note message="a value never changes its space"
/// @diagnostic.help message="use a value in the destination placement or create a new value there"
"#,
    );
}
