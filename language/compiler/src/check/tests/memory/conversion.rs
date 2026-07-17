use crate::tests::{DirRows, TestSession};

#[test]
fn test_prefer_exact_managed_overload_over_earlier_borrow_overload() {
    let session = TestSession::single(
        r#"
class User {}

function select(value: &readonly User): "borrowed" {
    return "borrowed";
}

function select(value: User): "managed" {
    return "managed";
}

declare const user: local User;
const selected = select(user);

selected satisfies "managed";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function select<comptime L0: Lifetime>(value: Borrowed<User, L0, "readonly">): "borrowed" {
    return "borrowed";
}

function select(value: User): "managed" {
    return "managed";
}

declare const user: local User;
const selected: "managed" = select(user);

selected satisfies "managed";

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

function select(value: &readonly User): "borrowed" {
/// @generic.template symbol=select#1 parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=select#1 type=<comptime select#1.L0: Lifetime>(Borrowed<User, select#1.L0, "readonly">) => "borrowed"
/// @type.symbol symbol=select.value#1 source="value: &readonly User" type=Borrowed<User, select#1.L0, "readonly">
/// @resolution.name source=User target=User

    return "borrowed";
}

function select(value: User): "managed" {
/// @type.symbol symbol=select#2 type=(User) => "managed"
/// @type.symbol symbol=select.value#2 source="value: User" type=User
/// @resolution.name source=User target=User

    return "managed";
}

declare const user: local User;
/// @type.symbol symbol=user source=user type=Placed<User, "local">
/// @resolution.name source=User target=User

const selected = select(user);
/// @type.symbol symbol=selected source=selected type="managed"
/// @resolution.name source=select target=[select#1, select#2]
/// @resolution.call source=select(user) parameters=(User) arguments=(provided(user) as User) return="managed" kind=symbol target=select#2
/// @resolution.name source=user target=user

selected satisfies "managed";
/// @resolution.name source=selected target=selected
"#,
        r#"

"#,
    );
}

#[test]
fn test_select_first_overload_within_managed_borrow_coercion_tier() {
    let session = TestSession::single(
        r#"
class User {}

function select(value: &readonly User): "readonly" {
    return "readonly";
}

function select(value: &User): "mutable" {
    return "mutable";
}

declare const user: local User;
const selected = select(user);

selected satisfies "readonly";
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked().with_coercion(), r#"
=== annotated ===
class User {}

function select<comptime L0: Lifetime>(value: Borrowed<User, L0, "readonly">): "readonly" {
    return "readonly";
}

function select<comptime L0: Lifetime>(value: Borrowed<User, L0, "mutable">): "mutable" {
    return "mutable";
}

declare const user: local User;
const selected: "readonly" = select(user as Borrowed<User, "static", "readonly">);

selected satisfies "readonly";

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

function select(value: &readonly User): "readonly" {
/// @generic.template symbol=select#1 parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=select#1 type=<comptime select#1.L0: Lifetime>(Borrowed<User, select#1.L0, "readonly">) => "readonly"
/// @type.symbol symbol=select.value#1 source="value: &readonly User" type=Borrowed<User, select#1.L0, "readonly">
/// @resolution.name source=User target=User

    return "readonly";
}

function select(value: &User): "mutable" {
/// @generic.template symbol=select#2 parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=select#2 type=<comptime select#2.L0: Lifetime>(Borrowed<User, select#2.L0, "mutable">) => "mutable"
/// @type.symbol symbol=select.value#2 source="value: &User" type=Borrowed<User, select#2.L0, "mutable">
/// @resolution.name source=User target=User

    return "mutable";
}

declare const user: local User;
/// @type.symbol symbol=user source=user type=Placed<User, "local">
/// @resolution.name source=User target=User

const selected = select(user);
/// @type.symbol symbol=selected source=selected type="readonly"
/// @resolution.name source=select target=[select#1, select#2]
/// @resolution.call source=select(user) parameters=(Borrowed<User, "static", "readonly">) arguments=(provided(user) as Borrowed<User, "static", "readonly">) return="readonly" kind=symbol target=select#1
/// @resolution.name source=user target=user
/// @coercion.node source=user from=Placed<User, "local"> to=Borrowed<User, "static", "readonly"> origin=implicit

selected satisfies "readonly";
/// @resolution.name source=selected target=selected
"#, r#"

"#);
}

#[test]
fn test_prefer_exact_borrow_overload_over_earlier_reborrow_overload() {
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

selected satisfies "mutable";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function select<comptime L0: Lifetime>(value: Borrowed<User, L0, "readonly">): "readonly" {
    return "readonly";
}

function select<comptime L0: Lifetime>(value: Borrowed<User, L0, "mutable">): "mutable" {
    return "mutable";
}

declare const user: Borrowed<User, "static", "mutable">;
const selected: "mutable" = select(user);

selected satisfies "mutable";

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

function select(value: &readonly User): "readonly" {
/// @generic.template symbol=select#1 parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=select#1 type=<comptime select#1.L0: Lifetime>(Borrowed<User, select#1.L0, "readonly">) => "readonly"
/// @type.symbol symbol=select.value#1 source="value: &readonly User" type=Borrowed<User, select#1.L0, "readonly">
/// @resolution.name source=User target=User

    return "readonly";
}

function select(value: &User): "mutable" {
/// @generic.template symbol=select#2 parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=select#2 type=<comptime select#2.L0: Lifetime>(Borrowed<User, select#2.L0, "mutable">) => "mutable"
/// @type.symbol symbol=select.value#2 source="value: &User" type=Borrowed<User, select#2.L0, "mutable">
/// @resolution.name source=User target=User

    return "mutable";
}

declare const user: &User;
/// @type.symbol symbol=user source=user type=Borrowed<User, "static", "mutable">
/// @resolution.name source=User target=User

const selected = select(user);
/// @type.symbol symbol=selected source=selected type="mutable"
/// @resolution.name source=select target=[select#1, select#2]
/// @resolution.call source=select(user) parameters=(Borrowed<User, "static", "mutable">) arguments=(provided(user) as Borrowed<User, "static", "mutable">) return="mutable" kind=symbol target=select#2
/// @resolution.name source=user target=user

selected satisfies "mutable";
/// @resolution.name source=selected target=selected
"#,
        r#""#,
    );
}

#[test]
fn test_coerce_local_managed_value_to_readonly_mutable_and_exclusive_borrows() {
    let session = TestSession::single(
        r#"
class User {}

declare function inspect(value: &readonly User): void;
declare function modify(value: &User): void;
declare function replace(value: &exclusive User): void;

const user: User = new User();

inspect(user);
modify(user);
replace(user);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function inspect<comptime L0: Lifetime>(value: Borrowed<User, L0, "readonly">): void;
declare function modify<comptime L0: Lifetime>(value: Borrowed<User, L0, "mutable">): void;
declare function replace<comptime L0: Lifetime>(value: Borrowed<User, L0, "exclusive">): void;

const user: User = new User();

inspect(user as Borrowed<User, "static", "readonly">);
modify(user as Borrowed<User, "static", "mutable">);
replace(user as Borrowed<User, "static", "exclusive">);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function inspect(value: &readonly User): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly User): void" type=<comptime inspect.L0: Lifetime>(Borrowed<User, inspect.L0, "readonly">) => void
/// @type.symbol symbol=inspect.value source="value: &readonly User" type=Borrowed<User, inspect.L0, "readonly">
/// @resolution.name source=User target=User

declare function modify(value: &User): void;
/// @generic.template symbol=modify parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=modify source="declare function modify(value: &User): void" type=<comptime modify.L0: Lifetime>(Borrowed<User, modify.L0, "mutable">) => void
/// @type.symbol symbol=modify.value source="value: &User" type=Borrowed<User, modify.L0, "mutable">
/// @resolution.name source=User target=User

declare function replace(value: &exclusive User): void;
/// @generic.template symbol=replace parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=replace source="declare function replace(value: &exclusive User): void" type=<comptime replace.L0: Lifetime>(Borrowed<User, replace.L0, "exclusive">) => void
/// @type.symbol symbol=replace.value source="value: &exclusive User" type=Borrowed<User, replace.L0, "exclusive">
/// @resolution.name source=User target=User

const user: User = new User();
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User
/// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
/// @resolution.name source=User target=User

inspect(user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(Borrowed<User, "static", "readonly">) arguments=(provided(user) as Borrowed<User, "static", "readonly">) return=void kind=symbol target=inspect
/// @resolution.name source=user target=user
/// @coercion.node source=user from=User to=Borrowed<User, "static", "readonly"> origin=implicit

modify(user);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(user) parameters=(Borrowed<User, "static", "mutable">) arguments=(provided(user) as Borrowed<User, "static", "mutable">) return=void kind=symbol target=modify
/// @resolution.name source=user target=user
/// @coercion.node source=user from=User to=Borrowed<User, "static", "mutable"> origin=implicit

replace(user);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(user) parameters=(Borrowed<User, "static", "exclusive">) arguments=(provided(user) as Borrowed<User, "static", "exclusive">) return=void kind=symbol target=replace
/// @resolution.name source=user target=user
/// @coercion.node source=user from=User to=Borrowed<User, "static", "exclusive"> origin=implicit
"#,
    );
}

#[test]
fn test_coerce_local_owned_value_to_readonly_mutable_and_exclusive_borrows() {
    let session = TestSession::single(
        r#"
class User {}

declare function inspect(value: local &readonly User): void;
declare function modify(value: local &User): void;
declare function replace(value: local &exclusive User): void;

declare const user: local ^User;

inspect(user);
modify(user);
replace(user);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function inspect<comptime L0: Lifetime>(value: local Borrowed<User, L0, "readonly">): void;
declare function modify<comptime L0: Lifetime>(value: local Borrowed<User, L0, "mutable">): void;
declare function replace<comptime L0: Lifetime>(value: local Borrowed<User, L0, "exclusive">): void;

declare const user: local ^User;

inspect(user as local Borrowed<User, "static", "readonly">);
modify(user as local Borrowed<User, "static", "mutable">);
replace(user as local Borrowed<User, "static", "exclusive">);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function inspect(value: local &readonly User): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: local &readonly User): void" type=<comptime inspect.L0: Lifetime>(Placed<Borrowed<User, inspect.L0, "readonly">, "local">) => void
/// @type.symbol symbol=inspect.value source="value: local &readonly User" type=Placed<Borrowed<User, inspect.L0, "readonly">, "local">
/// @resolution.name source=User target=User

declare function modify(value: local &User): void;
/// @generic.template symbol=modify parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=modify source="declare function modify(value: local &User): void" type=<comptime modify.L0: Lifetime>(Placed<Borrowed<User, modify.L0, "mutable">, "local">) => void
/// @type.symbol symbol=modify.value source="value: local &User" type=Placed<Borrowed<User, modify.L0, "mutable">, "local">
/// @resolution.name source=User target=User

declare function replace(value: local &exclusive User): void;
/// @generic.template symbol=replace parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=replace source="declare function replace(value: local &exclusive User): void" type=<comptime replace.L0: Lifetime>(Placed<Borrowed<User, replace.L0, "exclusive">, "local">) => void
/// @type.symbol symbol=replace.value source="value: local &exclusive User" type=Placed<Borrowed<User, replace.L0, "exclusive">, "local">
/// @resolution.name source=User target=User

declare const user: local ^User;
/// @type.symbol symbol=user source=user type=Placed<Owned<User>, "local">
/// @resolution.name source=User target=User

inspect(user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(Placed<Borrowed<User, "static", "readonly">, "local">) arguments=(provided(user) as Placed<Borrowed<User, "static", "readonly">, "local">) return=void kind=symbol target=inspect
/// @resolution.name source=user target=user
/// @coercion.node source=user from=Placed<Owned<User>, "local"> to=Placed<Borrowed<User, "static", "readonly">, "local"> origin=implicit

modify(user);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(user) parameters=(Placed<Borrowed<User, "static", "mutable">, "local">) arguments=(provided(user) as Placed<Borrowed<User, "static", "mutable">, "local">) return=void kind=symbol target=modify
/// @resolution.name source=user target=user
/// @coercion.node source=user from=Placed<Owned<User>, "local"> to=Placed<Borrowed<User, "static", "mutable">, "local"> origin=implicit

replace(user);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(user) parameters=(Placed<Borrowed<User, "static", "exclusive">, "local">) arguments=(provided(user) as Placed<Borrowed<User, "static", "exclusive">, "local">) return=void kind=symbol target=replace
/// @resolution.name source=user target=user
/// @coercion.node source=user from=Placed<Owned<User>, "local"> to=Placed<Borrowed<User, "static", "exclusive">, "local"> origin=implicit
"#,
    );
}

#[test]
fn test_coerce_readonly_managed_value_to_readonly_borrow() {
    let session = TestSession::single(
        r#"
class User {}

declare function inspect(value: &readonly User): void;
declare const user: local readonly User;

inspect(user);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare function inspect<comptime L0: Lifetime>(value: Borrowed<User, L0, "readonly">): void;
declare const user: local readonly User;

inspect(user as Borrowed<User, "static", "readonly">);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function inspect(value: &readonly User): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly User): void" type=<comptime inspect.L0: Lifetime>(Borrowed<User, inspect.L0, "readonly">) => void
/// @type.symbol symbol=inspect.value source="value: &readonly User" type=Borrowed<User, inspect.L0, "readonly">
/// @resolution.name source=User target=User

declare const user: local readonly User;
/// @type.symbol symbol=user source=user type=Placed<Readonly<User>, "local">
/// @resolution.name source=User target=User

inspect(user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(Borrowed<User, "static", "readonly">) arguments=(provided(user) as Borrowed<User, "static", "readonly">) return=void kind=symbol target=inspect
/// @resolution.name source=user target=user
/// @coercion.node source=user from=Placed<Readonly<User>, "local"> to=Borrowed<User, "static", "readonly"> origin=implicit
"#,
    );
}

#[test]
fn test_reject_mutable_and_exclusive_borrows_from_readonly_managed_value() {
    let session = TestSession::single(
        r#"
class User {}

declare function modify(value: local &User): void;
declare function replace(value: local &exclusive User): void;
declare const user: local readonly User;

modify(user);
replace(user);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare function modify<comptime L0: Lifetime>(value: local Borrowed<User, L0, "mutable">): void;
declare function replace<comptime L0: Lifetime>(value: local Borrowed<User, L0, "exclusive">): void;
declare const user: local readonly User;

modify(user);
replace(user);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function modify(value: local &User): void;
/// @generic.template symbol=modify parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=modify source="declare function modify(value: local &User): void" type=<comptime modify.L0: Lifetime>(Placed<Borrowed<User, modify.L0, "mutable">, "local">) => void
/// @type.symbol symbol=modify.value source="value: local &User" type=Placed<Borrowed<User, modify.L0, "mutable">, "local">
/// @resolution.name source=User target=User

declare function replace(value: local &exclusive User): void;
/// @generic.template symbol=replace parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=replace source="declare function replace(value: local &exclusive User): void" type=<comptime replace.L0: Lifetime>(Placed<Borrowed<User, replace.L0, "exclusive">, "local">) => void
/// @type.symbol symbol=replace.value source="value: local &exclusive User" type=Placed<Borrowed<User, replace.L0, "exclusive">, "local">
/// @resolution.name source=User target=User

declare const user: local readonly User;
/// @type.symbol symbol=user source=user type=Placed<Readonly<User>, "local">
/// @resolution.name source=User target=User

modify(user);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(user) parameters=(Placed<Borrowed<User, <error>, "mutable">, "local">) arguments=(provided(user) as Placed<Borrowed<User, <error>, "mutable">, "local">) return=void kind=symbol target=modify
/// @resolution.name source=user target=user

replace(user);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(user) parameters=(Placed<Borrowed<User, <error>, "exclusive">, "local">) arguments=(provided(user) as Placed<Borrowed<User, <error>, "exclusive">, "local">) return=void kind=symbol target=replace
/// @resolution.name source=user target=user
"#,
        r#"
/// @diagnostic.error code=EC209 message="argument of type 'local readonly User' is not assignable to parameter of type 'local &User'"
/// @diagnostic.label line=8 column=8 span="user" line_source="modify(user);"
/// @diagnostic.related line=8 column=1 span="modify(user)" line_source="modify(user);" message="in this call"
/// @diagnostic.error code=EC209 message="argument of type 'local readonly User' is not assignable to parameter of type 'local &exclusive User'"
/// @diagnostic.label line=9 column=9 span="user" line_source="replace(user);"
/// @diagnostic.related line=9 column=1 span="replace(user)" line_source="replace(user);" message="in this call"
"#,
    );
}

#[test]
fn test_coerce_explicit_and_aliased_managed_forms_to_borrow() {
    let session = TestSession::single(
        r#"
struct Cell {}

type ManagedCell = Managed<Cell>;

declare function inspect(value: &readonly Cell): void;
declare const explicit: Managed<Cell>;
declare const alias: ManagedCell;

inspect(explicit);
inspect(alias);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
struct Cell {}

type ManagedCell = Managed<Cell>;

declare function inspect<comptime L0: Lifetime>(value: Borrowed<Cell, L0, "readonly">): void;
declare const explicit: Managed<Cell>;
declare const alias: ManagedCell;

inspect(explicit as Borrowed<Cell, "static", "readonly">);
inspect(alias as Borrowed<Cell, "static", "readonly">);

=== checked ===
struct Cell {}
/// @type.symbol symbol=Cell source="struct Cell {}" type=Cell
/// @definition.struct symbol=Cell source="struct Cell {}"

type ManagedCell = Managed<Cell>;
/// @type.symbol symbol=ManagedCell source="type ManagedCell = Managed<Cell>" type=Managed<Cell>
/// @definition.type symbol=ManagedCell source="type ManagedCell = Managed<Cell>" value=Managed<Cell>
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Cell target=Cell

declare function inspect(value: &readonly Cell): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly Cell): void" type=<comptime inspect.L0: Lifetime>(Borrowed<Cell, inspect.L0, "readonly">) => void
/// @type.symbol symbol=inspect.value source="value: &readonly Cell" type=Borrowed<Cell, inspect.L0, "readonly">
/// @resolution.name source=Cell target=Cell

declare const explicit: Managed<Cell>;
/// @type.symbol symbol=explicit source=explicit type=Managed<Cell>
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Cell target=Cell

declare const alias: ManagedCell;
/// @type.symbol symbol=alias source=alias type=ManagedCell reduced=Managed<Cell>
/// @resolution.name source=ManagedCell target=ManagedCell

inspect(explicit);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(explicit) parameters=(Borrowed<Cell, "static", "readonly">) arguments=(provided(explicit) as Borrowed<Cell, "static", "readonly">) return=void kind=symbol target=inspect
/// @resolution.name source=explicit target=explicit
/// @coercion.node source=explicit from=Managed<Cell> to=Borrowed<Cell, "static", "readonly"> origin=implicit

inspect(alias);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(alias) parameters=(Borrowed<Cell, "static", "readonly">) arguments=(provided(alias) as Borrowed<Cell, "static", "readonly">) return=void kind=symbol target=inspect
/// @resolution.name source=alias target=alias
/// @coercion.node source=alias from=ManagedCell to=Borrowed<Cell, "static", "readonly"> origin=implicit

/// @generic.instance id=Managed<Cell> template=memory.managed.Managed arguments=(Cell)
"#,
        r#"

"#,
    );
}

#[test]
fn test_preserve_nominal_managed_identity_in_borrow_coercion() {
    let session = TestSession::single(
        r#"
struct Cell {}

newtype ManagedCellId = Managed<Cell>;

declare function inspect(value: &readonly ManagedCellId): void;
declare const cell: ManagedCellId;

inspect(cell);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
struct Cell {}

newtype ManagedCellId = Managed<Cell>;

declare function inspect<comptime L0: Lifetime>(
    value: Borrowed<ManagedCellId, L0, "readonly">,
): void;
declare const cell: ManagedCellId;

inspect(cell as Borrowed<ManagedCellId, "static", "readonly">);

=== checked ===
struct Cell {}
/// @type.symbol symbol=Cell source="struct Cell {}" type=Cell
/// @definition.struct symbol=Cell source="struct Cell {}"

newtype ManagedCellId = Managed<Cell>;
/// @type.symbol symbol=ManagedCellId source="newtype ManagedCellId = Managed<Cell>" type=ManagedCellId
/// @definition.newtype symbol=ManagedCellId source="newtype ManagedCellId = Managed<Cell>" value=Managed<Cell>
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Cell target=Cell

declare function inspect(value: &readonly ManagedCellId): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly ManagedCellId): void" type=<comptime inspect.L0: Lifetime>(Borrowed<ManagedCellId, inspect.L0, "readonly">) => void
/// @type.symbol symbol=inspect.value source="value: &readonly ManagedCellId" type=Borrowed<ManagedCellId, inspect.L0, "readonly">
/// @resolution.name source=ManagedCellId target=ManagedCellId

declare const cell: ManagedCellId;
/// @type.symbol symbol=cell source=cell type=ManagedCellId
/// @resolution.name source=ManagedCellId target=ManagedCellId

inspect(cell);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(cell) parameters=(Borrowed<ManagedCellId, "static", "readonly">) arguments=(provided(cell) as Borrowed<ManagedCellId, "static", "readonly">) return=void kind=symbol target=inspect
/// @resolution.name source=cell target=cell
/// @coercion.node source=cell from=ManagedCellId to=Borrowed<ManagedCellId, "static", "readonly"> origin=implicit
"#,
        r#"

"#,
    );
}

#[test]
fn test_coerce_local_readonly_managed_form_to_readonly_borrow() {
    let session = TestSession::single(
        r#"
struct Cell {}

declare function inspect(value: &readonly Cell): void;
declare const localCell: local Managed<Cell>;
declare const readonlyCell: local readonly Managed<Cell>;

inspect(localCell);
inspect(readonlyCell);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
struct Cell {}

declare function inspect<comptime L0: Lifetime>(value: Borrowed<Cell, L0, "readonly">): void;
declare const localCell: local Managed<Cell>;
declare const readonlyCell: local readonly Managed<Cell>;

inspect(localCell as Borrowed<Cell, "static", "readonly">);
inspect(readonlyCell as Borrowed<Cell, "static", "readonly">);

=== checked ===
struct Cell {}
/// @type.symbol symbol=Cell source="struct Cell {}" type=Cell
/// @definition.struct symbol=Cell source="struct Cell {}"

declare function inspect(value: &readonly Cell): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly Cell): void" type=<comptime inspect.L0: Lifetime>(Borrowed<Cell, inspect.L0, "readonly">) => void
/// @type.symbol symbol=inspect.value source="value: &readonly Cell" type=Borrowed<Cell, inspect.L0, "readonly">
/// @resolution.name source=Cell target=Cell

declare const localCell: local Managed<Cell>;
/// @type.symbol symbol=localCell source=localCell type=Placed<Managed<Cell>, "local">
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Cell target=Cell

declare const readonlyCell: local readonly Managed<Cell>;
/// @type.symbol symbol=readonlyCell source=readonlyCell type=Placed<Readonly<Managed<Cell>>, "local">
/// @resolution.name source=Managed target=memory.managed.Managed
/// @resolution.name source=Cell target=Cell

inspect(localCell);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(localCell) parameters=(Borrowed<Cell, "static", "readonly">) arguments=(provided(localCell) as Borrowed<Cell, "static", "readonly">) return=void kind=symbol target=inspect
/// @resolution.name source=localCell target=localCell
/// @coercion.node source=localCell from=Placed<Managed<Cell>, "local"> to=Borrowed<Cell, "static", "readonly"> origin=implicit

inspect(readonlyCell);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(readonlyCell) parameters=(Borrowed<Cell, "static", "readonly">) arguments=(provided(readonlyCell) as Borrowed<Cell, "static", "readonly">) return=void kind=symbol target=inspect
/// @resolution.name source=readonlyCell target=readonlyCell
/// @coercion.node source=readonlyCell from=Placed<Readonly<Managed<Cell>>, "local"> to=Borrowed<Cell, "static", "readonly"> origin=implicit

/// @generic.instance id=Managed<Cell> template=memory.managed.Managed arguments=(Cell)
"#,
        r#"

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
    user!: User;
    boxed!: Box<User>;
    users!: User[];
}

declare function inspect(value: &readonly User): void;
declare const state: State;

inspect(state.user);
inspect(state.boxed.value);
inspect(state.users[0]);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

struct Box<out T> {
    value: T;
}

class State {
    user!: User;
    boxed!: Box<User>;
    users!: User[];
}

declare function inspect<comptime L0: Lifetime>(value: Borrowed<User, L0, "readonly">): void;
declare const state: State;

inspect(state.user as Borrowed<User, "static", "readonly">);
inspect(state.boxed.value as Borrowed<User, "static", "readonly">);
inspect(state.users[0] as Borrowed<User, "static", "readonly">);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
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
/// @type.symbol symbol=State type=State
/// @definition.class symbol=State
/// @definition.field symbol=State.boxed source="boxed!: Box<User>" key=boxed type=Box<User>
/// @definition.field symbol=State.user source="user!: User" key=user type=User
/// @definition.field symbol=State.users source="users!: User[]" key=users type=Array<User>

    user!: User;
    /// @type.symbol symbol=State.user source="user!: User" type=User
    /// @resolution.name source=User target=User

    boxed!: Box<User>;
    /// @type.symbol symbol=State.boxed source="boxed!: Box<User>" type=Box<User>
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=User target=User

    users!: User[];
    /// @type.symbol symbol=State.users source="users!: User[]" type=Array<User>
    /// @resolution.name source=User target=User

}

declare function inspect(value: &readonly User): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly User): void" type=<comptime inspect.L0: Lifetime>(Borrowed<User, inspect.L0, "readonly">) => void
/// @type.symbol symbol=inspect.value source="value: &readonly User" type=Borrowed<User, inspect.L0, "readonly">
/// @resolution.name source=User target=User

declare const state: State;
/// @type.symbol symbol=state source=state type=State
/// @resolution.name source=State target=State

inspect(state.user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(state.user) parameters=(Borrowed<User, "static", "readonly">) arguments=(provided(state.user) as Borrowed<User, "static", "readonly">) return=void kind=symbol target=inspect
/// @resolution.name source=state target=state
/// @resolution.member source=state.user receiver=State kind=symbol target=State.user
/// @coercion.node source=state.user from=User to=Borrowed<User, "static", "readonly"> origin=implicit

inspect(state.boxed.value);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(state.boxed.value) parameters=(Borrowed<User, "static", "readonly">) arguments=(provided(state.boxed.value) as Borrowed<User, "static", "readonly">) return=void kind=symbol target=inspect
/// @resolution.name source=state target=state
/// @resolution.member source=state.boxed receiver=State kind=symbol target=State.boxed
/// @resolution.member source=state.boxed.value receiver=Box<User> kind=symbol target=Box.value
/// @coercion.node source=state.boxed.value from=User to=Borrowed<User, "static", "readonly"> origin=implicit

inspect(state.users[0]);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(state.users[0]) parameters=(Borrowed<User, "static", "readonly">) arguments=(provided(state.users[0]) as Borrowed<User, "static", "readonly">) return=void kind=symbol target=inspect
/// @resolution.name source=state target=state
/// @resolution.member source=state.users receiver=State kind=symbol target=State.users
/// @resolution.call source=state.users[0] parameters=(usize) arguments=(provided(0) as usize) return=User kind=symbol target=collections.array.index#4 receiver=Array<User> instance=Array<User>.<extension#6>.index#4
/// @generic.instance source=state.users[0] id=Array<User>.<extension#6>.index#4
/// @coercion.node source=state.users[0] from=User to=Borrowed<User, "static", "readonly"> origin=implicit
/// @coercion.node source=0 from=0 to=usize origin=implicit

/// @generic.instance id=Array<User>.<extension#6>.index#4 template=collections.array.index#4 arguments=(User, User)
/// @generic.instance id=Box<User> template=Box arguments=(User)
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

const selected: &readonly User = condition ? first : second;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const condition: boolean;
declare const first: User;
declare const second: User;

const selected: Borrowed<User, "static", "readonly"> = (condition
    ? (first as Borrowed<User, "static", "readonly">)
    : (second as Borrowed<User, "static", "readonly">)) as Borrowed<User, "static", "readonly">;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const condition: boolean;
/// @type.symbol symbol=condition source=condition type=boolean

declare const first: User;
/// @type.symbol symbol=first source=first type=User
/// @resolution.name source=User target=User

declare const second: User;
/// @type.symbol symbol=second source=second type=User
/// @resolution.name source=User target=User

const selected: &readonly User = condition ? first : second;
/// @type.symbol symbol=selected source=selected type=Borrowed<User, "static", "readonly">
/// @resolution.name source=User target=User
/// @resolution.name source=condition target=condition
/// @coercion.node source="condition ? first : second" from=User to=Borrowed<User, "static", "readonly"> origin=implicit
/// @resolution.name source=first target=first
/// @coercion.node source=first from=User to=Borrowed<User, "static", "readonly"> origin=implicit
/// @resolution.name source=second target=second
/// @coercion.node source=second from=User to=Borrowed<User, "static", "readonly"> origin=implicit
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

const selected: &readonly User = match (choice) {
    "first" => first
    _ => second
};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const choice: "first" | "second";
declare const first: User;
declare const second: User;

const selected: Borrowed<User, "static", "readonly"> = match (choice) {
    "first" => first
    _ => second
} as Borrowed<User, "static", "readonly">;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const choice: "first" | "second";
/// @type.symbol symbol=choice source=choice type="first" | "second"

declare const first: User;
/// @type.symbol symbol=first source=first type=User
/// @resolution.name source=User target=User

declare const second: User;
/// @type.symbol symbol=second source=second type=User
/// @resolution.name source=User target=User

const selected: &readonly User = match (choice) {
/// @type.symbol symbol=selected source=selected type=Borrowed<User, "static", "readonly">
/// @resolution.name source=User target=User
/// @coercion.node from=User to=Borrowed<User, "static", "readonly"> origin=implicit
/// @resolution.name source=choice target=choice

    "first" => first
    /// @resolution.pattern source="\"first\"" kind=literal value="first"
    /// @resolution.name source=first target=first

    _ => second
    /// @resolution.pattern source=_ kind=wildcard
    /// @resolution.name source=second target=second

};
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

const selected: (&readonly User, &readonly User) = (first, second);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const first: User;
declare const second: User;

const selected: (Borrowed<User, "static", "readonly">, Borrowed<User, "static", "readonly">) = (
    first as Borrowed<User, "static", "readonly">,
    second as Borrowed<User, "static", "readonly">,
);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const first: User;
/// @type.symbol symbol=first source=first type=User
/// @resolution.name source=User target=User

declare const second: User;
/// @type.symbol symbol=second source=second type=User
/// @resolution.name source=User target=User

const selected: (&readonly User, &readonly User) = (first, second);
/// @type.symbol symbol=selected source=selected type=(Borrowed<User, "static", "readonly">, Borrowed<User, "static", "readonly">)
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User
/// @resolution.name source=first target=first
/// @coercion.node source=first from=User to=Borrowed<User, "static", "readonly"> origin=implicit
/// @resolution.name source=second target=second
/// @coercion.node source=second from=User to=Borrowed<User, "static", "readonly"> origin=implicit
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare function inspect<comptime L0: Lifetime>(value: Borrowed<string, L0, "readonly">): void;

inspect("message" as Borrowed<string, "frame", "readonly">);

=== checked ===
declare function inspect(value: &readonly string): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly string): void" type=<comptime inspect.L0: Lifetime>(Borrowed<string, inspect.L0, "readonly">) => void
/// @type.symbol symbol=inspect.value source="value: &readonly string" type=Borrowed<string, inspect.L0, "readonly">

inspect("message");
/// @resolution.name source=inspect target=inspect
/// @resolution.call source="inspect(\"message\")" parameters=(Borrowed<string, "frame", "readonly">) arguments=(provided("message") as Borrowed<string, "frame", "readonly">) return=void kind=symbol target=inspect
/// @coercion.node source="\"message\"" from="message" to=Borrowed<string, "frame", "readonly"> origin=implicit
"#,
    );
}

#[test]
fn test_preserve_shared_place_when_borrowing_shared_managed_value() {
    let session = TestSession::single(
        r#"
class User {}

declare const user: shared User;
declare function inspect(value: shared &readonly User): void;
declare function modify(value: shared &User): void;

inspect(user);
modify(user);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const user: shared User;
declare function inspect<comptime L0: Lifetime>(value: shared Borrowed<User, L0, "readonly">): void;
declare function modify<comptime L0: Lifetime>(value: shared Borrowed<User, L0, "mutable">): void;

inspect(user as shared Borrowed<User, "static", "readonly">);
modify(user as shared Borrowed<User, "static", "mutable">);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const user: shared User;
/// @type.symbol symbol=user source=user type=Placed<User, "shared">
/// @resolution.name source=User target=User

declare function inspect(value: shared &readonly User): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: shared &readonly User): void" type=<comptime inspect.L0: Lifetime>(Placed<Borrowed<User, inspect.L0, "readonly">, "shared">) => void
/// @type.symbol symbol=inspect.value source="value: shared &readonly User" type=Placed<Borrowed<User, inspect.L0, "readonly">, "shared">
/// @resolution.name source=User target=User

declare function modify(value: shared &User): void;
/// @generic.template symbol=modify parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=modify source="declare function modify(value: shared &User): void" type=<comptime modify.L0: Lifetime>(Placed<Borrowed<User, modify.L0, "mutable">, "shared">) => void
/// @type.symbol symbol=modify.value source="value: shared &User" type=Placed<Borrowed<User, modify.L0, "mutable">, "shared">
/// @resolution.name source=User target=User

inspect(user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(Placed<Borrowed<User, "static", "readonly">, "shared">) arguments=(provided(user) as Placed<Borrowed<User, "static", "readonly">, "shared">) return=void kind=symbol target=inspect
/// @resolution.name source=user target=user
/// @coercion.node source=user from=Placed<User, "shared"> to=Placed<Borrowed<User, "static", "readonly">, "shared"> origin=implicit

modify(user);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(user) parameters=(Placed<Borrowed<User, "static", "mutable">, "shared">) arguments=(provided(user) as Placed<Borrowed<User, "static", "mutable">, "shared">) return=void kind=symbol target=modify
/// @resolution.name source=user target=user
/// @coercion.node source=user from=Placed<User, "shared"> to=Placed<Borrowed<User, "static", "mutable">, "shared"> origin=implicit
"#,
    );
}

#[test]
fn test_reject_exclusive_borrow_from_shared_managed_value() {
    let session = TestSession::single(
        r#"
class User {}

declare const user: shared User;
declare function replace(value: shared &exclusive User): void;

replace(user);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const user: shared User;
declare function replace<comptime L0: Lifetime>(
    value: shared Borrowed<User, L0, "exclusive">,
): void;

replace(user);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const user: shared User;
/// @type.symbol symbol=user source=user type=Placed<User, "shared">
/// @resolution.name source=User target=User

declare function replace(value: shared &exclusive User): void;
/// @generic.template symbol=replace parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=replace source="declare function replace(value: shared &exclusive User): void" type=<comptime replace.L0: Lifetime>(Placed<Borrowed<User, replace.L0, "exclusive">, "shared">) => void
/// @type.symbol symbol=replace.value source="value: shared &exclusive User" type=Placed<Borrowed<User, replace.L0, "exclusive">, "shared">
/// @resolution.name source=User target=User

replace(user);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(user) parameters=(Placed<Borrowed<User, <error>, "exclusive">, "shared">) arguments=(provided(user) as Placed<Borrowed<User, <error>, "exclusive">, "shared">) return=void kind=symbol target=replace
/// @resolution.name source=user target=user
"#,
        r#"
/// @diagnostic.error code=EC209 message="argument of type 'shared User' is not assignable to parameter of type 'shared &exclusive User'"
/// @diagnostic.label line=7 column=9 span="user" line_source="replace(user);"
/// @diagnostic.related line=7 column=1 span="replace(user)" line_source="replace(user);" message="in this call"
"#,
    );
}

#[test]
fn test_coerce_shared_owned_value_to_readonly_mutable_and_exclusive_borrows() {
    let session = TestSession::single(
        r#"
class User {}

declare const user: shared ^User;
declare function inspect(value: shared &readonly User): void;
declare function modify(value: shared &User): void;
declare function replace(value: shared &exclusive User): void;

inspect(user);
modify(user);
replace(user);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const user: shared ^User;
declare function inspect<comptime L0: Lifetime>(value: shared Borrowed<User, L0, "readonly">): void;
declare function modify<comptime L0: Lifetime>(value: shared Borrowed<User, L0, "mutable">): void;
declare function replace<comptime L0: Lifetime>(
    value: shared Borrowed<User, L0, "exclusive">,
): void;

inspect(user as shared Borrowed<User, "static", "readonly">);
modify(user as shared Borrowed<User, "static", "mutable">);
replace(user as shared Borrowed<User, "static", "exclusive">);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const user: shared ^User;
/// @type.symbol symbol=user source=user type=Placed<Owned<User>, "shared">
/// @resolution.name source=User target=User

declare function inspect(value: shared &readonly User): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: shared &readonly User): void" type=<comptime inspect.L0: Lifetime>(Placed<Borrowed<User, inspect.L0, "readonly">, "shared">) => void
/// @type.symbol symbol=inspect.value source="value: shared &readonly User" type=Placed<Borrowed<User, inspect.L0, "readonly">, "shared">
/// @resolution.name source=User target=User

declare function modify(value: shared &User): void;
/// @generic.template symbol=modify parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=modify source="declare function modify(value: shared &User): void" type=<comptime modify.L0: Lifetime>(Placed<Borrowed<User, modify.L0, "mutable">, "shared">) => void
/// @type.symbol symbol=modify.value source="value: shared &User" type=Placed<Borrowed<User, modify.L0, "mutable">, "shared">
/// @resolution.name source=User target=User

declare function replace(value: shared &exclusive User): void;
/// @generic.template symbol=replace parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=replace source="declare function replace(value: shared &exclusive User): void" type=<comptime replace.L0: Lifetime>(Placed<Borrowed<User, replace.L0, "exclusive">, "shared">) => void
/// @type.symbol symbol=replace.value source="value: shared &exclusive User" type=Placed<Borrowed<User, replace.L0, "exclusive">, "shared">
/// @resolution.name source=User target=User

inspect(user);
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(Placed<Borrowed<User, "static", "readonly">, "shared">) arguments=(provided(user) as Placed<Borrowed<User, "static", "readonly">, "shared">) return=void kind=symbol target=inspect
/// @resolution.name source=user target=user
/// @coercion.node source=user from=Placed<Owned<User>, "shared"> to=Placed<Borrowed<User, "static", "readonly">, "shared"> origin=implicit

modify(user);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(user) parameters=(Placed<Borrowed<User, "static", "mutable">, "shared">) arguments=(provided(user) as Placed<Borrowed<User, "static", "mutable">, "shared">) return=void kind=symbol target=modify
/// @resolution.name source=user target=user
/// @coercion.node source=user from=Placed<Owned<User>, "shared"> to=Placed<Borrowed<User, "static", "mutable">, "shared"> origin=implicit

replace(user);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(user) parameters=(Placed<Borrowed<User, "static", "exclusive">, "shared">) arguments=(provided(user) as Placed<Borrowed<User, "static", "exclusive">, "shared">) return=void kind=symbol target=replace
/// @resolution.name source=user target=user
/// @coercion.node source=user from=Placed<Owned<User>, "shared"> to=Placed<Borrowed<User, "static", "exclusive">, "shared"> origin=implicit
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

declare function inspectPoint<comptime L0: Lifetime>(value: Borrowed<Point, L0, "readonly">): void;
declare function inspectValues<comptime L0: Lifetime>(
    value: Borrowed<int32[], L0, "readonly">,
): void;
declare const point: Point;
declare const values: int32[];

inspectPoint(point as Borrowed<Point, "static", "readonly">);
inspectValues(values as Borrowed<int32[], "static", "readonly">);

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

declare function inspectPoint(value: &readonly Point): void;
/// @generic.template symbol=inspectPoint parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspectPoint source="declare function inspectPoint(value: &readonly Point): void" type=<comptime inspectPoint.L0: Lifetime>(Borrowed<Point, inspectPoint.L0, "readonly">) => void
/// @type.symbol symbol=inspectPoint.value source="value: &readonly Point" type=Borrowed<Point, inspectPoint.L0, "readonly">
/// @resolution.name source=Point target=Point

declare function inspectValues(value: &readonly int32[]): void;
/// @generic.template symbol=inspectValues parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspectValues source="declare function inspectValues(value: &readonly int32[]): void" type=<comptime inspectValues.L0: Lifetime>(Borrowed<Array<int32>, inspectValues.L0, "readonly">) => void
/// @type.symbol symbol=inspectValues.value source="value: &readonly int32[]" type=Borrowed<Array<int32>, inspectValues.L0, "readonly">

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.name source=Point target=Point

declare const values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>

inspectPoint(point);
/// @resolution.name source=inspectPoint target=inspectPoint
/// @resolution.call source=inspectPoint(point) parameters=(Borrowed<Point, "static", "readonly">) arguments=(provided(point) as Borrowed<Point, "static", "readonly">) return=void kind=symbol target=inspectPoint
/// @resolution.name source=point target=point
/// @coercion.node source=point from=Point to=Borrowed<Point, "static", "readonly"> origin=implicit

inspectValues(values);
/// @resolution.name source=inspectValues target=inspectValues
/// @resolution.call source=inspectValues(values) parameters=(Borrowed<Array<int32>, "static", "readonly">) arguments=(provided(values) as Borrowed<Array<int32>, "static", "readonly">) return=void kind=symbol target=inspectValues
/// @resolution.name source=values target=values
/// @coercion.node source=values from=Array<int32> to=Borrowed<Array<int32>, "static", "readonly"> origin=implicit
"#,
    );
}

#[test]
fn test_reject_exclusive_receiver_borrow_from_shared_managed_value() {
    let session = TestSession::single(
        r#"
declare const values: shared int32[];

values.push(1);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: shared int32[];

values.push(1);

=== checked ===
declare const values: shared int32[];
/// @type.symbol symbol=values source=values type=Placed<Array<int32>, "shared">

values.push(1);
/// @resolution.name source=values target=values
/// @resolution.member source=values.push receiver=Placed<Array<int32>, "shared"> kind=existential targets=[collections.array.push#1, collections.array.push#2]
"#,
        r#"
/// @diagnostic.error code=EC302 message="no overload matches arguments ('1')"
/// @diagnostic.label line=4 column=1 span="values.push(1)" line_source="values.push(1);"
/// @diagnostic.note message="the candidate '<comptime L0: Lifetime>(int32) => void' rejects the receiver: 'shared Array<int32>' is not assignable to '&exclusive Array<int32>'"
/// @diagnostic.note message="the candidate '<comptime L0: Lifetime>(...int32[]) => float64' rejects the receiver: 'shared Array<int32>' is not assignable to '&exclusive Array<int32>'"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const user: User;
const same: User = user;

same satisfies User;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User

const same = user;
/// @type.symbol symbol=same source=same type=User
/// @resolution.name source=user target=user

same satisfies User;
/// @resolution.name source=same target=same
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}

let user: User = new User();
let owned: ^User = user;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

let user: User = new User();
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User
/// @type.node source="new User()" type=User
/// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
/// @resolution.name source=User target=User

let owned: ^User = user;
/// @type.symbol symbol=owned source=owned type=Owned<User>
/// @resolution.name source=User target=User
/// @type.node source=user type=User
/// @resolution.name source=user target=user
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'User' is not assignable to type '^User'"
/// @diagnostic.label line=5 column=20 span="user" line_source="let owned: ^User = user;"
"#,
    );
}

#[test]
fn test_reject_noncopyable_borrow_as_owned() {
    let session = TestSession::single(
        r#"
class Buffer {}
struct Label { buffer: ^Buffer; }

declare const label: ^Label;
let borrow = &label;
let owned: ^Label = borrow;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Buffer {}
struct Label {
    buffer: ^Buffer;
}

declare const label: ^Label;
let borrow: Borrowed<Label, "static", "mutable"> = &label;
let owned: ^Label = borrow;

=== checked ===
class Buffer {}
/// @type.symbol symbol=Buffer source="class Buffer {}" type=Buffer
/// @definition.class symbol=Buffer source="class Buffer {}"

struct Label { buffer: ^Buffer; }
/// @type.symbol symbol=Label source="struct Label { buffer: ^Buffer; }" type=Label
/// @definition.struct symbol=Label source="struct Label { buffer: ^Buffer; }"
/// @definition.field symbol=Label.buffer source="buffer: ^Buffer" key=buffer type=Owned<Buffer>
/// @type.symbol symbol=Label.buffer source="buffer: ^Buffer" type=Owned<Buffer>
/// @resolution.name source=Buffer target=Buffer

declare const label: ^Label;
/// @type.symbol symbol=label source=label type=Owned<Label> reduced=Label
/// @resolution.name source=Label target=Label

let borrow = &label;
/// @type.symbol symbol=borrow source=borrow type=Borrowed<Label, "static", "mutable">
/// @type.node source=&label type=Borrowed<Label, "static", "mutable">
/// @type.node source=label type=Owned<Label> reduced=Label
/// @resolution.name source=label target=label

let owned: ^Label = borrow;
/// @type.symbol symbol=owned source=owned type=Owned<Label> reduced=Label
/// @resolution.name source=Label target=Label
/// @type.node source=borrow type=Borrowed<Label, "static", "mutable">
/// @resolution.name source=borrow target=borrow
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '&Label' is not assignable to type '^Label'"
/// @diagnostic.label line=7 column=21 span="borrow" line_source="let owned: ^Label = borrow;"
/// @diagnostic.note message="'^Label' reduces to 'Label'"
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

let point = ^Point { x: 1 };
let borrow = &point;
let copied: Point = borrow;
let owned: ^Point = borrow;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

let point: ^Point = ^Point { x: 1 };
let borrow: Borrowed<Point, "static", "mutable"> = &point;
let copied: Point = borrow as Point;
let owned: ^Point = borrow as Point;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

let point = ^Point { x: 1 };
/// @type.symbol symbol=point source=point type=Owned<Point> reduced=Point
/// @type.node source="^Point { x: 1 }" type=Owned<Point> reduced=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 to=int32 origin=implicit

let borrow = &point;
/// @type.symbol symbol=borrow source=borrow type=Borrowed<Point, "static", "mutable">
/// @type.node source=&point type=Borrowed<Point, "static", "mutable">
/// @type.node source=point type=Owned<Point> reduced=Point
/// @resolution.name source=point target=point

let copied: Point = borrow;
/// @type.symbol symbol=copied source=copied type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=borrow type=Borrowed<Point, "static", "mutable">
/// @resolution.name source=borrow target=borrow
/// @coercion.node source=borrow from=Borrowed<Point, "static", "mutable"> to=Point origin=implicit

let owned: ^Point = borrow;
/// @type.symbol symbol=owned source=owned type=Owned<Point> reduced=Point
/// @resolution.name source=Point target=Point
/// @type.node source=borrow type=Borrowed<Point, "static", "mutable">
/// @resolution.name source=borrow target=borrow
/// @coercion.node source=borrow from=Borrowed<Point, "static", "mutable"> to=Point origin=implicit
"#,
        r#""#,
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}

let owned: ^User = new User();

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

let owned: ^User = new User();
/// @type.symbol symbol=owned source=owned type=Owned<User>
/// @resolution.name source=User target=User
/// @type.node source="new User()" type=Owned<User>
/// @resolution.construct source="new User()" parameters=() return=Owned<User> kind=class target=User constructor=default
/// @resolution.name source=User target=User
"#,
        r#""#,
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

let point: Point = Point { x: 1 };
let owned: ^Point = point;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

let point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

let owned: ^Point = point;
/// @type.symbol symbol=owned source=owned type=Owned<Point> reduced=Point
/// @resolution.name source=Point target=Point
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
"#,
        r#""#,
    );
}

#[test]
fn test_copy_bounded_value_into_owned_result() {
    let session = TestSession::single(
        r#"
import { Copy } from "destack:memory";

function duplicate<T: Copy>(value: T): ^T {
    value
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Copy } from "destack:memory";

function duplicate<T: Copy>(value: T): ^T {
    value as ^T
}

=== checked ===
import { Copy } from "destack:memory";

function duplicate<T: Copy>(value: T): ^T {
/// @generic.template symbol=duplicate parameters=(T: memory.capability.Copy)
/// @type.symbol symbol=duplicate type=<T: memory.capability.Copy>(T) => Owned<T>
/// @type.symbol symbol=duplicate.T source="T: Copy" type=T
/// @resolution.name source=Copy target=memory.capability.Copy
/// @type.symbol symbol=duplicate.value source="value: T" type=T
/// @resolution.name source=T target=duplicate.T
/// @resolution.name source=T target=duplicate.T

    value
    /// @resolution.name source=value target=duplicate.value

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
import { Copy } from "destack:memory";

class Session {}

declare function duplicate<T: Copy>(value: T): ^T;
declare const session: ^Session;

duplicate(32);
duplicate(session);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Copy } from "destack:memory";

class Session {}

declare function duplicate<T: Copy>(value: T): ^T;
declare const session: ^Session;

duplicate<32>(32);
duplicate<^Session>(session);

=== checked ===
import { Copy } from "destack:memory";

class Session {}
/// @type.symbol symbol=Session source="class Session {}" type=Session
/// @definition.class symbol=Session source="class Session {}"

declare function duplicate<T: Copy>(value: T): ^T;
/// @generic.template symbol=duplicate parameters=(T: memory.capability.Copy)
/// @type.symbol symbol=duplicate source="declare function duplicate<T: Copy>(value: T): ^T" type=<T: memory.capability.Copy>(T) => Owned<T>
/// @type.symbol symbol=duplicate.T source="T: Copy" type=T
/// @resolution.name source=Copy target=memory.capability.Copy
/// @type.symbol symbol=duplicate.value source="value: T" type=T
/// @resolution.name source=T target=duplicate.T
/// @resolution.name source=T target=duplicate.T

declare const session: ^Session;
/// @type.symbol symbol=session source=session type=Owned<Session>
/// @resolution.name source=Session target=Session

duplicate(32);
/// @resolution.name source=duplicate target=duplicate
/// @resolution.call source=duplicate(32) parameters=(32) arguments=(provided(32) as 32) return=Owned<32> kind=symbol target=duplicate instance=duplicate<32>
/// @generic.instance source=duplicate(32) id=duplicate<32>

duplicate(session);
/// @resolution.name source=duplicate target=duplicate
/// @resolution.call source=duplicate(session) parameters=(Owned<Session>) arguments=(provided(session) as Owned<Session>) return=Owned<Owned<Session>> kind=symbol target=duplicate instance=duplicate<Owned<Session>>
/// @generic.instance source=duplicate(session) id=duplicate<Owned<Session>>
/// @resolution.name source=session target=session

/// @generic.instance id=duplicate<32> template=duplicate arguments=(32)
/// @generic.instance id=duplicate<Owned<Session>> template=duplicate arguments=(Owned<Session>)
"#,
        r#"
/// @diagnostic.error code=EC201 message="type '^Session' does not satisfy 'Copy'"
/// @diagnostic.label line=10 column=1 span="duplicate(session)" line_source="duplicate(session);"
/// @diagnostic.related line=6 column=28 span="T" line_source="declare function duplicate<T: Copy>(value: T): ^T;" message="required by this bound on 'T'"
"#,
    );
}
