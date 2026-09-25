use crate::tests::{DirRows, TestSession};

#[test]
fn test_call_selects_first_applicable_overload_after_borrowing() {
    let session = TestSession::single(
        r#"
class User {}

function select(value: &readonly User): "borrowed" {
    return "borrowed";
}

function select(value: User): "managed" {
    return "managed";
}

declare const user: User;
const selected = select(user);

selected satisfies "borrowed";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

function select<'a>(value: &'a readonly User): "borrowed" {
    return "borrowed";
}

function select(value: User): "managed" {
    return "managed";
}

declare const user: User;
const selected: "borrowed" = select<"managed">(user as &'managed readonly User);

selected satisfies "borrowed";

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

function select(value: &readonly User): "borrowed" {
/// @generic.template symbol=select#1 parameters=('a)
/// @type.symbol symbol=select#1 type=<select#1.'a>(&select#1.'a readonly User) => "borrowed"
/// @type.symbol symbol=select.value#1 source="value: &readonly User" type=&select#1.'a readonly User
/// @resolution.name source=User target=User

    return "borrowed";
}

function select(value: User): "managed" {
/// @type.symbol symbol=select#2 type=(User) => "managed"
/// @type.symbol symbol=select.value#2 source="value: User" type=User
/// @resolution.name source=User target=User

    return "managed";
}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const selected = select(user);
/// @type.symbol symbol=selected source=selected type="borrowed"
/// @resolution.pattern source=selected kind=binding target=selected
/// @resolution.name source=select target=[select#1, select#2]
/// @resolution.call source=select(user) parameters=(&'managed readonly User) arguments=(provided(user) as &'managed readonly User) return="borrowed" regions=("managed" & "local") kind=symbol target=select#1 instance="select#1<\"managed\" & \"local\">"
/// @generic.instantiation id="select#1<\"managed\" & \"local\">" template=select#1 arguments=("managed" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

selected satisfies "borrowed";
/// @resolution.name source=selected target=selected
/// @resolution.place source=selected placement="local" lifetime="static" access="immutable"
/// @resolution.access source=selected root=selected
"#,
        r#"

"#,
    );
}

#[test]
fn test_call_selects_first_applicable_overload_after_readonly_borrowing() {
    let session = TestSession::single(
        r#"
class User {}

function select(value: &readonly User): "readonly" {
    return "readonly";
}

function select(value: &User): "mutable" {
    return "mutable";
}

declare const user: User;
const selected = select(user);

selected satisfies "readonly";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

function select<'a>(value: &'a readonly User): "readonly" {
    return "readonly";
}

function select<'a>(value: &'a User): "mutable" {
    return "mutable";
}

declare const user: User;
const selected: "readonly" = select<"managed">(user as &'managed readonly User);

selected satisfies "readonly";

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

function select(value: &readonly User): "readonly" {
/// @generic.template symbol=select#1 parameters=('a)
/// @type.symbol symbol=select#1 type=<select#1.'a>(&select#1.'a readonly User) => "readonly"
/// @type.symbol symbol=select.value#1 source="value: &readonly User" type=&select#1.'a readonly User
/// @resolution.name source=User target=User

    return "readonly";
}

function select(value: &User): "mutable" {
/// @generic.template symbol=select#2 parameters=('a)
/// @type.symbol symbol=select#2 type=<select#2.'a>(&select#2.'a User) => "mutable"
/// @type.symbol symbol=select.value#2 source="value: &User" type=&select#2.'a User
/// @resolution.name source=User target=User

    return "mutable";
}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const selected = select(user);
/// @type.symbol symbol=selected source=selected type="readonly"
/// @resolution.pattern source=selected kind=binding target=selected
/// @resolution.name source=select target=[select#1, select#2]
/// @resolution.call source=select(user) parameters=(&'managed readonly User) arguments=(provided(user) as &'managed readonly User) return="readonly" regions=("managed" & "local") kind=symbol target=select#1 instance="select#1<\"managed\" & \"local\">"
/// @generic.instantiation id="select#1<\"managed\" & \"local\">" template=select#1 arguments=("managed" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

selected satisfies "readonly";
/// @resolution.name source=selected target=selected
/// @resolution.place source=selected placement="local" lifetime="static" access="immutable"
/// @resolution.access source=selected root=selected
"#,
        r#"
"#,
    );
}

#[test]
fn test_call_selects_first_applicable_overload_after_access_weakening() {
    let session = TestSession::single(
        r#"
class User {}

function select(value: &readonly User): "readonly" {
    return "readonly";
}

function select(value: &User): "mutable" {
    return "mutable";
}

declare const user: &User;
const selected = select(user);

selected satisfies "readonly";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function select<'a>(value: &'a readonly User): "readonly" {
    return "readonly";
}

function select<'a>(value: &'a User): "mutable" {
    return "mutable";
}

declare const user: &'static User;
const selected: "readonly" = select<"static">(user);

selected satisfies "readonly";

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

function select(value: &readonly User): "readonly" {
/// @generic.template symbol=select#1 parameters=('a)
/// @type.symbol symbol=select#1 type=<select#1.'a>(&select#1.'a readonly User) => "readonly"
/// @type.symbol symbol=select.value#1 source="value: &readonly User" type=&select#1.'a readonly User
/// @resolution.name source=User target=User

    return "readonly";
}

function select(value: &User): "mutable" {
/// @generic.template symbol=select#2 parameters=('a)
/// @type.symbol symbol=select#2 type=<select#2.'a>(&select#2.'a User) => "mutable"
/// @type.symbol symbol=select.value#2 source="value: &User" type=&select#2.'a User
/// @resolution.name source=User target=User

    return "mutable";
}

declare const user: &User;
/// @type.symbol symbol=user source=user type=&'static User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const selected = select(user);
/// @type.symbol symbol=selected source=selected type="readonly"
/// @resolution.pattern source=selected kind=binding target=selected
/// @resolution.name source=select target=[select#1, select#2]
/// @resolution.call source=select(user) parameters=(&'static readonly User) arguments=(provided(user) as &'static readonly User) return="readonly" regions=("static" & "local") kind=symbol target=select#1 instance="select#1<\"static\" & \"local\">"
/// @generic.instantiation id="select#1<\"static\" & \"local\">" template=select#1 arguments=("static" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user

selected satisfies "readonly";
/// @resolution.name source=selected target=selected
/// @resolution.place source=selected placement="local" lifetime="static" access="immutable"
/// @resolution.access source=selected root=selected
"#,
        r#"
"#,
    );
}

#[test]
fn test_coerce_a_local_managed_value_to_readonly_and_mutable_borrows() {
    let session = TestSession::single(
        r#"
class User {}

declare function inspect(value: &readonly User): void;
declare function modify(value: &User): void;
declare function replace(value: &User): void;

const user: User = new User();

inspect(user);
modify(user);
replace(user);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function inspect<'a>(value: &readonly User): void;
declare function modify<'a>(value: &User): void;
declare function replace<'a>(value: &User): void;

const user: User = new User();

inspect<"managed">(user as &'managed readonly User);
modify<"managed">(user as &'managed User);
replace<"managed">(user as &'managed User);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function inspect(value: &readonly User): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly User): void" type=<inspect.'a>(&inspect.'a readonly User) => void
/// @resolution.name source=User target=User

declare function modify(value: &User): void;
/// @generic.template symbol=modify parameters=('a)
/// @type.symbol symbol=modify source="declare function modify(value: &User): void" type=<modify.'a>(&modify.'a User) => void
/// @resolution.name source=User target=User

declare function replace(value: &User): void;
/// @generic.template symbol=replace parameters=('a)
/// @type.symbol symbol=replace source="declare function replace(value: &User): void" type=<replace.'a>(&replace.'a User) => void
/// @resolution.name source=User target=User

const user: User = new User();
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User
/// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
/// @resolution.name source=User target=User

inspect(user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(&'managed readonly User) arguments=(provided(user) as &'managed readonly User) return=void regions=("managed" & "local") kind=symbol target=inspect instance="inspect<\"managed\" & \"local\">"
/// @generic.instantiation id="inspect<\"managed\" & \"local\">" template=inspect arguments=("managed" & "local")
/// @generic.instance id="inspect<\"bound0\" & \"local\">" template=inspect arguments=("bound0" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

modify(user);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(user) parameters=(&'managed User) arguments=(provided(user) as &'managed User) return=void regions=("managed" & "local") kind=symbol target=modify instance="modify<\"managed\" & \"local\">"
/// @generic.instantiation id="modify<\"managed\" & \"local\">" template=modify arguments=("managed" & "local")
/// @generic.instance id="modify<\"bound0\" & \"local\">" template=modify arguments=("bound0" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=User adjustments=[{ kind: borrow, target: &'managed User }] origin=implicit

replace(user);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(user) parameters=(&'managed User) arguments=(provided(user) as &'managed User) return=void regions=("managed" & "local") kind=symbol target=replace instance="replace<\"managed\" & \"local\">"
/// @generic.instantiation id="replace<\"managed\" & \"local\">" template=replace arguments=("managed" & "local")
/// @generic.instance id="replace<\"bound0\" & \"local\">" template=replace arguments=("bound0" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=User adjustments=[{ kind: borrow, target: &'managed User }] origin=implicit
"#,
    );
}

#[test]
fn test_coerce_a_local_owned_value_to_readonly_and_mutable_borrows() {
    let session = TestSession::single(
        r#"
class User {}

declare function inspect(value: &readonly User): void;
declare function modify(value: &User): void;
declare function replace(value: &User): void;

declare let user: ^User;

inspect(user);
modify(user);
replace(user);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function inspect<'a>(value: &readonly User): void;
declare function modify<'a>(value: &User): void;
declare function replace<'a>(value: &User): void;

declare let user: ^User;

inspect<"static">(user as &'static readonly User);
modify<"static">(user as &'static User);
replace<"static">(user as &'static User);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function inspect(value: &readonly User): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly User): void" type=<inspect.'a>(&inspect.'a readonly User) => void
/// @resolution.name source=User target=User

declare function modify(value: &User): void;
/// @generic.template symbol=modify parameters=('a)
/// @type.symbol symbol=modify source="declare function modify(value: &User): void" type=<modify.'a>(&modify.'a User) => void
/// @resolution.name source=User target=User

declare function replace(value: &User): void;
/// @generic.template symbol=replace parameters=('a)
/// @type.symbol symbol=replace source="declare function replace(value: &User): void" type=<replace.'a>(&replace.'a User) => void
/// @resolution.name source=User target=User

declare let user: ^User;
/// @type.symbol symbol=user source=user type=^User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

inspect(user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(&'static readonly User) arguments=(provided(user) as &'static readonly User) return=void regions=("static" & "local") kind=symbol target=inspect instance="inspect<\"static\" & \"local\">"
/// @generic.instantiation id="inspect<\"static\" & \"local\">" template=inspect arguments=("static" & "local")
/// @generic.instance id="inspect<\"bound0\" & \"local\">" template=inspect arguments=("bound0" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=^User adjustments=[{ kind: borrow, target: &'static readonly User }] origin=implicit

modify(user);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(user) parameters=(&'static User) arguments=(provided(user) as &'static User) return=void regions=("static" & "local") kind=symbol target=modify instance="modify<\"static\" & \"local\">"
/// @generic.instantiation id="modify<\"static\" & \"local\">" template=modify arguments=("static" & "local")
/// @generic.instance id="modify<\"bound0\" & \"local\">" template=modify arguments=("bound0" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=^User adjustments=[{ kind: borrow, target: &'static User }] origin=implicit

replace(user);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(user) parameters=(&'static User) arguments=(provided(user) as &'static User) return=void regions=("static" & "local") kind=symbol target=replace instance="replace<\"static\" & \"local\">"
/// @generic.instantiation id="replace<\"static\" & \"local\">" template=replace arguments=("static" & "local")
/// @generic.instance id="replace<\"bound0\" & \"local\">" template=replace arguments=("bound0" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=^User adjustments=[{ kind: borrow, target: &'static User }] origin=implicit
"#,
    );
}

#[test]
fn test_coerce_readonly_managed_value_to_readonly_borrow() {
    let session = TestSession::single(
        r#"
class User {}

declare function inspect(value: &readonly User): void;
declare const user: readonly User;

inspect(user);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function inspect<'a>(value: &readonly User): void;
declare const user: readonly User;

inspect<"managed">(user as &'managed readonly User);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function inspect(value: &readonly User): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly User): void" type=<inspect.'a>(&inspect.'a readonly User) => void
/// @resolution.name source=User target=User

declare const user: readonly User;
/// @type.symbol symbol=user source=user type=readonly User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

inspect(user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(&'managed readonly User) arguments=(provided(user) as &'managed readonly User) return=void regions=("managed" & "local") kind=symbol target=inspect instance="inspect<\"managed\" & \"local\">"
/// @generic.instantiation id="inspect<\"managed\" & \"local\">" template=inspect arguments=("managed" & "local")
/// @generic.instance id="inspect<\"bound0\" & \"local\">" template=inspect arguments=("bound0" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=readonly User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit
"#,
    );
}

#[test]
fn test_reject_mutable_borrows_from_a_readonly_managed_value() {
    let session = TestSession::single(
        r#"
class User {}

declare function modify(value: &User): void;
declare function replace(value: &User): void;
declare const user: readonly User;

modify(user);
replace(user);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare function modify<'a>(value: &User): void;
declare function replace<'a>(value: &User): void;
declare const user: readonly User;

modify<"managed">(user);
replace<"managed">(user);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function modify(value: &User): void;
/// @generic.template symbol=modify parameters=('a)
/// @type.symbol symbol=modify source="declare function modify(value: &User): void" type=<modify.'a>(&modify.'a User) => void
/// @resolution.name source=User target=User

declare function replace(value: &User): void;
/// @generic.template symbol=replace parameters=('a)
/// @type.symbol symbol=replace source="declare function replace(value: &User): void" type=<replace.'a>(&replace.'a User) => void
/// @resolution.name source=User target=User

declare const user: readonly User;
/// @type.symbol symbol=user source=user type=readonly User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

modify(user);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(user) parameters=(&'managed User) arguments=(provided(user) as &'managed User) return=void regions=("managed" & "local") kind=symbol target=modify instance="modify<\"managed\" & \"local\">"
/// @generic.instantiation id="modify<\"managed\" & \"local\">" template=modify arguments=("managed" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user

replace(user);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(user) parameters=(&'managed User) arguments=(provided(user) as &'managed User) return=void regions=("managed" & "local") kind=symbol target=replace instance="replace<\"managed\" & \"local\">"
/// @generic.instantiation id="replace<\"managed\" & \"local\">" template=replace arguments=("managed" & "local")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'readonly User' is not assignable to parameter of type '&'managed User'"
/// @diagnostic.label line=8 column=8 span="user" line_source="modify(user);"
/// @diagnostic.related line=8 column=1 span="modify(user)" line_source="modify(user);" message="in this call"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'readonly User' is not assignable to parameter of type '&'managed User'"
/// @diagnostic.label line=9 column=9 span="user" line_source="replace(user);"
/// @diagnostic.related line=9 column=1 span="replace(user)" line_source="replace(user);" message="in this call"
"#,
    );
}
#[test]
fn test_coerce_managed_values_projected_from_fields_and_indices_to_readonly_borrows() {
    let session = TestSession::single(
        r#"
class User {}

struct Box<T> {
    value: T;
}

class State {
    user: User = new User();
    boxed: Box<User> = Box { value: new User() };
    users: User[] = [];
}

declare function inspect(value: &readonly User): void;
declare const state: State;

inspect(state.user);
inspect(state.boxed.value);
inspect(state.users[0]);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

struct Box<out T> {
    value: T;
}

class State {
    user: User = new User();
    boxed: Box<User> = Box<User> { value: new User() };
    users: User[] = [];
}

declare function inspect<'a>(value: &readonly User): void;
declare const state: State;

inspect<"managed">(state.user as &'managed readonly User);
inspect<"managed">(state.boxed.value as &'managed readonly User);
inspect<"managed">(state.users[0] as &'managed readonly User);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

struct Box<T> {
/// @generic.template symbol=Box parameters=(out T)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

}

class State {
/// @type.symbol symbol=State type=typeof State
/// @definition.class symbol=State
/// @definition.field symbol=State.boxed source="boxed: Box<User> = Box { value: new User() }" key=boxed type=Box<User>
/// @definition.field symbol=State.user source="user: User = new User()" key=user type=User
/// @definition.field symbol=State.users source="users: User[] = []" key=users type=User[]

    user: User = new User();
    /// @type.symbol symbol=State.user source="user: User = new User()" type=User
    /// @resolution.name source=User target=User
    /// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
    /// @resolution.name source=User target=User

    boxed: Box<User> = Box { value: new User() };
    /// @type.symbol symbol=State.boxed source="boxed: Box<User> = Box { value: new User() }" type=Box<User>
    /// @generic.instance id=Box<User> template=Box arguments=(User)
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=User target=User
    /// @resolution.name source=Box target=Box
    /// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
    /// @resolution.name source=User target=User

    users: User[] = [];
    /// @type.symbol symbol=State.users source="users: User[] = []" type=User[]
    /// @generic.instance id=Array<User> template=Array arguments=(User)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<User>> template=sliceAssumeInit arguments=(MaybeUninit<User>)
    /// @generic.instance id=sliceUninit<MaybeUninit<User>> template=sliceUninit arguments=(MaybeUninit<User>)
    /// @resolution.name source=User target=User
    /// @resolution.call source=[] parameters=(^Slice<User>) arguments=(rest() as User) return=User[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<User>
    /// @generic.instantiation id=arrayFromOwnedSlice<User> template=arrayFromOwnedSlice arguments=(User)
    /// @generic.instance id=arrayFromOwnedSlice<User> template=arrayFromOwnedSlice arguments=(User)

}

declare function inspect(value: &readonly User): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly User): void" type=<inspect.'a>(&inspect.'a readonly User) => void
/// @resolution.name source=User target=User

declare const state: State;
/// @type.symbol symbol=state source=state type=State
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State

inspect(state.user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(state.user) parameters=(&'managed readonly User) arguments=(provided(state.user) as &'managed readonly User) return=void regions=("managed" & "local") kind=symbol target=inspect instance="inspect<\"managed\" & \"local\">"
/// @generic.instantiation id="inspect<\"managed\" & \"local\">" template=inspect arguments=("managed" & "local")
/// @generic.instance id="inspect<\"bound0\" & \"local\">" template=inspect arguments=("bound0" & "local")
/// @resolution.name source=state target=state
/// @resolution.member source=state.user receiver=State type=User kind=field target_receiver=State key=user target=State.user target_type=User
/// @resolution.place source=state placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state root=state
/// @resolution.place source=state.user placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=state.user root=state keys=[user]
/// @coercion.node source=state.user from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

inspect(state.boxed.value);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(state.boxed.value) parameters=(&'managed readonly User) arguments=(provided(state.boxed.value) as &'managed readonly User) return=void regions=("managed" & "local") kind=symbol target=inspect instance="inspect<\"managed\" & \"local\">"
/// @resolution.name source=state target=state
/// @resolution.member source=state.boxed receiver=State type=Box<User> kind=field target_receiver=State key=boxed target=State.boxed target_type=Box<User>
/// @resolution.member source=state.boxed.value receiver=Box<User> type=User kind=field target_receiver=Box<User> key=value target=Box.value target_type=User
/// @resolution.place source=state placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state root=state
/// @resolution.place source=state.boxed placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=state.boxed root=state keys=[boxed]
/// @resolution.place source=state.boxed.value placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=state.boxed.value root=state keys=[boxed, value]
/// @coercion.node source=state.boxed.value from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

inspect(state.users[0]);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(state.users[0]) parameters=(&'managed readonly User) arguments=(provided(state.users[0]) as &'managed readonly User) return=void regions=("managed" & "local") kind=symbol target=inspect instance="inspect<\"managed\" & \"local\">"
/// @resolution.name source=state target=state
/// @resolution.member source=state.users receiver=State type=User[] kind=field target_receiver=State key=users target=State.users target_type=User[]
/// @resolution.place source=state placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state root=state
/// @resolution.place source=state.users placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=state.users root=state keys=[users]
/// @resolution.subscript source=state.users[0] type=User kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=User, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<User, \"managed\" & \"local\">" template=index#2 arguments=(User, "managed" & "local")
/// @generic.instance id="index#2<User, \"bound0\" & \"local\">" template=index#2 arguments=(User, "bound0" & "local")
/// @generic.instance id=Clone.clone<User> template=Clone.clone arguments=()
/// @coercion.node source=state.users[0] from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

/// @generic.template symbol=Clone.clone parameters=('a)
/// @type.symbol symbol=Clone.clone type=<Clone.clone.'a>(this: &Clone.clone.'a immutable User) => ^User
"#,
    );
}

#[test]
fn test_coerce_each_conditional_branch_from_managed_to_readonly_borrow() {
    let session = TestSession::single(
        r#"
class User {}

declare const condition: boolean;
declare const first: User;
declare const second: User;

function select(): void {
    const selected: &readonly User = condition ? first : second;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const condition: boolean;
declare const first: User;
declare const second: User;

function select(): void {
    const selected: &'managed readonly User = condition
        ? (first as &'managed readonly User)
        : (second as &'managed readonly User);
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const condition: boolean;
/// @type.symbol symbol=condition source=condition type=boolean
/// @resolution.pattern source=condition kind=binding target=condition

declare const first: User;
/// @type.symbol symbol=first source=first type=User
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=User target=User

declare const second: User;
/// @type.symbol symbol=second source=second type=User
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.name source=User target=User

function select(): void {
/// @type.symbol symbol=select type=() => void

    const selected: &readonly User = condition ? first : second;
    /// @type.symbol symbol=select.selected source=selected type=&'managed readonly User
    /// @resolution.pattern source=selected kind=binding target=select.selected
    /// @resolution.name source=User target=User
    /// @resolution.name source=condition target=condition
    /// @resolution.place source=condition placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=condition root=condition
    /// @resolution.name source=first target=first
    /// @resolution.place source=first placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=first root=first
    /// @coercion.node source=first from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit
    /// @resolution.name source=second target=second
    /// @resolution.place source=second placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=second root=second
    /// @coercion.node source=second from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

}
"#,
    );
}

#[test]
fn test_coerce_each_match_arm_from_managed_to_readonly_borrow() {
    let session = TestSession::single(
        r#"
class User {}

declare const choice: "first" | "second";
declare const first: User;
declare const second: User;

function select(): void {
    const selected: &readonly User = match (choice) {
        "first" => first
        _ => second
    };
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const choice: "first" | "second";
declare const first: User;
declare const second: User;

function select(): void {
    const selected: &'managed readonly User = match (choice) {
        "first" => first as &'managed readonly User
        _ => second as &'managed readonly User
    };
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const choice: "first" | "second";
/// @type.symbol symbol=choice source=choice type="first" | "second"
/// @resolution.pattern source=choice kind=binding target=choice

declare const first: User;
/// @type.symbol symbol=first source=first type=User
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=User target=User

declare const second: User;
/// @type.symbol symbol=second source=second type=User
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.name source=User target=User

function select(): void {
/// @type.symbol symbol=select type=() => void

    const selected: &readonly User = match (choice) {
    /// @type.symbol symbol=select.selected source=selected type=&'managed readonly User
    /// @resolution.pattern source=selected kind=binding target=select.selected
    /// @resolution.name source=User target=User
    /// @resolution.coverage exhaustive=true disjoint=false
    /// @resolution.name source=choice target=choice
    /// @resolution.place source=choice placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=choice root=choice

        "first" => first
        /// @resolution.pattern source="\"first\"" kind=literal value="first"
        /// @resolution.name source=first target=first
        /// @resolution.place source=first placement="local" lifetime="static" access="immutable"
        /// @resolution.access source=first root=first
        /// @coercion.node source=first from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

        _ => second
        /// @resolution.pattern source=_ kind=wildcard
        /// @resolution.name source=second target=second
        /// @resolution.place source=second placement="local" lifetime="static" access="immutable"
        /// @resolution.access source=second root=second
        /// @coercion.node source=second from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

    };
}
"#,
    );
}

#[test]
fn test_coerce_each_tuple_element_from_managed_to_readonly_borrow() {
    let session = TestSession::single(
        r#"
class User {}

declare const first: User;
declare const second: User;

function select(): void {
    const selected: (&readonly User, &readonly User) = (first, second);
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const first: User;
declare const second: User;

function select(): void {
    const selected: (&'managed readonly User, &'managed readonly User) = (
        first as &'managed readonly User,
        second as &'managed readonly User,
    );
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const first: User;
/// @type.symbol symbol=first source=first type=User
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=User target=User

declare const second: User;
/// @type.symbol symbol=second source=second type=User
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.name source=User target=User

function select(): void {
/// @type.symbol symbol=select type=() => void

    const selected: (&readonly User, &readonly User) = (first, second);
    /// @type.symbol symbol=select.selected source=selected type=(&'managed readonly User, &'managed readonly User)
    /// @resolution.pattern source=selected kind=binding target=select.selected
    /// @resolution.name source=User target=User
    /// @resolution.name source=User target=User
    /// @resolution.name source=first target=first
    /// @resolution.place source=first placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=first root=first
    /// @coercion.node source=first from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit
    /// @resolution.name source=second target=second
    /// @resolution.place source=second placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=second root=second
    /// @coercion.node source=second from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

}
"#,
    );
}

#[test]
fn test_coerce_managed_string_literal_to_readonly_borrow() {
    let session = TestSession::single(
        r#"
declare function inspect(value: &readonly string): void;

inspect("message");
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare function inspect<'a>(value: &readonly string): void;

inspect<"managed">("message" as &'managed readonly string);

=== dir ===
declare function inspect(value: &readonly string): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly string): void" type=<inspect.'a>(&inspect.'a readonly string) => void

inspect("message");
/// @resolution.name source=inspect target=inspect
/// @resolution.call source="inspect(\"message\")" parameters=(&'managed readonly string) arguments=(provided("message") as &'managed readonly string) return=void regions=("managed" & "local") kind=symbol target=inspect instance="inspect<\"managed\" & \"local\">"
/// @generic.instantiation id="inspect<\"managed\" & \"local\">" template=inspect arguments=("managed" & "local")
/// @generic.instance id="inspect<\"bound0\" & \"local\">" template=inspect arguments=("bound0" & "local")
/// @coercion.node source="\"message\"" from="message" adjustments=[{ kind: materialize, target: string }, { kind: borrow, target: &'managed readonly string }] origin=implicit
"#,
    );
}

#[test]
fn test_preserve_shared_place_when_borrowing_shared_managed_value() {
    let session = TestSession::single(
        r#"
shared class User {}

declare const user: User;
declare function inspect(value: &readonly User): void;
declare function modify(value: &User): void;

inspect(user);
modify(user);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
shared class User {}

declare const user: User;
declare function inspect<'a>(value: &readonly User): void;
declare function modify<'a>(value: &User): void;

inspect<"managed">(user as &'managed readonly User);
modify<"managed">(user as &'managed User);

=== dir ===
shared class User {}
/// @type.symbol symbol=User source="shared class User {}" type=typeof User
/// @definition.class symbol=User source="shared class User {}"

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

declare function inspect(value: &readonly User): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly User): void" type=<inspect.'a>(&inspect.'a readonly User) => void
/// @resolution.name source=User target=User

declare function modify(value: &User): void;
/// @generic.template symbol=modify parameters=('a)
/// @type.symbol symbol=modify source="declare function modify(value: &User): void" type=<modify.'a>(&modify.'a User) => void
/// @resolution.name source=User target=User

inspect(user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(&'managed readonly User) arguments=(provided(user) as &'managed readonly User) return=void regions=("managed" & "shared") kind=symbol target=inspect instance="inspect<\"managed\" & \"shared\">"
/// @generic.instantiation id="inspect<\"managed\" & \"shared\">" template=inspect arguments=("managed" & "shared")
/// @generic.instance id="inspect<\"bound0\" & \"shared\">" template=inspect arguments=("bound0" & "shared")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=implicit

modify(user);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(user) parameters=(&'managed User) arguments=(provided(user) as &'managed User) return=void regions=("managed" & "shared") kind=symbol target=modify instance="modify<\"managed\" & \"shared\">"
/// @generic.instantiation id="modify<\"managed\" & \"shared\">" template=modify arguments=("managed" & "shared")
/// @generic.instance id="modify<\"bound0\" & \"shared\">" template=modify arguments=("bound0" & "shared")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=User adjustments=[{ kind: borrow, target: &'managed User }] origin=implicit
"#,
    );
}

/// Type a mutable borrow of a shared managed value, which verify rejects.
#[test]
fn test_type_a_mutable_borrow_of_a_shared_managed_value() {
    let session = TestSession::single(
        r#"
shared class User {}

declare const user: User;
declare function replace(value: &User): void;

replace(user);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
shared class User {}

declare const user: User;
declare function replace<'a>(value: &User): void;

replace<"managed">(user as &'managed User);

=== dir ===
shared class User {}
/// @type.symbol symbol=User source="shared class User {}" type=typeof User
/// @definition.class symbol=User source="shared class User {}"

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

declare function replace(value: &User): void;
/// @generic.template symbol=replace parameters=('a)
/// @type.symbol symbol=replace source="declare function replace(value: &User): void" type=<replace.'a>(&replace.'a User) => void
/// @resolution.name source=User target=User

replace(user);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(user) parameters=(&'managed User) arguments=(provided(user) as &'managed User) return=void regions=("managed" & "shared") kind=symbol target=replace instance="replace<\"managed\" & \"shared\">"
/// @generic.instantiation id="replace<\"managed\" & \"shared\">" template=replace arguments=("managed" & "shared")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
"#,
        r#"

"#,
    );
}

/// Coerce a shared owned value to readonly and mutable borrows.
#[test]
fn test_coerce_a_shared_owned_value_to_readonly_and_mutable_borrows() {
    let session = TestSession::single(
        r#"
shared class User {}

declare shared let user: ^User;
declare function inspect(value: &readonly User): void;
declare function modify(value: &User): void;
declare function replace(value: &User): void;

inspect(user);
modify(user);
replace(user);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
shared class User {}

declare shared let user: ^User;
declare function inspect<'a>(value: &readonly User): void;
declare function modify<'a>(value: &User): void;
declare function replace<'a>(value: &User): void;

inspect<"static">(user as &'static readonly User);
modify<"static">(user as &'static User);
replace<"static">(user as &'static User);

=== dir ===
shared class User {}
/// @type.symbol symbol=User source="shared class User {}" type=typeof User
/// @definition.class symbol=User source="shared class User {}"

declare shared let user: ^User;
/// @type.symbol symbol=user source=user type=^User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

declare function inspect(value: &readonly User): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly User): void" type=<inspect.'a>(&inspect.'a readonly User) => void
/// @resolution.name source=User target=User

declare function modify(value: &User): void;
/// @generic.template symbol=modify parameters=('a)
/// @type.symbol symbol=modify source="declare function modify(value: &User): void" type=<modify.'a>(&modify.'a User) => void
/// @resolution.name source=User target=User

declare function replace(value: &User): void;
/// @generic.template symbol=replace parameters=('a)
/// @type.symbol symbol=replace source="declare function replace(value: &User): void" type=<replace.'a>(&replace.'a User) => void
/// @resolution.name source=User target=User

inspect(user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(&'static readonly User) arguments=(provided(user) as &'static readonly User) return=void regions=("static" & "shared") kind=symbol target=inspect instance="inspect<\"static\" & \"shared\">"
/// @generic.instantiation id="inspect<\"static\" & \"shared\">" template=inspect arguments=("static" & "shared")
/// @generic.instance id="inspect<\"bound0\" & \"shared\">" template=inspect arguments=("bound0" & "shared")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=^User adjustments=[{ kind: borrow, target: &'static readonly User }] origin=implicit

modify(user);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(user) parameters=(&'static User) arguments=(provided(user) as &'static User) return=void regions=("static" & "shared") kind=symbol target=modify instance="modify<\"static\" & \"shared\">"
/// @generic.instantiation id="modify<\"static\" & \"shared\">" template=modify arguments=("static" & "shared")
/// @generic.instance id="modify<\"bound0\" & \"shared\">" template=modify arguments=("bound0" & "shared")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=^User adjustments=[{ kind: borrow, target: &'static User }] origin=implicit

replace(user);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(user) parameters=(&'static User) arguments=(provided(user) as &'static User) return=void regions=("static" & "shared") kind=symbol target=replace instance="replace<\"static\" & \"shared\">"
/// @generic.instantiation id="replace<\"static\" & \"shared\">" template=replace arguments=("static" & "shared")
/// @generic.instance id="replace<\"bound0\" & \"shared\">" template=replace arguments=("bound0" & "shared")
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=^User adjustments=[{ kind: borrow, target: &'static User }] origin=implicit
"#,
    );
}

#[test]
fn test_coerce_direct_struct_and_managed_array_to_borrowed_forms() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

declare function inspectPoint(value: &readonly Point): void;
declare function inspectValues(value: &readonly int32[]): void;
declare const point: Point;
declare const values: int32[];

inspectPoint(point);
inspectValues(values);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

declare function inspectPoint<'a>(value: &readonly Point): void;
declare function inspectValues<'a>(value: &readonly int32[]): void;
declare const point: Point;
declare const values: int32[];

inspectPoint<"static">(point as &'static readonly Point);
inspectValues<"managed">(values as &'managed readonly int32[]);

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

declare function inspectPoint(value: &readonly Point): void;
/// @generic.template symbol=inspectPoint parameters=('a)
/// @type.symbol symbol=inspectPoint source="declare function inspectPoint(value: &readonly Point): void" type=<inspectPoint.'a>(&inspectPoint.'a readonly Point) => void
/// @resolution.name source=Point target=Point

declare function inspectValues(value: &readonly int32[]): void;
/// @generic.template symbol=inspectValues parameters=('a)
/// @type.symbol symbol=inspectValues source="declare function inspectValues(value: &readonly int32[]): void" type=<inspectValues.'a>(&inspectValues.'a readonly int32[]) => void
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

declare const values: int32[];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values

inspectPoint(point);
/// @resolution.name source=inspectPoint target=inspectPoint
/// @resolution.call source=inspectPoint(point) parameters=(&'static readonly Point) arguments=(provided(point) as &'static readonly Point) return=void regions=("static" & "local") kind=symbol target=inspectPoint instance="inspectPoint<\"static\" & \"local\">"
/// @generic.instantiation id="inspectPoint<\"static\" & \"local\">" template=inspectPoint arguments=("static" & "local")
/// @generic.instance id="inspectPoint<\"bound0\" & \"local\">" template=inspectPoint arguments=("bound0" & "local")
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
/// @coercion.node source=point from=Point adjustments=[{ kind: borrow, target: &'static readonly Point }] origin=implicit

inspectValues(values);
/// @resolution.name source=inspectValues target=inspectValues
/// @resolution.call source=inspectValues(values) parameters=(&'managed readonly int32[]) arguments=(provided(values) as &'managed readonly int32[]) return=void regions=("managed" & "local") kind=symbol target=inspectValues instance="inspectValues<\"managed\" & \"local\">"
/// @generic.instantiation id="inspectValues<\"managed\" & \"local\">" template=inspectValues arguments=("managed" & "local")
/// @generic.instance id="inspectValues<\"bound0\" & \"local\">" template=inspectValues arguments=("bound0" & "local")
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @coercion.node source=values from=int32[] adjustments=[{ kind: borrow, target: &'managed readonly int32[] }] origin=implicit
"#,
    );
}

#[test]
fn test_restrict_implicit_borrows_of_immutable_direct_storage() {
    let session = TestSession::single(
        r#"
declare function inspect(value: &readonly int32): void;
declare function modify(value: &int32): void;
declare function replace(value: &int32): void;

declare const value: int32;

inspect(value);
modify(value);
replace(value);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare function inspect<'a>(value: &readonly int32): void;
declare function modify<'a>(value: &int32): void;
declare function replace<'a>(value: &int32): void;

declare const value: int32;

inspect<"static">(value as &'static readonly int32);
modify<"static">(value);
replace<"static">(value);

=== dir ===
declare function inspect(value: &readonly int32): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly int32): void" type=<inspect.'a>(&inspect.'a readonly int32) => void

declare function modify(value: &int32): void;
/// @generic.template symbol=modify parameters=('a)
/// @type.symbol symbol=modify source="declare function modify(value: &int32): void" type=<modify.'a>(&modify.'a int32) => void

declare function replace(value: &int32): void;
/// @generic.template symbol=replace parameters=('a)
/// @type.symbol symbol=replace source="declare function replace(value: &int32): void" type=<replace.'a>(&replace.'a int32) => void

declare const value: int32;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value

inspect(value);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(value) parameters=(&'static readonly int32) arguments=(provided(value) as &'static readonly int32) return=void regions=("static" & "local") kind=symbol target=inspect instance="inspect<\"static\" & \"local\">"
/// @generic.instantiation id="inspect<\"static\" & \"local\">" template=inspect arguments=("static" & "local")
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @coercion.node source=value from=int32 adjustments=[{ kind: borrow, target: &'static readonly int32 }] origin=implicit

modify(value);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(value) parameters=(&'static int32) arguments=(provided(value) as &'static int32) return=void regions=("static" & "local") kind=symbol target=modify instance="modify<\"static\" & \"local\">"
/// @generic.instantiation id="modify<\"static\" & \"local\">" template=modify arguments=("static" & "local")
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

replace(value);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(value) parameters=(&'static int32) arguments=(provided(value) as &'static int32) return=void regions=("static" & "local") kind=symbol target=replace instance="replace<\"static\" & \"local\">"
/// @generic.instantiation id="replace<\"static\" & \"local\">" template=replace arguments=("static" & "local")
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'int32' is not assignable to parameter of type '&'static int32'"
/// @diagnostic.label line=9 column=8 span="value" line_source="modify(value);"
/// @diagnostic.related line=9 column=1 span="modify(value)" line_source="modify(value);" message="in this call"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'int32' is not assignable to parameter of type '&'static int32'"
/// @diagnostic.label line=10 column=9 span="value" line_source="replace(value);"
/// @diagnostic.related line=10 column=1 span="replace(value)" line_source="replace(value);" message="in this call"
"#,
    );
}

/// Type an exclusive receiver borrow of a shared managed value, which verify rejects.
#[test]
fn test_type_an_exclusive_receiver_borrow_of_a_shared_managed_value() {
    let session = TestSession::single(
        r#"
shared class Message {}

shared class Messages {
    push(&exclusive this, message: Message): void {}
}

declare const messages: Messages;
declare const message: Message;

messages.push(message);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
shared class Message {}

shared class Messages {
    push(&exclusive this, message: Message): void {}
}

declare const messages: Messages;
declare const message: Message;

messages.push<"managed">(message);

=== dir ===
shared class Message {}
/// @type.symbol symbol=Message source="shared class Message {}" type=typeof Message
/// @definition.class symbol=Message source="shared class Message {}"

shared class Messages {
/// @type.symbol symbol=Messages type=typeof Messages
/// @definition.class symbol=Messages
/// @definition.method symbol=Messages.push source="push(&exclusive this, message: Message): void {}" slot=push type=<Messages.push.'a>(this: &Messages.push.'a exclusive Messages, Message) => void

    push(&exclusive this, message: Message): void {}
    /// @generic.template symbol=Messages.push parameters=('a)
    /// @type.symbol symbol=Messages.push source="push(&exclusive this, message: Message): void {}" type=<Messages.push.'a>(this: &Messages.push.'a exclusive Messages, Message) => void
    /// @type.symbol symbol=Messages.push.this source="&exclusive this" type=&Messages.push.'a exclusive Messages
    /// @type.symbol symbol=Messages.push.message source="message: Message" type=Message
    /// @resolution.name source=Message target=Message

}

declare const messages: Messages;
/// @type.symbol symbol=messages source=messages type=Messages
/// @resolution.pattern source=messages kind=binding target=messages
/// @resolution.name source=Messages target=Messages

declare const message: Message;
/// @type.symbol symbol=message source=message type=Message
/// @resolution.pattern source=message kind=binding target=message
/// @resolution.name source=Message target=Message

messages.push(message);
/// @resolution.name source=messages target=messages
/// @resolution.member source=messages.push receiver=Messages type=<Messages.push.'a>(this: &Messages.push.'a exclusive Messages, Message) => void kind=symbol target_receiver=Messages target=Messages.push
/// @resolution.call source=messages.push(message) parameters=(Message) arguments=(provided(message) as Message) return=void regions=("managed" & "shared") kind=symbol target=Messages.push receiver=Messages adjustments=(borrow(&'managed exclusive Messages)) instance="Messages.push<\"managed\" & \"shared\">"
/// @resolution.place source=messages placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=messages root=messages
/// @generic.instantiation id="Messages.push<\"managed\" & \"shared\">" template=Messages.push arguments=("managed" & "shared")
/// @resolution.name source=message target=message
/// @resolution.place source=message placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=message root=message
"#,
        r#"

"#,
    );
}

#[test]
fn test_keep_managed_value_without_borrow_context() {
    let session = TestSession::single(
        r#"
class User {}

declare const user: User;
const same = user;

same satisfies User;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const user: User;
const same: User = user;

same satisfies User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const same = user;
/// @type.symbol symbol=same source=same type=User
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=user target=user
/// @resolution.access source=user root=user

same satisfies User;
/// @resolution.name source=same target=same
/// @resolution.place source=same placement="local" lifetime="static" access="immutable"
/// @resolution.access source=same root=same
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_reject_managed_value_as_owned() {
    let session = TestSession::single(
        r#"
class User {}

let user: User = new User();
let owned: ^User = user;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}

let user: User = new User();
let owned: ^User = user;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

let user: User = new User();
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User
/// @type.node source="new User()" type=User
/// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
/// @type.node source=User type=typeof User
/// @resolution.name source=User target=User

let owned: ^User = user;
/// @type.symbol symbol=owned source=owned type=^User
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=User target=User
/// @type.node source=user type=User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'User' is not assignable to type '^User'"
/// @diagnostic.label line=5 column=20 span="user" line_source="let owned: ^User = user;"
/// @diagnostic.related line=5 column=12 span="^" line_source="let owned: ^User = user;" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_reject_noncopyable_borrow_as_owned() {
    let session = TestSession::single(
        r#"
struct Label { values: ^Array<uint8>; }

declare let label: ^Label;
let borrow = &label;
let owned: ^Label = borrow;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Label {
    values: ^uint8[];
}

declare let label: Label;
let borrow: &'static Label = &label;
let owned: Label = borrow;

=== dir ===
struct Label { values: ^Array<uint8>; }
/// @type.symbol symbol=Label source="struct Label { values: ^Array<uint8>; }" type=Label
/// @definition.struct symbol=Label source="struct Label { values: ^Array<uint8>; }"
/// @definition.field symbol=Label.values source="values: ^Array<uint8>" key=values type=^Array<uint8>
/// @type.symbol symbol=Label.values source="values: ^Array<uint8>" type=^Array<uint8>
/// @resolution.name source=Array target=Array

declare let label: ^Label;
/// @type.symbol symbol=label source=label type=Label
/// @resolution.pattern source=label kind=binding target=label
/// @resolution.name source=Label target=Label

let borrow = &label;
/// @type.symbol symbol=borrow source=borrow type=&'static Label
/// @resolution.pattern source=borrow kind=binding target=borrow
/// @type.node source=&label type=&'static Label
/// @type.node source=label type=Label
/// @resolution.name source=label target=label
/// @resolution.place source=label placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=label root=label

let owned: ^Label = borrow;
/// @type.symbol symbol=owned source=owned type=Label
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=Label target=Label
/// @type.node source=borrow type=&'static Label
/// @resolution.name source=borrow target=borrow
/// @resolution.place source=borrow placement="local" lifetime="static" access="mutable"
/// @resolution.access source=borrow root=borrow
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '&'static Label' is not assignable to type 'Label'"
/// @diagnostic.label line=6 column=21 span="borrow" line_source="let owned: ^Label = borrow;"
/// @diagnostic.related line=6 column=12 span="^" line_source="let owned: ^Label = borrow;" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_read_copyable_value_out_of_borrow() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point: ^Point = Point { x: 1 };
let borrow = &point;
let copied: Point = borrow;
let owned: ^Point = borrow;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

let point: Point = Point { x: 1 };
let borrow: &'static Point = &point;
let copied: Point = borrow as Point;
let owned: Point = borrow as Point;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

let point: ^Point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int32 }] origin=implicit

let borrow = &point;
/// @type.symbol symbol=borrow source=borrow type=&'static Point
/// @resolution.pattern source=borrow kind=binding target=borrow
/// @type.node source=&point type=&'static Point
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point

let copied: Point = borrow;
/// @type.symbol symbol=copied source=copied type=Point
/// @resolution.pattern source=copied kind=binding target=copied
/// @resolution.name source=Point target=Point
/// @type.node source=borrow type=&'static Point
/// @resolution.name source=borrow target=borrow
/// @resolution.place source=borrow placement="local" lifetime="static" access="mutable"
/// @resolution.access source=borrow root=borrow
/// @coercion.node source=borrow from=&'static Point adjustments=[{ kind: read, target: Point }] origin=implicit

let owned: ^Point = borrow;
/// @type.symbol symbol=owned source=owned type=Point
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=Point target=Point
/// @type.node source=borrow type=&'static Point
/// @resolution.name source=borrow target=borrow
/// @resolution.place source=borrow placement="local" lifetime="static" access="mutable"
/// @resolution.access source=borrow root=borrow
/// @coercion.node source=borrow from=&'static Point adjustments=[{ kind: read, target: Point }] origin=implicit
"#,
        r#"

"#,
    );
}

#[test]
fn test_materialize_construction_at_owned_target() {
    let session = TestSession::single(
        r#"
class User {}

let owned: ^User = new User();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}

let owned: ^User = new User();

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

let owned: ^User = new User();
/// @type.symbol symbol=owned source=owned type=^User
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=User target=User
/// @type.node source="new User()" type=^User
/// @resolution.construct source="new User()" parameters=() return=^User kind=class target=User constructor=default
/// @type.node source=User type=typeof User
/// @resolution.name source=User target=User
"#,
        r#"

"#,
    );
}

#[test]
fn test_copy_value_into_owned_target() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point = Point { x: 1 };
let owned: ^Point = point;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

let point: Point = Point { x: 1 };
let owned: Point = point;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

let point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

let owned: ^Point = point;
/// @type.symbol symbol=owned source=owned type=Point
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=Point target=Point
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
"#,
        r#"

"#,
    );
}

#[test]
fn test_copy_bounded_value_into_owned_result() {
    let session = TestSession::single(
        r#"
import { Copy } from "tspp:memory";

function duplicate<T: Copy>(value: T): ^T {
    value
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Copy } from "tspp:memory";

function duplicate<T: Copy>(value: T): ^T {
    value as ^T
}

=== dir ===
import { Copy } from "tspp:memory";

function duplicate<T: Copy>(value: T): ^T {
/// @generic.template symbol=duplicate parameters=(T: Copy)
/// @type.symbol symbol=duplicate type=<T: Copy>(T) => ^T
/// @type.symbol symbol=duplicate.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy
/// @type.symbol symbol=duplicate.value source="value: T" type=T
/// @resolution.name source=T target=duplicate.T
/// @resolution.name source=T target=duplicate.T

    value
    /// @resolution.name source=value target=duplicate.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=duplicate.value

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_inferred_argument_violating_copy_bound() {
    let session = TestSession::single(
        r#"
import { Copy } from "tspp:memory";

declare function duplicate<T: Copy>(value: T): ^T;
declare const values: ^Array<int32>;

duplicate(32);
duplicate(values);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Copy } from "tspp:memory";

declare function duplicate<T: Copy>(value: T): ^T;
declare const values: ^int32[];

duplicate<int64>(32);
duplicate<^int32[]>(values);

=== dir ===
import { Copy } from "tspp:memory";

declare function duplicate<T: Copy>(value: T): ^T;
/// @generic.template symbol=duplicate parameters=(T: Copy)
/// @type.symbol symbol=duplicate source="declare function duplicate<T: Copy>(value: T): ^T" type=<T: Copy>(T) => ^T
/// @type.symbol symbol=duplicate.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=T target=duplicate.T
/// @resolution.name source=T target=duplicate.T

declare const values: ^Array<int32>;
/// @type.symbol symbol=values source=values type=^Array<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Array target=Array

duplicate(32);
/// @resolution.name source=duplicate target=duplicate
/// @resolution.call source=duplicate(32) parameters=(int64) arguments=(provided(32) as int64) return=^int64 kind=symbol target=duplicate instance=duplicate<int64>
/// @generic.instantiation id=duplicate<int64> template=duplicate arguments=(int64)

duplicate(values);
/// @resolution.name source=duplicate target=duplicate
/// @resolution.call source=duplicate(values) parameters=(^Array<int32>) arguments=(provided(values) as ^Array<int32>) return=^Array<int32> kind=symbol target=duplicate instance=duplicate<^Array<int32>>
/// @generic.instantiation id=duplicate<^Array<int32>> template=duplicate arguments=(^Array<int32>)
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type '^int32[]' does not satisfy 'Copy'"
/// @diagnostic.label line=8 column=1 span="duplicate(values)" line_source="duplicate(values);"
/// @diagnostic.related line=4 column=28 span="T" line_source="declare function duplicate<T: Copy>(value: T): ^T;" message="required by this bound on 'T'"
"#,
    );
}

#[test]
fn test_read_borrowed_copy_element_in_filter_predicate() {
    let session = TestSession::single(
        r#"
declare const values: (int32 | undefined)[];
const mapped = values.map((value) => value);
const filtered = mapped.filter((value) => value != undefined);

filtered;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: (int32 | undefined)[];
const mapped: ^(int32 | undefined)[] = values.map<int32 | undefined, int32 | undefined, "managed">(
    (value: int32 | undefined): int32 | undefined => value,
);
const filtered: ^(int32 | undefined)[] = mapped.filter<int32 | undefined>(
    (value: &immutable (int32 | undefined)): boolean => (value as int32 | undefined) != undefined,
);

filtered;

=== dir ===
declare const values: (int32 | undefined)[];
/// @type.symbol symbol=values source=values type=int32 | undefined[]
/// @resolution.pattern source=values kind=binding target=values

const mapped = values.map((value) => value);
/// @type.symbol symbol=mapped source=mapped type=^int32 | undefined[]
/// @resolution.pattern source=mapped kind=binding target=mapped
/// @resolution.name source=values target=values
/// @resolution.member source=values.map receiver=int32 | undefined[] type=<map.U#2, map#2.'a>(this: &map#2.'a readonly int32 | undefined[], (int32 | undefined, isize) => map.U#2) => ^map.U#2[] kind=symbol target_receiver=int32 | undefined[] target=map#2
/// @resolution.call source="values.map((value) => value)" parameters=((int32 | undefined, isize) => int32 | undefined) arguments=(provided((value) => value) as (int32 | undefined, isize) => int32 | undefined) return=^int32 | undefined[] regions=("managed" & "local") kind=symbol target=map#2 receiver=int32 | undefined[] adjustments=(borrow(&'managed readonly int32 | undefined[])) instance="Array<int32 | undefined>.<extension#4>.map#2<int32 | undefined, \"managed\" & \"local\">"
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @generic.instantiation id="map#2<int32 | undefined, int32 | undefined, \"managed\" & \"local\">" template=map#2 arguments=(int32 | undefined, int32 | undefined, "managed" & "local")
/// @generic.instantiation id="map#2<int32 | undefined>" template=map#2 arguments=(int32 | undefined)
/// @type.symbol symbol=symbol2 source="(value) => value" type=Function<(int32 | undefined,), int32 | undefined, "readonly">
/// @type.symbol symbol=symbol2.value source=value type=int32 | undefined
/// @resolution.name source=value target=symbol2.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol2.value

const filtered = mapped.filter((value) => value != undefined);
/// @type.symbol symbol=filtered source=filtered type=^int32 | undefined[]
/// @resolution.pattern source=filtered kind=binding target=filtered
/// @resolution.name source=mapped target=mapped
/// @resolution.member source=mapped.filter receiver=^int32 | undefined[] type=(this: ^int32 | undefined[], <type_expression.'a>(&type_expression.'a immutable (int32 | undefined), isize) => boolean) => ^int32 | undefined[] kind=symbol target_receiver=^int32 | undefined[] target=filter#1
/// @resolution.call source="mapped.filter((value) => value != undefined)" parameters=(<type_expression.'a>(&type_expression.'a immutable (int32 | undefined), isize) => boolean) arguments=(provided((value) => value != undefined) as <type_expression.'a>(&type_expression.'a immutable (int32 | undefined), isize) => boolean) return=^int32 | undefined[] kind=symbol target=filter#1 receiver=^int32 | undefined[] instance=^T#3[].<extension#3>.filter#1
/// @resolution.place source=mapped placement="local" lifetime="static" access="immutable"
/// @resolution.access source=mapped root=mapped
/// @generic.instantiation id="filter#1<int32 | undefined>" template=filter#1 arguments=(int32 | undefined)
/// @type.symbol symbol=symbol5 source="(value) => value != undefined" type=Function<(&type_expression.'a immutable (int32 | undefined),), boolean, "readonly">
/// @type.symbol symbol=symbol5.value source=value type=&type_expression.'a immutable (int32 | undefined)
/// @resolution.name source=value target=symbol5.value
/// @resolution.operator source="value != undefined" type=boolean operator="!=" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined as undefined families=(undefined)]
/// @resolution.place source=value placement=type_expression.'a lifetime=type_expression.'a access="immutable"
/// @resolution.access source=value root=symbol5.value

filtered;
/// @resolution.name source=filtered target=filtered
/// @resolution.place source=filtered placement="local" lifetime="static" access="immutable"
/// @resolution.access source=filtered root=filtered
"#,
        r#"

"#,
    );
}

#[test]
fn test_read_borrowed_copy_value_at_argument() {
    let session = TestSession::single(
        r#"
declare function consume(value: int32): void;

declare const shared: &'static int32;
declare const exclusive: &'static int32;

consume(shared);
consume(exclusive);

const kept = shared;
kept;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare function consume(value: int32): void;

declare const shared: &'static int32;
declare const exclusive: &'static int32;

consume(shared as int32);
consume(exclusive as int32);

const kept: &'static int32 = shared;
kept;

=== dir ===
declare function consume(value: int32): void;
/// @type.symbol symbol=consume source="declare function consume(value: int32): void" type=(int32) => void

declare const shared: &'static int32;
/// @type.symbol symbol=shared source=shared type=&'static int32
/// @resolution.pattern source=shared kind=binding target=shared

declare const exclusive: &'static int32;
/// @type.symbol symbol=exclusive source=exclusive type=&'static int32
/// @resolution.pattern source=exclusive kind=binding target=exclusive

consume(shared);
/// @resolution.name source=consume target=consume
/// @resolution.call source=consume(shared) parameters=(int32) arguments=(provided(shared) as int32) return=void kind=symbol target=consume
/// @resolution.name source=shared target=shared
/// @resolution.place source=shared placement="local" lifetime="static" access="immutable"
/// @resolution.access source=shared root=shared
/// @coercion.node source=shared from=&'static int32 adjustments=[{ kind: read, target: int32 }] origin=implicit

consume(exclusive);
/// @resolution.name source=consume target=consume
/// @resolution.call source=consume(exclusive) parameters=(int32) arguments=(provided(exclusive) as int32) return=void kind=symbol target=consume
/// @resolution.name source=exclusive target=exclusive
/// @resolution.place source=exclusive placement="local" lifetime="static" access="immutable"
/// @resolution.access source=exclusive root=exclusive
/// @coercion.node source=exclusive from=&'static int32 adjustments=[{ kind: read, target: int32 }] origin=implicit

const kept = shared;
/// @type.symbol symbol=kept source=kept type=&'static int32
/// @resolution.pattern source=kept kind=binding target=kept
/// @resolution.name source=shared target=shared
/// @resolution.access source=shared root=shared

kept;
/// @resolution.name source=kept target=kept
/// @resolution.place source=kept placement="local" lifetime="static" access="immutable"
/// @resolution.access source=kept root=kept
"#,
        r#"

"#,
    );
}

#[test]
fn test_keep_readonly_view_on_borrowed_managed_handle() {
    let session = TestSession::single(
        r#"
declare const borrow: &'static readonly Array<int32>;

const handle: Array<int32> = borrow;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare const borrow: &'static readonly int32[];

const handle: int32[] = borrow;

=== dir ===
declare const borrow: &'static readonly Array<int32>;
/// @type.symbol symbol=borrow source=borrow type=&'static readonly Array<int32>
/// @resolution.pattern source=borrow kind=binding target=borrow
/// @resolution.name source=Array target=Array

const handle: Array<int32> = borrow;
/// @type.symbol symbol=handle source=handle type=Array<int32>
/// @resolution.pattern source=handle kind=binding target=handle
/// @resolution.name source=Array target=Array
/// @resolution.name source=borrow target=borrow
/// @resolution.place source=borrow placement="local" lifetime="static" access="immutable"
/// @resolution.access source=borrow root=borrow
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '&'static readonly int32[]' is not assignable to type 'int32[]'"
/// @diagnostic.label line=4 column=30 span="borrow" line_source="const handle: Array<int32> = borrow;"
/// @diagnostic.related line=4 column=15 span="Array" line_source="const handle: Array<int32> = borrow;" message="expected due to this annotation"
"#,
    );
}
