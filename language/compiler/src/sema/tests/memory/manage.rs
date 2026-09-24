use crate::tests::{DirRows, TestSession};

#[test]
fn test_transfer_an_owned_result_into_an_annotated_managed_binding() {
    let session = TestSession::single(
        r#"
class User {}

declare function make(): ^User;

function run(): void {
    let user: User = make();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function make(): ^User;

function run(): void {
    let user: User = make() as User;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let user: User = make();
    /// @type.symbol symbol=run.user source=user type=User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make
    /// @coercion.node source=make() from=^User adjustments=[{ kind: manage, target: User }] origin=implicit

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_take_the_family_default_for_an_unannotated_binding() {
    let session = TestSession::single(
        r#"
class User {}

declare function make(): ^User;

function run(): void {
    let user = make();
    const other = make();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function make(): ^User;

function run(): void {
    let user: ^User = make();
    const other: ^User = make();
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let user = make();
    /// @type.symbol symbol=run.user source=user type=^User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make

    const other = make();
    /// @type.symbol symbol=run.other source=other type=^User
    /// @resolution.pattern source=other kind=binding target=run.other
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_transfer_an_owned_result_into_a_field_an_argument_and_a_return() {
    let session = TestSession::single(
        r#"
class User {}

class Holder {
    user: User;

    constructor(user: User) {
        this.user = user;
    }
}

declare function make(): ^User;
declare function take(user: User): void;

function run(holder: Holder): User {
    holder.user = make();
    take(make());
    return make();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

class Holder {
    user: User;

    constructor(user: User) {
        this.user = user;
    }
}

declare function make(): ^User;
declare function take(user: User): void;

function run(holder: Holder): User {
    holder.user = make() as User;
    take(make() as User);
    return make() as User;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

class Holder {
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder
/// @definition.field symbol=Holder.user source="user: User" key=user type=User
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=(this: &'managed Holder, User) => Holder

    user: User;
    /// @type.symbol symbol=Holder.user source="user: User" type=User
    /// @resolution.name source=User target=User

    constructor(user: User) {
    /// @type.symbol symbol=Holder.constructor type=(this: &'managed Holder, User) => Holder
    /// @type.symbol symbol=Holder.constructor.this type=&'managed Holder
    /// @type.symbol symbol=Holder.constructor.user source="user: User" type=User
    /// @resolution.name source=User target=User

        this.user = user;
        /// @resolution.receiver source=this kind=this declaration=Holder type=&'managed Holder
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.user kind=place
        /// @resolution.place source=this.user placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.user root=this keys=[user]
        /// @resolution.assignment source=this.user write="receiver=&'managed Holder, target=field(receiver=&'managed Holder, target=Holder.user, type=User), type=User" type=User
        /// @resolution.name source=user target=Holder.constructor.user
        /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=user root=Holder.constructor.user

    }
}

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

declare function take(user: User): void;
/// @type.symbol symbol=take source="declare function take(user: User): void" type=(User) => void
/// @resolution.name source=User target=User

function run(holder: Holder): User {
/// @type.symbol symbol=run type=(Holder) => User
/// @type.symbol symbol=run.holder source="holder: Holder" type=Holder
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=User target=User

    holder.user = make();
    /// @resolution.name source=holder target=run.holder
    /// @resolution.place source=holder placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=holder root=run.holder
    /// @resolution.pattern.assign source=holder.user kind=place
    /// @resolution.place source=holder.user placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=holder.user root=run.holder keys=[user]
    /// @resolution.assignment source=holder.user write="receiver=Holder, target=field(receiver=Holder, target=Holder.user, type=User), type=User" type=User
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make
    /// @coercion.node source=make() from=^User adjustments=[{ kind: manage, target: User }] origin=implicit

    take(make());
    /// @resolution.name source=take target=take
    /// @resolution.call source=take(make()) parameters=(User) arguments=(provided(make()) as User) return=void kind=symbol target=take
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make
    /// @coercion.node source=make() from=^User adjustments=[{ kind: manage, target: User }] origin=implicit

    return make();
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make
    /// @coercion.node source=make() from=^User adjustments=[{ kind: manage, target: User }] origin=implicit

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_transfer_an_owned_result_into_an_array_literal_member() {
    let session = TestSession::single(
        r#"
class User {}

declare function make(): ^User;

function run(): void {
    let users: User[] = [make()];
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function make(): ^User;

function run(): void {
    let users: User[] = [make() as User];
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let users: User[] = [make()];
    /// @type.symbol symbol=run.users source=users type=User[]
    /// @resolution.pattern source=users kind=binding target=run.users
    /// @resolution.name source=User target=User
    /// @resolution.call source=[make()] parameters=(^Slice<User>) arguments=(rest(provided(make()) as User) as User) return=User[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<User>
    /// @generic.instantiation id=arrayFromOwnedSlice<User> template=arrayFromOwnedSlice arguments=(User)
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make
    /// @coercion.node source=make() from=^User adjustments=[{ kind: manage, target: User }] origin=implicit

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_infer_a_bare_type_parameter_at_the_family_default() {
    let session = TestSession::single(
        r#"
class User {}

declare function make(): ^User;
declare function identity<T>(value: T): T;
declare function first<T>(value: T | undefined): T;

function run(): void {
    let user = identity(make());
    let found = first(make());
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function make(): ^User;
declare function identity<T>(value: T): T;
declare function first<T>(value: T | undefined): T;

function run(): void {
    let user: ^User = identity<^User>(make());
    let found: ^User = first<^User>(make() as ^User | undefined);
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

declare function identity<T>(value: T): T;
/// @generic.template symbol=identity parameters=(T#1)
/// @type.symbol symbol=identity source="declare function identity<T>(value: T): T" type=<T#1>(T#1) => T#1
/// @type.symbol symbol=identity.T source=T type=T#1
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

declare function first<T>(value: T | undefined): T;
/// @generic.template symbol=first parameters=(T#2)
/// @type.symbol symbol=first source="declare function first<T>(value: T | undefined): T" type=<T#2>(T#2 | undefined) => T#2
/// @type.symbol symbol=first.T source=T type=T#2
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T

function run(): void {
/// @type.symbol symbol=run type=() => void

    let user = identity(make());
    /// @type.symbol symbol=run.user source=user type=^User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=identity target=identity
    /// @resolution.call source=identity(make()) parameters=(^User) arguments=(provided(make()) as ^User) return=^User kind=symbol target=identity instance=identity<^User>
    /// @generic.instantiation id=identity<^User> template=identity arguments=(^User)
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make

    let found = first(make());
    /// @type.symbol symbol=run.found source=found type=^User
    /// @resolution.pattern source=found kind=binding target=run.found
    /// @resolution.name source=first target=first
    /// @resolution.call source=first(make()) parameters=(^User | undefined) arguments=(provided(make()) as ^User | undefined) return=^User kind=symbol target=first instance=first<^User>
    /// @generic.instantiation id=first<^User> template=first arguments=(^User)
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make
    /// @coercion.node source=make() from=^User adjustments=[{ kind: union, target: ^User | undefined, cases: ({ source: ^User, target: ^User }) }] origin=implicit

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_infer_an_unannotated_return_at_the_family_default() {
    let session = TestSession::single(
        r#"
class User {}

declare function make(): ^User;

const get = () => make();
const block = () => {
    return make();
};
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function make(): ^User;

const get: () => ^User = (): ^User => make();
const block: () => ^User = (): ^User => {
    return make();
};

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

const get = () => make();
/// @type.symbol symbol=get source=get type=Function<(), ^User, "readonly">
/// @resolution.pattern source=get kind=binding target=get
/// @type.symbol symbol=symbol3 source=() => make() type=Function<(), ^User, "readonly">
/// @resolution.name source=make target=make
/// @resolution.call source=make() parameters=() return=^User kind=symbol target=make

const block = () => {
/// @type.symbol symbol=block source=block type=Function<(), ^User, "readonly">
/// @resolution.pattern source=block kind=binding target=block
/// @type.symbol symbol=symbol5 type=Function<(), ^User, "readonly">

    return make();
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make

};
"#,
        r#"
"#,
    );
}

#[test]
fn test_keep_the_owned_form_where_it_is_spelled() {
    let session = TestSession::single(
        r#"
class User {}

declare function make(): ^User;
declare function identity<T>(value: T): T;
declare function pair(): (^User, ^User);

function run(): void {
    let owned: ^User = make();
    let explicit = identity<^User>(make());
    let users = pair();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function make(): ^User;
declare function identity<T>(value: T): T;
declare function pair(): (^User, ^User);

function run(): void {
    let owned: ^User = make();
    let explicit: ^User = identity<^User>(make());
    let users: (^User, ^User) = pair();
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

declare function identity<T>(value: T): T;
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity source="declare function identity<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

declare function pair(): (^User, ^User);
/// @type.symbol symbol=pair source="declare function pair(): (^User, ^User)" type=() => (^User, ^User)
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let owned: ^User = make();
    /// @type.symbol symbol=run.owned source=owned type=^User
    /// @resolution.pattern source=owned kind=binding target=run.owned
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make

    let explicit = identity<^User>(make());
    /// @type.symbol symbol=run.explicit source=explicit type=^User
    /// @resolution.pattern source=explicit kind=binding target=run.explicit
    /// @resolution.name source=identity target=identity
    /// @resolution.call source=identity<^User>(make()) parameters=(^User) arguments=(provided(make()) as ^User) return=^User kind=symbol target=identity instance=identity<^User>
    /// @generic.instantiation id=identity<^User> template=identity arguments=(^User)
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make

    let users = pair();
    /// @type.symbol symbol=run.users source=users type=(^User, ^User)
    /// @resolution.pattern source=users kind=binding target=run.users
    /// @resolution.name source=pair target=pair
    /// @resolution.call source=pair() parameters=() return=(^User, ^User) kind=symbol target=pair

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_take_an_owned_value_family_result_by_value() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

declare function make(): ^Point;

function run(): void {
    let point = make();
    let annotated: Point = make();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

declare function make(): ^Point;

function run(): void {
    let point: Point = make();
    let annotated: Point = make();
}

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

declare function make(): ^Point;
/// @type.symbol symbol=make source="declare function make(): ^Point" type=() => Point
/// @resolution.name source=Point target=Point

function run(): void {
/// @type.symbol symbol=run type=() => void

    let point = make();
    /// @type.symbol symbol=run.point source=point type=Point
    /// @resolution.pattern source=point kind=binding target=run.point
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Point kind=symbol target=make

    let annotated: Point = make();
    /// @type.symbol symbol=run.annotated source=annotated type=Point
    /// @resolution.pattern source=annotated kind=binding target=run.annotated
    /// @resolution.name source=Point target=Point
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Point kind=symbol target=make

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_transfer_an_owned_value_explicitly_into_managed() {
    let session = TestSession::single(
        r#"
class User {}

declare function make(): ^User;

function run(): void {
    let user: User = make().intoManaged();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function make(): ^User;

function run(): void {
    let user: User = make().intoManaged<User>();
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let user: User = make().intoManaged();
    /// @type.symbol symbol=run.user source=user type=User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.member source=make().intoManaged receiver=^User type=(this: ^User) => User kind=symbol target_receiver=^User target=intoManaged#1
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make
    /// @resolution.call source=make().intoManaged() parameters=() return=User kind=symbol target=intoManaged#1 receiver=^User instance=^T.<extension#1>.intoManaged#1
    /// @generic.instantiation id=intoManaged#1<User> template=intoManaged#1 arguments=(User)

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_move_an_owned_binding_into_a_managed_destination() {
    let session = TestSession::single(
        r#"
class User {}

declare function make(): ^User;
declare function take(user: User): void;

function run(): void {
    let owned: ^User = make();
    take(owned);
    take(owned);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function make(): ^User;
declare function take(user: User): void;

function run(): void {
    let owned: ^User = make();
    take(owned as User);
    take(owned as User);
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

declare function take(user: User): void;
/// @type.symbol symbol=take source="declare function take(user: User): void" type=(User) => void
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let owned: ^User = make();
    /// @type.symbol symbol=run.owned source=owned type=^User
    /// @resolution.pattern source=owned kind=binding target=run.owned
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make

    take(owned);
    /// @resolution.name source=take target=take
    /// @resolution.call source=take(owned) parameters=(User) arguments=(provided(owned) as User) return=void kind=symbol target=take
    /// @resolution.name source=owned target=run.owned
    /// @resolution.place source=owned placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=owned root=run.owned
    /// @coercion.node source=owned from=^User adjustments=[{ kind: manage, target: User }] origin=implicit

    take(owned);
    /// @resolution.name source=take target=take
    /// @resolution.call source=take(owned) parameters=(User) arguments=(provided(owned) as User) return=void kind=symbol target=take
    /// @resolution.name source=owned target=run.owned
    /// @resolution.place source=owned placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=owned root=run.owned
    /// @coercion.node source=owned from=^User adjustments=[{ kind: manage, target: User }] origin=implicit

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_select_a_blanket_member_over_the_owned_form() {
    let session = TestSession::single(
        r#"
import { Owned } from "destack:memory";

class User {}

extension<T> of Owned<T> {
    release(this: ^T): T {
        return this;
    }
}

extension<T> of ^T {
    unwrap(this: ^T): T {
        return this;
    }
}

declare function make(): ^User;

function run(): void {
    let user: User = make().release();
    let other: User = make().unwrap();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
import { Owned } from "destack:memory";

class User {}

extension<T> of Owned<T> {
    release(this: ^T): T {
        return this as T;
    }
}

extension<T> of ^T {
    unwrap(this: ^T): T {
        return this as T;
    }
}

declare function make(): ^User;

function run(): void {
    let user: User = make().release<User>();
    let other: User = make().unwrap<User>();
}

=== dir ===
import { Owned } from "destack:memory";

class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

extension<T> of Owned<T> {
/// @generic.template symbol=<module>#2 parameters=(T#1)
/// @definition.extension symbol=<module>#2 form=local target=^T#1
/// @definition.method symbol=release slot=release type=(this: ^T#1) => T#1
/// @type.symbol symbol=T#1 source=T type=T#1
/// @resolution.name source=Owned target=Owned
/// @resolution.name source=T target=T#1

    release(this: ^T): T {
    /// @type.symbol symbol=release type=(this: ^T#1) => T#1
    /// @type.symbol symbol=release.this source="this: ^T" type=^T#1
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=T target=T#1

        return this;
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=^T#1
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @coercion.node source=this from=^T#1 adjustments=[{ kind: representation, target: T#1 }] origin=implicit

    }
}

extension<T> of ^T {
/// @generic.template symbol=<module>#3 parameters=(T#2)
/// @definition.extension symbol=<module>#3 form=local target=^T#2
/// @definition.method symbol=unwrap slot=unwrap type=(this: ^T#2) => T#2
/// @type.symbol symbol=T#2 source=T type=T#2
/// @resolution.name source=T target=T#2

    unwrap(this: ^T): T {
    /// @type.symbol symbol=unwrap type=(this: ^T#2) => T#2
    /// @type.symbol symbol=unwrap.this source="this: ^T" type=^T#2
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=T target=T#2

        return this;
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=^T#2
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @coercion.node source=this from=^T#2 adjustments=[{ kind: representation, target: T#2 }] origin=implicit

    }
}

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let user: User = make().release();
    /// @type.symbol symbol=run.user source=user type=User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.member source=make().release receiver=^User type=(this: ^User) => User kind=symbol target_receiver=^User target=release
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make
    /// @resolution.call source=make().release() parameters=() return=User kind=symbol target=release receiver=^User instance=^T#1.<extension#1>.release
    /// @generic.instantiation id=release<User> template=release arguments=(User)

    let other: User = make().unwrap();
    /// @type.symbol symbol=run.other source=other type=User
    /// @resolution.pattern source=other kind=binding target=run.other
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.member source=make().unwrap receiver=^User type=(this: ^User) => User kind=symbol target_receiver=^User target=unwrap
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make
    /// @resolution.call source=make().unwrap() parameters=() return=User kind=symbol target=unwrap receiver=^User instance=^T#2.<extension#2>.unwrap
    /// @generic.instantiation id=unwrap<User> template=unwrap arguments=(User)

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_move_an_owned_place_through_unannotated_destinations() {
    let session = TestSession::single(
        r#"
class User {}

declare function identity<T>(value: T): T;

function forward(input: ^User): ^User {
    let moved = input;
    return moved;
}

function pass(input: ^User): ^User {
    return identity(input);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function identity<T>(value: T): T;

function forward(input: ^User): ^User {
    let moved: ^User = input;
    return moved;
}

function pass(input: ^User): ^User {
    return identity<^User>(input);
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function identity<T>(value: T): T;
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity source="declare function identity<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

function forward(input: ^User): ^User {
/// @type.symbol symbol=forward type=(^User) => ^User
/// @type.symbol symbol=forward.input source="input: ^User" type=^User
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

    let moved = input;
    /// @type.symbol symbol=forward.moved source=moved type=^User
    /// @resolution.pattern source=moved kind=binding target=forward.moved
    /// @resolution.name source=input target=forward.input
    /// @resolution.access source=input root=forward.input

    return moved;
    /// @resolution.name source=moved target=forward.moved
    /// @resolution.place source=moved placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=moved root=forward.moved

}

function pass(input: ^User): ^User {
/// @type.symbol symbol=pass type=(^User) => ^User
/// @type.symbol symbol=pass.input source="input: ^User" type=^User
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

    return identity(input);
    /// @resolution.name source=identity target=identity
    /// @resolution.call source=identity(input) parameters=(^User) arguments=(provided(input) as ^User) return=^User kind=symbol target=identity instance=identity<^User>
    /// @generic.instantiation id=identity<^User> template=identity arguments=(^User)
    /// @resolution.name source=input target=pass.input
    /// @resolution.place source=input placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=input root=pass.input

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_keep_an_owned_temporary_through_a_form_hole_annotation() {
    let session = TestSession::single(
        r#"
class User {}

declare function make(): ^User;
declare function names(): ^string[];

function run(): void {
    const user: ^_ = make();
    const owned: ^_ = names();
    const managed = names();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function make(): ^User;
declare function names(): ^string[];

function run(): void {
    const user: ^User = make();
    const owned: ^string[] = names();
    const managed: ^string[] = names();
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => ^User
/// @resolution.name source=User target=User

declare function names(): ^string[];
/// @type.symbol symbol=names source="declare function names(): ^string[]" type=() => ^string[]

function run(): void {
/// @type.symbol symbol=run type=() => void

    const user: ^_ = make();
    /// @type.symbol symbol=run.user source=user type=^User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=^User kind=symbol target=make

    const owned: ^_ = names();
    /// @type.symbol symbol=run.owned source=owned type=^string[]
    /// @resolution.pattern source=owned kind=binding target=run.owned
    /// @resolution.name source=names target=names
    /// @resolution.call source=names() parameters=() return=^string[] kind=symbol target=names

    const managed = names();
    /// @type.symbol symbol=run.managed source=managed type=^string[]
    /// @resolution.pattern source=managed kind=binding target=run.managed
    /// @resolution.name source=names target=names
    /// @resolution.call source=names() parameters=() return=^string[] kind=symbol target=names

}
"#,
        r#"
"#,
    );
}

/// Complete a generic default into the form its declaration writes.
#[test]
fn test_complete_a_generic_default_into_its_declared_form() {
    let session = TestSession::single(
        r#"
struct Slot<T> {
    value: T;
}

export extension<T> of Slot<T> {
    static of(value: T): Slot<T> {
        Slot { value }
    }
}

function make<T: Default>(): T {
    return T.default();
}

function fill<T: Default>(): Slot<T> {
    return Slot.of(T.default());
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
struct Slot<out T> {
    value: T;
}

export extension<T> of Slot<T> {
    static of(value: T): Slot<T> {
        Slot<T> { value }
    }
}

function make<T: Default>(): T {
    return T.default() as T;
}

function fill<T: Default>(): Slot<T> {
    return Slot.of<T>(T.default() as T);
}

=== dir ===
struct Slot<T> {
/// @generic.template symbol=Slot parameters=(out T#1)
/// @type.symbol symbol=Slot type=Slot
/// @definition.struct symbol=Slot template=(out T#1)
/// @definition.field symbol=Slot.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Slot.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Slot.value source="value: T" type=T#1
    /// @resolution.name source=T target=Slot.T

}

export extension<T> of Slot<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @generic.instance id=Slot<T#2> template=Slot arguments=(T#2)
/// @definition.extension symbol=<module>#2 form=exported target=Slot<T#2>
/// @definition.method symbol=of slot=of static=true type=(T#2) => Slot<T#2>
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Slot target=Slot
/// @resolution.name source=T target=T

    static of(value: T): Slot<T> {
    /// @type.symbol symbol=of type=(T#2) => Slot<T#2>
    /// @type.symbol symbol=of.value source="value: T" type=T#2
    /// @resolution.name source=T target=T
    /// @resolution.name source=Slot target=Slot
    /// @resolution.name source=T target=T

        Slot { value }
        /// @type.node source="Slot { value }" type=Slot<T#2>
        /// @resolution.name source=Slot target=Slot
        /// @type.node source=value type=T#2
        /// @resolution.name source=value target=of.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=of.value

    }
}

function make<T: Default>(): T {
/// @generic.template symbol=make parameters=(T#3: Default)
/// @type.symbol symbol=make type=<T#3: Default>() => T#3
/// @type.symbol symbol=make.T source="T: Default" type=T#3
/// @resolution.name source=Default target=Default
/// @resolution.name source=T target=make.T

    return T.default();
    /// @type.node source=T type=T#3
    /// @type.node source=T.default type=() => ^T#3
    /// @type.node source=T.default() type=^T#3
    /// @resolution.name source=T target=make.T
    /// @resolution.member source=T.default receiver=T#3 type=() => ^T#3 kind=symbol target_receiver=T#3 target=Default.default
    /// @resolution.call source=T.default() parameters=() return=^T#3 kind=symbol target=Default.default
    /// @generic.instantiation id=Default.default<T#3> template=Default.default arguments=() owner=make
    /// @generic.instance id=Default.default<T#3> template=Default.default arguments=()
    /// @coercion.node source=T.default() from=^T#3 adjustments=[{ kind: representation, target: T#3 }] origin=implicit

}

function fill<T: Default>(): Slot<T> {
/// @generic.template symbol=fill parameters=(T#4: Default)
/// @type.symbol symbol=fill type=<T#4: Default>() => Slot<T#4>
/// @generic.instance id=Slot<T#4> template=Slot arguments=(T#4)
/// @type.symbol symbol=fill.T source="T: Default" type=T#4
/// @resolution.name source=Default target=Default
/// @resolution.name source=Slot target=Slot
/// @resolution.name source=T target=fill.T

    return Slot.of(T.default());
    /// @type.node source=Slot type=Slot
    /// @type.node source=Slot.of type=(T#2) => Slot<T#2>
    /// @type.node source=Slot.of(T.default()) type=Slot<T#4>
    /// @resolution.name source=Slot target=Slot
    /// @resolution.member source=Slot.of receiver=Slot type=(T#2) => Slot<T#2> kind=symbol target_receiver=Slot target=of
    /// @resolution.call source=Slot.of(T.default()) parameters=(T#4) arguments=(provided(T.default()) as T#4) return=Slot<T#4> kind=symbol target=of instance=Slot<T#4>.<extension#1>.of
    /// @generic.instantiation id=of<T#4> template=of arguments=(T#4) owner=fill
    /// @generic.instance id=of<T#4> template=of arguments=(T#4)
    /// @type.node source=T type=T#4
    /// @type.node source=T.default type=() => ^T#4
    /// @type.node source=T.default() type=^T#4
    /// @resolution.name source=T target=fill.T
    /// @resolution.member source=T.default receiver=T#4 type=() => ^T#4 kind=symbol target_receiver=T#4 target=Default.default
    /// @resolution.call source=T.default() parameters=() return=^T#4 kind=symbol target=Default.default
    /// @generic.instantiation id=Default.default<T#4> template=Default.default arguments=() owner=fill
    /// @generic.instance id=Default.default<T#4> template=Default.default arguments=()
    /// @coercion.node source=T.default() from=^T#4 adjustments=[{ kind: representation, target: T#4 }] origin=implicit

}
"#,
    );
}
