use crate::tests::TestSession;

#[test]
fn test_lower_lifetime_parameters_to_polymorphic_slots() {
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

function test.main.identity<'a>(v0: ref<User, borrowed, 'a, readonly>): ref<User, borrowed, 'a, readonly> {
entry(v0: ref<User, borrowed, 'a, readonly>):
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

function identity<'a>(
    value: Borrowed<User, 'a, "readonly">,
): Borrowed<User, 'a, "readonly"> {
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

function test.main.identity<'a>(v0: ref<User, borrowed, 'a, readonly>): ref<User, borrowed, 'a, readonly> {
entry(v0: ref<User, borrowed, 'a, readonly>):
    return v0
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_lifetime_unions_to_combined_provenance() {
    let session = TestSession::single(
        r#"
struct User {
    id: int32;
}

function identity<'a, 'b>(
    value: Borrowed<User, 'a | 'b, "readonly">,
): Borrowed<User, 'a | 'b, "readonly"> {
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

function test.main.identity<'a, 'b>(v0: ref<User, borrowed, 'a | 'b, readonly>): ref<User, borrowed, 'a | 'b, readonly> {
entry(v0: ref<User, borrowed, 'a | 'b, readonly>):
    return v0
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_share_one_polymorphic_nominal_across_lifetime_applications() {
    let session = TestSession::single(
        r#"
struct User {
    id: int32;
}

struct View<'a> {
    user: Borrowed<User, 'a, "readonly">;
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
type View<'a> {
    user: ref<User, borrowed, 'a, readonly>;
}

function test.main.retain<'a>(v0: View<'a>): View<'a> {
entry(v0: View<'a>):
    return v0
}

function test.main.get<'a>(v0: View<'a>): ref<User, borrowed, 'a, readonly> {
entry(v0: View<'a>):
    v1: ref<User, borrowed, 'a, readonly> = field.get v0, 0
    return v1
}

function test.main.retainStatic(v0: View<'static>): View<'static> {
entry(v0: View<'static>):
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

struct View<'a> {
    user: Borrowed<User, 'a, "readonly">;
}

struct Holder<'a> {
    view: View<'a>;
}

function retain<'a>(value: Holder<'a>): Holder<'a> {
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
type View<'a> {
    user: ref<User, borrowed, 'a, readonly>;
}

@copy
type Holder<'a> {
    view: View<'a>;
}

function test.main.retain<'a>(v0: Holder<'a>): Holder<'a> {
entry(v0: Holder<'a>):
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
fn test_share_one_function_body_across_lifetime_instantiations() {
    let session = TestSession::single(
        r#"
class User {
    id: int32 = 0;
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

function test.main.User.constructor(v0: ref<uninit<User>, borrowed, exclusive>): void {
entry(v0: ref<uninit<User>, borrowed, exclusive>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive> = field.address v0, 0
    store v2, v1
    return
}

function test.main.inspectBorrowed<'a>(v0: int32, v1: ref<User, borrowed, 'a, readonly>): int32 {
entry(v0: int32, v1: ref<User, borrowed, 'a, readonly>):
    v2: int32 = call test.main.inspect<int32>(v0, v1): <'a>(int32, ref<User, borrowed, 'a, readonly>) => int32
    return v2
}

function test.main.inspectManaged(v0: int32, v1: ref<User, managed, mutable>): int32 {
entry(v0: int32, v1: ref<User, managed, mutable>):
    v2: ref<User, borrowed, 'frame, readonly> = cast.bit v1 -> ref<User, borrowed, 'frame, readonly>
    v3: int32 = call test.main.inspect<int32>(v0, v2): <'a>(int32, ref<User, borrowed, 'a, readonly>) => int32
    return v3
}

function test.main.inspect<int32, 'a>(v0: int32, v1: ref<User, borrowed, 'a, readonly>): int32 {
entry(v0: int32, v1: ref<User, borrowed, 'a, readonly>):
    v2: ref<int32, borrowed, readonly> = field.address v1, 0
    v3: int32 = load v2
    return v3
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_const_lifetime_unions_to_combined_provenance() {
    let session = TestSession::single(
        r#"
struct User {
    id: int32;
}

function identity<'a, 'b>(
    value: Borrowed<User, 'a | 'b, "readonly">,
): Borrowed<User, 'a | 'b, "readonly"> {
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

function test.main.identity<'a, 'b>(v0: ref<User, borrowed, 'a | 'b, readonly>): ref<User, borrowed, 'a | 'b, readonly> {
entry(v0: ref<User, borrowed, 'a | 'b, readonly>):
    return v0
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}
