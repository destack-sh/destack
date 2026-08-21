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
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let user: User = make();
    /// @type.symbol symbol=run.user source=user type=User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

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
    let user: User = make() as User;
    const other: User = make() as User;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let user = make();
    /// @type.symbol symbol=run.user source=user type=User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

    const other = make();
    /// @type.symbol symbol=run.other source=other type=User
    /// @resolution.pattern source=other kind=binding target=run.other
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

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

    constructor(user: User): this {
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
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

class Holder {
/// @type.symbol symbol=Holder type=Holder
/// @definition.class symbol=Holder
/// @definition.field symbol=Holder.user source="user: User" key=user type=User
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=(User) => this

    user: User;
    /// @type.symbol symbol=Holder.user source="user: User" type=User
    /// @resolution.name source=User target=User

    constructor(user: User) {
    /// @type.symbol symbol=Holder.constructor type=(User) => this
    /// @type.symbol symbol=Holder.constructor.user source="user: User" type=User
    /// @resolution.name source=User target=User

        this.user = user;
        /// @resolution.receiver source=this kind=this declaration=Holder type=Holder
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.user kind=place
        /// @resolution.access source=this.user root=this keys=[user]
        /// @resolution.assignment source=this.user write="receiver=Holder, target=field(receiver=Holder, target=Holder.user, type=User), type=User" type=User
        /// @resolution.name source=user target=Holder.constructor.user
        /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=user root=Holder.constructor.user

    }
}

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

declare function take(user: User): void;
/// @type.symbol symbol=take source="declare function take(user: User): void" type=(User) => void
/// @type.symbol symbol=take.user source="user: User" type=User
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
    /// @resolution.access source=holder.user root=run.holder keys=[user]
    /// @resolution.assignment source=holder.user write="receiver=Holder, target=field(receiver=Holder, target=Holder.user, type=User), type=User" type=User
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

    take(make());
    /// @resolution.name source=take target=take
    /// @resolution.call source=take(make()) parameters=(User) arguments=(provided(make()) as User) return=void kind=symbol target=take
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

    return make();
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

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
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let users: User[] = [make()];
    /// @type.symbol symbol=run.users source=users type=User[]
    /// @resolution.pattern source=users kind=binding target=run.users
    /// @resolution.name source=User target=User
    /// @resolution.call source=[make()] parameters=(&collections.array.arrayFromSlice.'a readonly Slice<collections.array.arrayFromSlice.T>) arguments=(rest(make()) as User) return=User[] kind=symbol target=collections.array.arrayFromSlice instance=collections.array.arrayFromSlice<User>
    /// @generic.instantiation id=collections.array.arrayFromSlice<User> template=collections.array.arrayFromSlice arguments=(User)
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

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
    let user: User = identity<User>(make() as User);
    let found: User = first<User>(make() as User | undefined);
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

declare function identity<T>(value: T): T;
/// @generic.template symbol=identity parameters=(T#1)
/// @type.symbol symbol=identity source="declare function identity<T>(value: T): T" type=<T#1>(T#1) => T#1
/// @type.symbol symbol=identity.T source=T type=T#1
/// @type.symbol symbol=identity.value source="value: T" type=T#1
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

declare function first<T>(value: T | undefined): T;
/// @generic.template symbol=first parameters=(T#2)
/// @type.symbol symbol=first source="declare function first<T>(value: T | undefined): T" type=<T#2>(T#2 | undefined) => T#2
/// @type.symbol symbol=first.T source=T type=T#2
/// @type.symbol symbol=first.value source="value: T | undefined" type=T#2 | undefined
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T

function run(): void {
/// @type.symbol symbol=run type=() => void

    let user = identity(make());
    /// @type.symbol symbol=run.user source=user type=User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=identity target=identity
    /// @resolution.call source=identity(make()) parameters=(User) arguments=(provided(make()) as User) return=User kind=symbol target=identity instance=identity<User>
    /// @generic.instantiation id=identity<User> template=identity arguments=(User)
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

    let found = first(make());
    /// @type.symbol symbol=run.found source=found type=User
    /// @resolution.pattern source=found kind=binding target=run.found
    /// @resolution.name source=first target=first
    /// @resolution.call source=first(make()) parameters=(User | undefined) arguments=(provided(make()) as User | undefined) return=User kind=symbol target=first instance=first<User>
    /// @generic.instantiation id=first<User> template=first arguments=(User)
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: union, target: User | undefined, cases: ({ source: Owned<User>, target: User, adjustments: [{ kind: manage, target: User }] }) }] origin=implicit

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

const get: () => User = (): User => make() as User;
const block: () => User = (): User => {
    return make() as User;
};

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

const get = () => make();
/// @type.symbol symbol=get source=get type=Function<(), User>
/// @resolution.pattern source=get kind=binding target=get
/// @type.symbol symbol=symbol3 source=() => make() type=Function<(), User>
/// @resolution.name source=make target=make
/// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
/// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

const block = () => {
/// @type.symbol symbol=block source=block type=Function<(), User>
/// @resolution.pattern source=block kind=binding target=block
/// @type.symbol symbol=symbol5 type=Function<(), User>

    return make();
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @coercion.node source=make() from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

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
    let explicit: User = identity<^User>(make()) as User;
    let users: (^User, ^User) = pair();
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

declare function identity<T>(value: T): T;
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity source="declare function identity<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

declare function pair(): (^User, ^User);
/// @type.symbol symbol=pair source="declare function pair(): (^User, ^User)" type=() => (Owned<User>, Owned<User>)
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let owned: ^User = make();
    /// @type.symbol symbol=run.owned source=owned type=Owned<User>
    /// @resolution.pattern source=owned kind=binding target=run.owned
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make

    let explicit = identity<^User>(make());
    /// @type.symbol symbol=run.explicit source=explicit type=User
    /// @resolution.pattern source=explicit kind=binding target=run.explicit
    /// @resolution.name source=identity target=identity
    /// @resolution.call source=identity<^User>(make()) parameters=(Owned<User>) arguments=(provided(make()) as Owned<User>) return=Owned<User> kind=symbol target=identity instance=identity<Owned<User>>
    /// @generic.instantiation id=identity<Owned<User>> template=identity arguments=(Owned<User>)
    /// @coercion.node source=identity<^User>(make()) from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make

    let users = pair();
    /// @type.symbol symbol=run.users source=users type=(Owned<User>, Owned<User>)
    /// @resolution.pattern source=users kind=binding target=run.users
    /// @resolution.name source=pair target=pair
    /// @resolution.call source=pair() parameters=() return=(Owned<User>, Owned<User>) kind=symbol target=pair

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
/// @type.symbol symbol=make source="declare function make(): ^Point" type=() => Owned<Point>
/// @resolution.name source=Point target=Point

function run(): void {
/// @type.symbol symbol=run type=() => void

    let point = make();
    /// @type.symbol symbol=run.point source=point type=Point
    /// @resolution.pattern source=point kind=binding target=run.point
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<Point> kind=symbol target=make

    let annotated: Point = make();
    /// @type.symbol symbol=run.annotated source=annotated type=Point
    /// @resolution.pattern source=annotated kind=binding target=run.annotated
    /// @resolution.name source=Point target=Point
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<Point> kind=symbol target=make

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
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let user: User = make().intoManaged();
    /// @type.symbol symbol=run.user source=user type=User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.member source=make().intoManaged receiver=Owned<User> type=(this: Owned<User>) => Managed<User> kind=symbol target_receiver=Owned<User> target=memory.owned.intoManaged#1
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @resolution.call source=make().intoManaged() parameters=() return=Managed<User> kind=symbol target=memory.owned.intoManaged#1 receiver=Owned<User> instance=Owned<memory.owned.T>.<extension#1>.intoManaged#1
    /// @generic.instantiation id=memory.owned.intoManaged#1<User> template=memory.owned.intoManaged#1 arguments=(User)

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
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

declare function take(user: User): void;
/// @type.symbol symbol=take source="declare function take(user: User): void" type=(User) => void
/// @type.symbol symbol=take.user source="user: User" type=User
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let owned: ^User = make();
    /// @type.symbol symbol=run.owned source=owned type=Owned<User>
    /// @resolution.pattern source=owned kind=binding target=run.owned
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make

    take(owned);
    /// @resolution.name source=take target=take
    /// @resolution.call source=take(owned) parameters=(User) arguments=(provided(owned) as User) return=void kind=symbol target=take
    /// @resolution.name source=owned target=run.owned
    /// @resolution.place source=owned placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=owned root=run.owned
    /// @coercion.node source=owned from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

    take(owned);
    /// @resolution.name source=take target=take
    /// @resolution.call source=take(owned) parameters=(User) arguments=(provided(owned) as User) return=void kind=symbol target=take
    /// @resolution.name source=owned target=run.owned
    /// @resolution.place source=owned placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=owned root=run.owned
    /// @coercion.node source=owned from=Owned<User> adjustments=[{ kind: manage, target: User }] origin=implicit

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
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

extension<T> of Owned<T> {
/// @generic.template symbol=<module>#2 parameters=(T#1)
/// @definition.extension symbol=<module>#2 form=local target=Owned<T#1>
/// @definition.method symbol=release slot=release type=(this: Owned<T#1>) => T#1
/// @type.symbol symbol=T#1 source=T type=T#1
/// @resolution.name source=Owned target=memory.owned.Owned
/// @resolution.name source=T target=T#1

    release(this: ^T): T {
    /// @type.symbol symbol=release type=(this: Owned<T#1>) => T#1
    /// @type.symbol symbol=release.this source="this: ^T" type=Owned<T#1>
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=T target=T#1

        return this;
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Owned<T#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @coercion.node source=this from=Owned<T#1> adjustments=[{ kind: carrier, target: T#1 }] origin=implicit

    }
}

extension<T> of ^T {
/// @generic.template symbol=<module>#3 parameters=(T#2)
/// @definition.extension symbol=<module>#3 form=local target=Owned<T#2>
/// @definition.method symbol=unwrap slot=unwrap type=(this: Owned<T#2>) => T#2
/// @type.symbol symbol=T#2 source=T type=T#2
/// @resolution.name source=T target=T#2

    unwrap(this: ^T): T {
    /// @type.symbol symbol=unwrap type=(this: Owned<T#2>) => T#2
    /// @type.symbol symbol=unwrap.this source="this: ^T" type=Owned<T#2>
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=T target=T#2

        return this;
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=Owned<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @coercion.node source=this from=Owned<T#2> adjustments=[{ kind: carrier, target: T#2 }] origin=implicit

    }
}

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

function run(): void {
/// @type.symbol symbol=run type=() => void

    let user: User = make().release();
    /// @type.symbol symbol=run.user source=user type=User
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.member source=make().release receiver=Owned<User> type=(this: Owned<User>) => User kind=symbol target_receiver=Owned<User> target=release
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @resolution.call source=make().release() parameters=() return=User kind=symbol target=release receiver=Owned<User> instance=Owned<T#1>.<extension#1>.release
    /// @generic.instantiation id=release<User> template=release arguments=(User)

    let other: User = make().unwrap();
    /// @type.symbol symbol=run.other source=other type=User
    /// @resolution.pattern source=other kind=binding target=run.other
    /// @resolution.name source=User target=User
    /// @resolution.name source=make target=make
    /// @resolution.member source=make().unwrap receiver=Owned<User> type=(this: Owned<User>) => User kind=symbol target_receiver=Owned<User> target=unwrap
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make
    /// @resolution.call source=make().unwrap() parameters=() return=User kind=symbol target=unwrap receiver=Owned<User> instance=Owned<T#2>.<extension#2>.unwrap
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
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function identity<T>(value: T): T;
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity source="declare function identity<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

function forward(input: ^User): ^User {
/// @type.symbol symbol=forward type=(Owned<User>) => Owned<User>
/// @type.symbol symbol=forward.input source="input: ^User" type=Owned<User>
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

    let moved = input;
    /// @type.symbol symbol=forward.moved source=moved type=Owned<User>
    /// @resolution.pattern source=moved kind=binding target=forward.moved
    /// @resolution.name source=input target=forward.input
    /// @resolution.access source=input root=forward.input

    return moved;
    /// @resolution.name source=moved target=forward.moved
    /// @resolution.place source=moved placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=moved root=forward.moved

}

function pass(input: ^User): ^User {
/// @type.symbol symbol=pass type=(Owned<User>) => Owned<User>
/// @type.symbol symbol=pass.input source="input: ^User" type=Owned<User>
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

    return identity(input);
    /// @resolution.name source=identity target=identity
    /// @resolution.call source=identity(input) parameters=(Owned<User>) arguments=(provided(input) as Owned<User>) return=Owned<User> kind=symbol target=identity instance=identity<Owned<User>>
    /// @generic.instantiation id=identity<Owned<User>> template=identity arguments=(Owned<User>)
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
    const managed: string[] = names() as string[];
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function make(): ^User;
/// @type.symbol symbol=make source="declare function make(): ^User" type=() => Owned<User>
/// @resolution.name source=User target=User

declare function names(): ^string[];
/// @type.symbol symbol=names source="declare function names(): ^string[]" type=() => Owned<string[]>

function run(): void {
/// @type.symbol symbol=run type=() => void

    const user: ^_ = make();
    /// @type.symbol symbol=run.user source=user type=Owned<User>
    /// @resolution.pattern source=user kind=binding target=run.user
    /// @resolution.name source=make target=make
    /// @resolution.call source=make() parameters=() return=Owned<User> kind=symbol target=make

    const owned: ^_ = names();
    /// @type.symbol symbol=run.owned source=owned type=Owned<string[]>
    /// @resolution.pattern source=owned kind=binding target=run.owned
    /// @resolution.name source=names target=names
    /// @resolution.call source=names() parameters=() return=Owned<string[]> kind=symbol target=names

    const managed = names();
    /// @type.symbol symbol=run.managed source=managed type=string[]
    /// @resolution.pattern source=managed kind=binding target=run.managed
    /// @resolution.name source=names target=names
    /// @resolution.call source=names() parameters=() return=Owned<string[]> kind=symbol target=names
    /// @coercion.node source=names() from=Owned<string[]> adjustments=[{ kind: manage, target: string[] }] origin=implicit

}
"#,
        r#"
"#,
    );
}
