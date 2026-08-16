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
type LocalBorrowed<'a> = local &'a readonly User;
type BorrowedLocal<'a> = &'a readonly local User;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

type LocalReadonly = local readonly User;
type ReadonlyLocal = readonly local User;
type SharedOwned = shared ^User;
type OwnedShared = ^shared User;
type LocalBorrowed<'a> = local &'a readonly User;
type BorrowedLocal<'a> = &'a readonly local User;

=== dir ===
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

type LocalBorrowed<'a> = local &'a readonly User;
/// @generic.template symbol=LocalBorrowed parameters=('a#1)
/// @type.symbol symbol=LocalBorrowed source="type LocalBorrowed<'a> = local &'a readonly User" type=Placed<&'a#1 readonly User, "local">
/// @definition.type symbol=LocalBorrowed source="type LocalBorrowed<'a> = local &'a readonly User" template=('a#1) value=Placed<&'a#1 readonly User, "local">
/// @type.symbol symbol=LocalBorrowed.'a source='a type='a#1
/// @resolution.name source='a target=LocalBorrowed.'a
/// @resolution.name source=User target=User

type BorrowedLocal<'a> = &'a readonly local User;
/// @generic.template symbol=BorrowedLocal parameters=('a#2)
/// @type.symbol symbol=BorrowedLocal source="type BorrowedLocal<'a> = &'a readonly local User" type=Placed<&'a#2 readonly User, "local">
/// @definition.type symbol=BorrowedLocal source="type BorrowedLocal<'a> = &'a readonly local User" template=('a#2) value=Placed<&'a#2 readonly User, "local">
/// @type.symbol symbol=BorrowedLocal.'a source='a type='a#2
/// @resolution.name source='a target=BorrowedLocal.'a
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const localUser: local User;
declare const sharedUser: shared User;

const localFromShared: local User = sharedUser;
const sharedFromLocal: shared User = localUser;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=Placed<User, "local">
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<User, "shared">
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

const localFromShared: local User = sharedUser;
/// @type.symbol symbol=localFromShared source=localFromShared type=Placed<User, "local">
/// @resolution.pattern source=localFromShared kind=binding target=localFromShared
/// @resolution.name source=User target=User
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser

const sharedFromLocal: shared User = localUser;
/// @type.symbol symbol=sharedFromLocal source=sharedFromLocal type=Placed<User, "shared">
/// @resolution.pattern source=sharedFromLocal kind=binding target=sharedFromLocal
/// @resolution.name source=User target=User
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localUser root=localUser
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'shared User' is not assignable to type 'local User'"
/// @diagnostic.label line=7 column=37 span="sharedUser" line_source="const localFromShared: local User = sharedUser;"
/// @diagnostic.related line=7 column=24 span="local" line_source="const localFromShared: local User = sharedUser;" message="expected due to this annotation"
/// @diagnostic.note message="a value never changes its space"
/// @diagnostic.help message="use a value in the destination placement or create a new value there"
/// @diagnostic.error id=not-assignable message="type 'local User' is not assignable to type 'shared User'"
/// @diagnostic.label line=8 column=38 span="localUser" line_source="const sharedFromLocal: shared User = localUser;"
/// @diagnostic.related line=8 column=24 span="shared" line_source="const sharedFromLocal: shared User = localUser;" message="expected due to this annotation"
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}
class Team {}

type LocalChoice = local (User | shared Team);
type CarrierPlace = PlaceOf<LocalChoice>;

declare const localChoice: local (User | shared Team);
declare const carrierPlace: "local";

localChoice satisfies local (User | shared Team);
carrierPlace satisfies "local";

=== dir ===
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
/// @type.symbol symbol=CarrierPlace source="type CarrierPlace = PlaceOf<LocalChoice>" type="local"
/// @definition.type symbol=CarrierPlace source="type CarrierPlace = PlaceOf<LocalChoice>" value="local"
/// @resolution.name source=PlaceOf target=memory.type.PlaceOf
/// @resolution.name source=LocalChoice target=LocalChoice

declare const localChoice: LocalChoice;
/// @type.symbol symbol=localChoice source=localChoice type=Placed<User | Placed<Team, "shared">, "local">
/// @resolution.pattern source=localChoice kind=binding target=localChoice
/// @resolution.name source=LocalChoice target=LocalChoice

declare const carrierPlace: CarrierPlace;
/// @type.symbol symbol=carrierPlace source=carrierPlace type="local"
/// @resolution.pattern source=carrierPlace kind=binding target=carrierPlace
/// @resolution.name source=CarrierPlace target=CarrierPlace

localChoice satisfies local (User | shared Team);
/// @resolution.name source=localChoice target=localChoice
/// @resolution.place source=localChoice placement="local" lifetime="static" access="readonly"
/// @resolution.access source=localChoice root=localChoice
/// @resolution.name source=User target=User
/// @resolution.name source=Team target=Team

carrierPlace satisfies "local";
/// @resolution.name source=carrierPlace target=carrierPlace
/// @resolution.place source=carrierPlace placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=carrierPlace root=carrierPlace
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const localUser: local ^User;
declare const sharedUser: shared ^User;

const sharedFromLocal: shared ^User = localUser;
const localFromShared: local ^User = sharedUser;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const localUser: local ^User;
/// @type.symbol symbol=localUser source=localUser type=Placed<Owned<User>, "local">
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared ^User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<Owned<User>, "shared">
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

const sharedFromLocal: shared ^User = localUser;
/// @type.symbol symbol=sharedFromLocal source=sharedFromLocal type=Placed<Owned<User>, "shared">
/// @resolution.pattern source=sharedFromLocal kind=binding target=sharedFromLocal
/// @resolution.name source=User target=User
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="readonly"
/// @resolution.access source=localUser root=localUser

const localFromShared: local ^User = sharedUser;
/// @type.symbol symbol=localFromShared source=localFromShared type=Placed<Owned<User>, "local">
/// @resolution.pattern source=localFromShared kind=binding target=localFromShared
/// @resolution.name source=User target=User
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=sharedUser root=sharedUser
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'local ^User' is not assignable to type 'shared ^User'"
/// @diagnostic.label line=7 column=39 span="localUser" line_source="const sharedFromLocal: shared ^User = localUser;"
/// @diagnostic.related line=7 column=24 span="shared" line_source="const sharedFromLocal: shared ^User = localUser;" message="expected due to this annotation"
/// @diagnostic.note message="a value never changes its space"
/// @diagnostic.help message="use a value in the destination placement or create a new value there"
/// @diagnostic.error id=not-assignable message="type 'shared ^User' is not assignable to type 'local ^User'"
/// @diagnostic.label line=8 column=38 span="sharedUser" line_source="const localFromShared: local ^User = sharedUser;"
/// @diagnostic.related line=8 column=24 span="local" line_source="const localFromShared: local ^User = sharedUser;" message="expected due to this annotation"
/// @diagnostic.note message="a value never changes its space"
/// @diagnostic.help message="use a value in the destination placement or create a new value there"
"#,
    );
}
