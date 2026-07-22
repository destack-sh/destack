use crate::tests::TestSession;

#[test]
fn test_lower_lifetime_parameters_to_polymorphic_mir_slots() {
    let session = TestSession::single(
        r#"
struct User {
    id: int32;
}

function identity(value: &readonly User): &readonly User {
    return value;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type User {
    id: int32;
}

function main.identity<L0: lifetime>(v0: ref<User, borrowed, lifetime(L0), readonly>): ref<User, borrowed, lifetime(L0), readonly> {
entry(v0: ref<User, borrowed, lifetime(L0), readonly>):
    return v0
}
/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_explicit_borrowed_type_to_reference() {
    let session = TestSession::single(
        r#"
struct User {
    id: int32;
}

function identity<comptime L: Lifetime>(
    value: Borrowed<User, L, "readonly">,
): Borrowed<User, L, "readonly"> {
    return value;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type User {
    id: int32;
}

function main.identity<L0: lifetime>(v0: ref<User, borrowed, lifetime(L0), readonly>): ref<User, borrowed, lifetime(L0), readonly> {
entry(v0: ref<User, borrowed, lifetime(L0), readonly>):
    return v0
}
/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_lifetime_unions_to_combined_mir_provenance() {
    let session = TestSession::single(
        r#"
struct User {
    id: int32;
}

function identity<comptime L0: Lifetime, comptime L1: Lifetime>(
    value: Borrowed<User, L0 | L1, "readonly">,
): Borrowed<User, L0 | L1, "readonly"> {
    return value;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type User {
    id: int32;
}

function main.identity<L0: lifetime, L1: lifetime>(v0: ref<User, borrowed, lifetime(L0, L1), readonly>): ref<User, borrowed, lifetime(L0, L1), readonly> {
entry(v0: ref<User, borrowed, lifetime(L0, L1), readonly>):
    return v0
}
/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_shares_one_polymorphic_nominal_across_lifetime_applications() {
    let session = TestSession::single(
        r#"
struct User {
    id: int32;
}

struct View {
    user: &readonly User;
}

function retain(value: View): View {
    return value;
}

function get(value: View): &readonly User {
    return value.user;
}

function retainStatic(value: View<"static">): View<"static"> {
    return value;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type User {
    id: int32;
}

@copy
type View<L0: lifetime> {
    user: ref<User, borrowed, lifetime(L0), readonly>;
}

function main.retain<L0: lifetime>(v0: View<lifetime(L0)>): View<lifetime(L0)> {
entry(v0: View<lifetime(L0)>):
    return v0
}

function main.get<L0: lifetime>(v0: View<lifetime(L0)>): ref<User, borrowed, lifetime(L0), readonly> {
entry(v0: View<lifetime(L0)>):
    v1: ref<User, borrowed, lifetime(L0), readonly> = field.get v0, 0
    return v1
}

function main.retainStatic(v0: View<lifetime(static)>): View<lifetime(static)> {
entry(v0: View<lifetime(static)>):
    return v0
}
/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
/// @layout.struct name=View size=8 align=8
/// @layout.field owner=View index=0 name=user offset=0 size=8 align=8
"#,
    );
}

#[test]
fn test_preserve_copy_through_lifetime_applied_nominal_fields() {
    let session = TestSession::single(
        r#"
struct User {
    id: int32;
}

struct View<comptime L: Lifetime> {
    user: Borrowed<User, L, "readonly">;
}

struct Holder<comptime L: Lifetime> {
    view: View<L>;
}

function retain<comptime L: Lifetime>(value: Holder<L>): Holder<L> {
    return value;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type User {
    id: int32;
}

@copy
type View<L0: lifetime> {
    user: ref<User, borrowed, lifetime(L0), readonly>;
}

@copy
type Holder<L0: lifetime> {
    view: View<lifetime(L0)>;
}

function main.retain<L0: lifetime>(v0: Holder<lifetime(L0)>): Holder<lifetime(L0)> {
entry(v0: Holder<lifetime(L0)>):
    return v0
}
/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
/// @layout.struct name=View size=8 align=8
/// @layout.field owner=View index=0 name=user offset=0 size=8 align=8
/// @layout.struct name=Holder size=8 align=8
/// @layout.field owner=Holder index=0 name=view offset=0 size=8 align=8
"#,
    );
}

#[test]
fn test_lower_shares_one_function_body_across_lifetime_instantiations() {
    let session = TestSession::single(
        r#"
class User {
    id: int32;
}

function inspect<T>(marker: T, value: &readonly User): int32 {
    return value.id;
}

function inspectBorrowed(marker: int32, value: &readonly User): int32 {
    return inspect(marker, value);
}

function inspectManaged(marker: int32, value: User): int32 {
    return inspect(marker, value);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type User {
    id: int32;
}

function main.inspectBorrowed<L0: lifetime>(v0: int32, v1: ref<User, borrowed, lifetime(L0), readonly>): int32 {
entry(v0: int32, v1: ref<User, borrowed, lifetime(L0), readonly>):
    v2: int32 = call main.inspect(v0, v1)
    return v2
}

function main.inspectManaged(v0: int32, v1: ref<User, managed, mutable>): int32 {
entry(v0: int32, v1: ref<User, managed, mutable>):
    v2: ref<User, borrowed, readonly> = cast.bit v1 -> ref<User, borrowed, readonly>
    v3: int32 = call main.inspect(v0, v2)
    return v3
}

function main.inspect<L0: lifetime>(v0: int32, v1: ref<User, borrowed, lifetime(L0), readonly>): int32 {
entry(v0: int32, v1: ref<User, borrowed, lifetime(L0), readonly>):
    v2: ref<int32, borrowed, readonly> = field.address v1, 0
    v3: int32 = load v2
    return v3
}
/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}
