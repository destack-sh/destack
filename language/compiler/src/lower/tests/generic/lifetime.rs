use crate::tests::TestSession;

/// Keep a borrow-polymorphic function out of the lowered module until a caller applies it.
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

function test.main.identity<'a>(v0: ref<User, borrowed, 'a, readonly, local>): ref<User, borrowed, 'a, readonly, local> {
entry(v0: ref<User, borrowed, 'a, readonly, local>):
    return v0
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

/// Lower a written `Borrowed` annotation to a borrowed reference carrying its lifetime.
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

function test.main.identity<'a>(v0: ref<User, borrowed, 'a, readonly, local>): ref<User, borrowed, 'a, readonly, local> {
entry(v0: ref<User, borrowed, 'a, readonly, local>):
    return v0
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

/// Lower a union of lifetime parameters to one reference over their combined provenance.
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

function test.main.identity<'a, 'b>(v0: ref<User, borrowed, 'a | 'b, readonly, local>): ref<User, borrowed, 'a | 'b, readonly, local> {
entry(v0: ref<User, borrowed, 'a | 'b, readonly, local>):
    return v0
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

/// Emit one nominal type for a lifetime-parametric struct used at several applications.
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
    user: ref<User, borrowed, 'a, readonly, local>;
}

function test.main.retain<'a>(v0: View<'a>): View<'a> {
entry(v0: View<'a>):
    return v0
}

function test.main.get<'a>(v0: View<'a>): ref<User, borrowed, 'a, readonly, local> {
entry(v0: View<'a>):
    v1: ref<User, borrowed, 'a, readonly, local> = field.get v0, 0
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

/// Carry the copy attribute through a field typed as a lifetime-applied nominal.
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
    user: ref<User, borrowed, 'a, readonly, local>;
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

/// Emit one function body for a generic callee reached at several lifetime instantiations.
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

function test.main.User.constructor(v0: ref<uninit<User>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<User>, borrowed, exclusive, local>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.inspectBorrowed<'a>(v0: int32, v1: ref<User, borrowed, 'a, readonly, local>): int32 {
entry(v0: int32, v1: ref<User, borrowed, 'a, readonly, local>):
    v2: int32 = call test.main.inspect<int32>(v0, v1): <'a>(int32, ref<User, borrowed, 'a, readonly, local>) => int32
    return v2
}

function test.main.inspectManaged(v0: int32, v1: ref<User, managed, mutable, local>): int32 {
entry(v0: int32, v1: ref<User, managed, mutable, local>):
    v2: ref<User, borrowed, 'frame, readonly, local> = cast.bit v1 -> ref<User, borrowed, 'frame, readonly, local>
    v3: int32 = call test.main.inspect<int32>(v0, v2): <'a>(int32, ref<User, borrowed, 'a, readonly, local>) => int32
    return v3
}

function test.main.inspect<int32, 'a>(v0: int32, v1: ref<User, borrowed, 'a, readonly, local>): int32 {
entry(v0: int32, v1: ref<User, borrowed, 'a, readonly, local>):
    v2: ref<int32, borrowed, readonly, local> = field.address v1, 0
    v3: int32 = load v2
    return v3
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

/// Lower a union of const lifetime parameters to one reference over their combined provenance.
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

function test.main.identity<'a, 'b>(v0: ref<User, borrowed, 'a | 'b, readonly, local>): ref<User, borrowed, 'a | 'b, readonly, local> {
entry(v0: ref<User, borrowed, 'a | 'b, readonly, local>):
    return v0
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}
