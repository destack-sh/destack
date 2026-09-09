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

    session.assert_mir_function("main.ds", "test.main.identity", r#"
@copy
type test.main.User {
    id: int32;
}

function test.main.identity<'a>(v0: ref<test.main.User, borrowed, 'a, readonly, local>): ref<test.main.User, borrowed, 'a, readonly, local> {
    local l0: ref<test.main.User, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.User, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.User, borrowed, 'a, readonly, local> = local.get l0
    return v1
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);
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

    session.assert_mir_function("main.ds", "test.main.identity", r#"
@copy
type test.main.User {
    id: int32;
}

function test.main.identity<'a>(v0: ref<test.main.User, borrowed, 'a, readonly, local>): ref<test.main.User, borrowed, 'a, readonly, local> {
    local l0: ref<test.main.User, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.User, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.User, borrowed, 'a, readonly, local> = local.get l0
    return v1
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);
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

    session.assert_mir_function("main.ds", "test.main.identity", r#"
@copy
type test.main.User {
    id: int32;
}

function test.main.identity<'a, 'b>(v0: ref<test.main.User, borrowed, 'a | 'b, readonly, local>): ref<test.main.User, borrowed, 'a | 'b, readonly, local> {
    local l0: ref<test.main.User, borrowed, 'a | 'b, readonly, local>

entry(v0: ref<test.main.User, borrowed, 'a | 'b, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.User, borrowed, 'a | 'b, readonly, local> = local.get l0
    return v1
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);
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

    session.assert_mir_function(
        "main.ds",
        "test.main.retain",
        r#"
@copy
type test.main.View<'a> {
    user: ref<test.main.User, borrowed, 'a, readonly>;
}

function test.main.retain<'a>(v0: test.main.View<'a & local>): test.main.View<'a & local> {
    local l0: test.main.View<'a & local>

entry(v0: test.main.View<'a & local>):
    local.set l0, v0
    v1: test.main.View<'a & local> = local.get l0
    return v1
}

/// @layout.struct name=test.main.View<'a & local> size=8 align=8
/// @layout.field owner=test.main.View<'a & local> index=0 name=user offset=0 size=8 align=8
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.get",
        r#"
@copy
type test.main.User {
    id: int32;
}

@copy
type test.main.View<'a> {
    user: ref<test.main.User, borrowed, 'a, readonly>;
}

function test.main.get<'a>(v0: test.main.View<'a & local>): ref<test.main.User, borrowed, 'a, readonly, local> {
    local l0: test.main.View<'a & local>

entry(v0: test.main.View<'a & local>):
    local.set l0, v0
    v1: test.main.View<'a & local> = local.get l0
    v2: ref<test.main.User, borrowed, 'a, readonly, local> = field.get v1, 0
    return v2
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
/// @layout.struct name=test.main.View<'a & local> size=8 align=8
/// @layout.field owner=test.main.View<'a & local> index=0 name=user offset=0 size=8 align=8
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.retainStatic",
        r#"
@copy
type test.main.View<'a> {
    user: ref<test.main.User, borrowed, 'a, readonly>;
}

function test.main.retainStatic(v0: test.main.View<'static & local>): test.main.View<'static & local> {
    local l0: test.main.View<'static & local>

entry(v0: test.main.View<'static & local>):
    local.set l0, v0
    v1: test.main.View<'static & local> = local.get l0
    return v1
}

/// @layout.struct name=test.main.View<'static & local> size=8 align=8
/// @layout.field owner=test.main.View<'static & local> index=0 name=user offset=0 size=8 align=8
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

    session.assert_mir_function(
        "main.ds",
        "test.main.retain",
        r#"
@copy
type test.main.Holder<'a> {
    view: test.main.View<'a>;
}

function test.main.retain<'a>(v0: test.main.Holder<'a & local>): test.main.Holder<'a & local> {
    local l0: test.main.Holder<'a & local>

entry(v0: test.main.Holder<'a & local>):
    local.set l0, v0
    v1: test.main.Holder<'a & local> = local.get l0
    return v1
}

/// @layout.struct name=test.main.Holder<'a & local> size=8 align=8
/// @layout.field owner=test.main.Holder<'a & local> index=0 name=view offset=0 size=8 align=8
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

    session.assert_mir_function(
        "main.ds",
        "test.main.User.constructor",
        r#"
type test.main.User {
    id: int32;
}

function test.main.User.constructor<'a>(v0: ref<uninit<test.main.User>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<test.main.User>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.User>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<test.main.User>, borrowed, 'a, mutable, local> = local.get l0
    v2: int32 = 0
    v3: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v1, 0
    store v3, v2
    return
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.inspectBorrowed",
        r#"
type test.main.User {
    id: int32;
}

function test.main.inspectBorrowed<'a>(v0: int32, v1: ref<test.main.User, borrowed, 'a, readonly, local>): int32 {
    local l0: int32
    local l1: ref<test.main.User, borrowed, 'a, readonly, local>

entry(v0: int32, v1: ref<test.main.User, borrowed, 'a, readonly, local>):
    local.set l0, v0
    local.set l1, v1
    v2: int32 = local.get l0
    v3: ref<test.main.User, borrowed, 'a, readonly, local> = local.get l1
    v4: int32 = call test.main.inspect<int32>(v2, v3): <'a>(int32, ref<test.main.User, borrowed, 'a, readonly, local>) => int32
    return v4
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.inspectManaged", r#"
type test.main.User {
    id: int32;
}

function test.main.inspectManaged(v0: int32, v1: ref<test.main.User, managed, mutable, local>): int32 {
    local l0: int32
    local l1: ref<test.main.User, managed, mutable, local>

entry(v0: int32, v1: ref<test.main.User, managed, mutable, local>):
    local.set l0, v0
    local.set l1, v1
    v2: int32 = local.get l0
    v3: ref<test.main.User, managed, mutable, local> = local.get l1
    v4: ref<test.main.User, borrowed, 'managed, readonly, local> = cast.bit v3 -> ref<test.main.User, borrowed, 'managed, readonly, local>
    v5: int32 = call test.main.inspect<int32>(v2, v4): <'a>(int32, ref<test.main.User, borrowed, 'a, readonly, local>) => int32
    return v5
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.inspect<int32>", r#"
type test.main.User {
    id: int32;
}

shared function test.main.inspect<int32, 'a>(v0: int32, v1: ref<test.main.User, borrowed, 'a, readonly, local>): int32;

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);
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

    session.assert_mir_function("main.ds", "test.main.identity", r#"
@copy
type test.main.User {
    id: int32;
}

function test.main.identity<'a, 'b>(v0: ref<test.main.User, borrowed, 'a | 'b, readonly, local>): ref<test.main.User, borrowed, 'a | 'b, readonly, local> {
    local l0: ref<test.main.User, borrowed, 'a | 'b, readonly, local>

entry(v0: ref<test.main.User, borrowed, 'a | 'b, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.User, borrowed, 'a | 'b, readonly, local> = local.get l0
    return v1
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);
}

#[test]
fn test_lower_an_elided_wrapper_region_from_a_borrowed_parameter() {
    let session = TestSession::single(
        r#"
import { MaybeOwned } from "destack:memory";

function wrap(text: &readonly string): MaybeOwned<string> {
    MaybeOwned.borrowed(text)
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.wrap", r#"
@languageItem("string.String")
type String;

@copy
@languageItem("memory.Cow")
type Cow<'a, T>;

function test.main.wrap<'a>(v0: ref<String, borrowed, 'a, readonly, local>): Cow<'a & local, ref<String, managed, mutable, local>> {
    local l0: ref<String, borrowed, 'a, readonly, local>

entry(v0: ref<String, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<String, borrowed, 'a, readonly, local> = local.get l0
    v2: Cow<'a & local, ref<String, managed, mutable, local>> = call Cow.borrowed<ref<String, managed, mutable, local>>(v1): <'a>(ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly, local>) => Cow<'a & local, ref<String, managed, mutable, local>>
    return v2
}
"#);
}

#[test]
fn test_lower_an_elided_wrapper_region_from_an_implicit_receiver() {
    let session = TestSession::single(
        r#"
import { Error } from "destack:error";
import { MaybeOwned } from "destack:memory";

enum Kind {
    Syntax,
    Depth,
}

export struct ParseError {
    kind: Kind;
    message: string;
    offset?: usize;
}

export extension of ParseError implements Error {
    display(): MaybeOwned<string> {
        MaybeOwned.borrowed(this.message)
    }
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.ParseError.Error.display",
        r#"
@languageItem("string.String")
type String;

@copy
type test.main.ParseError {
    kind: test.main.Kind;
    message: ref<String, managed, mutable, local>;
    offset: variant<uint1> { 0uint1 = void; 1uint1 = usize; };
}

@copy
@languageItem("memory.Cow")
type Cow<'a, T>;

function test.main.ParseError.Error.display<'a>(v0: ref<test.main.ParseError, borrowed, 'a, readonly, local>): Cow<'a & local, ref<String, managed, mutable, local>> {
    local l0: ref<test.main.ParseError, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.ParseError, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.ParseError, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly, local> = field.project v1, 1
    v3: ref<String, managed, readonly, local> = load v2
    v4: ref<String, borrowed, 'a, readonly, local> = cast.bit v3 -> ref<String, borrowed, 'a, readonly, local>
    v5: Cow<'a & local, ref<String, managed, mutable, local>> = call Cow.borrowed<ref<String, managed, mutable, local>>(v4): <'a>(ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly, local>) => Cow<'a & local, ref<String, managed, mutable, local>>
    return v5
}

/// @layout.struct name=test.main.ParseError size=32 align=8
/// @layout.field owner=test.main.ParseError index=0 name=kind offset=24 size=1 align=1
/// @layout.field owner=test.main.ParseError index=1 name=message offset=0 size=8 align=8
/// @layout.field owner=test.main.ParseError index=2 name=offset offset=8 size=16 align=8
"#,
    );
    session.assert_mir_function("main.ds", "test.main.ParseError.Error.display", r#"
@languageItem("string.String")
type String;

@copy
type test.main.ParseError {
    kind: test.main.Kind;
    message: ref<String, managed, mutable, local>;
    offset: variant<uint1> { 0uint1 = void; 1uint1 = usize; };
}

@copy
@languageItem("memory.Cow")
type Cow<'a, T>;

function test.main.ParseError.Error.display<'a>(v0: ref<test.main.ParseError, borrowed, 'a, readonly, local>): Cow<'a & local, ref<String, managed, mutable, local>> {
    local l0: ref<test.main.ParseError, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.ParseError, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.ParseError, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly, local> = field.project v1, 1
    v3: ref<String, managed, readonly, local> = load v2
    v4: ref<String, borrowed, 'a, readonly, local> = cast.bit v3 -> ref<String, borrowed, 'a, readonly, local>
    v5: Cow<'a & local, ref<String, managed, mutable, local>> = call Cow.borrowed<ref<String, managed, mutable, local>>(v4): <'a>(ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly, local>) => Cow<'a & local, ref<String, managed, mutable, local>>
    return v5
}

/// @layout.struct name=test.main.ParseError size=32 align=8
/// @layout.field owner=test.main.ParseError index=0 name=kind offset=24 size=1 align=1
/// @layout.field owner=test.main.ParseError index=1 name=message offset=0 size=8 align=8
/// @layout.field owner=test.main.ParseError index=2 name=offset offset=8 size=16 align=8
"#);
}

#[test]
fn test_instantiate_a_constructor_at_the_regions_its_construction_binds() {
    let session = TestSession::single(
        r#"
class Label {
    text: string;

    constructor(text: &readonly string) {
        this.text = text.slice();
    }
}

class Holder<'a> {
    value: &'a readonly int32;

    constructor(value: &'a readonly int32) {
        this.value = value;
    }

    get(&readonly this): &'a readonly int32 {
        this.value
    }
}

function label(text: &readonly string): Label {
    return new Label(text);
}

function hold<'a>(value: &'a readonly int32): Holder<'a> {
    return new Holder(value);
}

function read<'a>(holder: &readonly Holder<'a>): &'a readonly int32 {
    return holder.get();
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.label", r#"
@languageItem("string.String")
type String;

type test.main.Label {
    text: ref<String, managed, mutable, local>;
}

function test.main.label<'a>(v0: ref<String, borrowed, 'a, readonly, local>): ref<test.main.Label, managed, mutable, local> {
    local l0: ref<String, borrowed, 'a, readonly, local>

entry(v0: ref<String, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<String, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<test.main.Label, managed, mutable, local> = new.zeroed test.main.Label
    v3: ref<uninit<test.main.Label>, borrowed, 'managed, mutable, local> = cast.bit v2 -> ref<uninit<test.main.Label>, borrowed, 'managed, mutable, local>
    call test.main.Label.constructor(v3, v1): <'a, 'b>(ref<uninit<test.main.Label>, borrowed, 'b, mutable, local>, ref<String, borrowed, 'a, readonly, local>) => void
    return v2
}

/// @layout.struct name=test.main.Label size=8 align=8
/// @layout.field owner=test.main.Label index=0 name=text offset=0 size=8 align=8
"#);
    session.assert_mir_function("main.ds", "test.main.hold", r#"
type test.main.Holder<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test.main.hold<'a>(v0: ref<int32, borrowed, 'a, readonly, local>): ref<test.main.Holder<'a & local>, managed, mutable, local> {
    local l0: ref<int32, borrowed, 'a, readonly, local>

entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<int32, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<test.main.Holder<'a & local>, managed, mutable, local> = new.zeroed test.main.Holder<'a & local>
    v3: ref<uninit<test.main.Holder<'a & local>>, borrowed, 'managed, mutable, local> = cast.bit v2 -> ref<uninit<test.main.Holder<'a & local>>, borrowed, 'managed, mutable, local>
    call test.main.Holder.constructor(v3, v1): <'a, 'b>(ref<uninit<test.main.Holder<'a & local>>, borrowed, 'b, mutable, local>, ref<int32, borrowed, 'a, readonly, local>) => void
    return v2
}

/// @layout.struct name=test.main.Holder<'a & local> size=8 align=8
/// @layout.field owner=test.main.Holder<'a & local> index=0 name=value offset=0 size=8 align=8
"#);
    session.assert_mir_function("main.ds", "test.main.read", r#"
type test.main.Holder<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test.main.read<'a, 'b>(v0: ref<test.main.Holder<'a & local>, borrowed, 'b, readonly, local>): ref<int32, borrowed, 'a, readonly, local> {
    local l0: ref<test.main.Holder<'a & local>, borrowed, 'b, readonly, local>

entry(v0: ref<test.main.Holder<'a & local>, borrowed, 'b, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Holder<'a & local>, borrowed, 'b, readonly, local> = local.get l0
    v2: ref<int32, borrowed, 'a, readonly, local> = call test.main.Holder.get(v1): <'a, 'b>(ref<test.main.Holder<'a & local>, borrowed, 'b, readonly, local>) => ref<int32, borrowed, 'a, readonly, local>
    return v2
}

/// @layout.struct name=test.main.Holder<'a & local> size=8 align=8
/// @layout.field owner=test.main.Holder<'a & local> index=0 name=value offset=0 size=8 align=8
"#);
}
