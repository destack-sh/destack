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

    session.assert_mir_function("main.tspp", "test.main.identity", r#"
type test.main.User {
    id: int32;
}

function test.main.identity<'a>(v0: ref<test.main.User, borrowed, 'a, readonly>): ref<test.main.User, borrowed, 'a, readonly> {
    local l0: ref<test.main.User, borrowed, 'a, readonly>

entry(v0: ref<test.main.User, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.User, borrowed, 'a, readonly> = load l0
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

    session.assert_mir_function("main.tspp", "test.main.identity", r#"
type test.main.User {
    id: int32;
}

function test.main.identity<'a>(v0: ref<test.main.User, borrowed, 'a, readonly>): ref<test.main.User, borrowed, 'a, readonly> {
    local l0: ref<test.main.User, borrowed, 'a, readonly>

entry(v0: ref<test.main.User, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.User, borrowed, 'a, readonly> = load l0
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

    session.assert_mir_function("main.tspp", "test.main.identity", r#"
type test.main.User {
    id: int32;
}

function test.main.identity<'a, 'b>(v0: ref<test.main.User, borrowed, 'a | 'b, readonly>): ref<test.main.User, borrowed, 'a | 'b, readonly> {
    local l0: ref<test.main.User, borrowed, 'a | 'b, readonly>

entry(v0: ref<test.main.User, borrowed, 'a | 'b, readonly>):
    store l0, v0
    v1: ref<test.main.User, borrowed, 'a | 'b, readonly> = load l0
    return v1
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);
}

/// Lower the lifetimes and places of joined regions.
#[test]
fn test_lower_joined_regions_to_lifetimes_and_places() {
    let session = TestSession::single(
        r#"
function identity<'a, 'b>(
    value: Borrowed<int32, 'a | 'b, "readonly">,
): Borrowed<int32, 'a | 'b, "readonly"> {
    return value;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.identity", r#"
function test.main.identity<'a, 'b>(v0: ref<int32, borrowed, 'a | 'b, readonly>): ref<int32, borrowed, 'a | 'b, readonly> {
    local l0: ref<int32, borrowed, 'a | 'b, readonly>

entry(v0: ref<int32, borrowed, 'a | 'b, readonly>):
    store l0, v0
    v1: ref<int32, borrowed, 'a | 'b, readonly> = load l0
    return v1
}
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
        "main.tspp",
        "test.main.retain",
        r#"
type test.main.View<'a> {
    user: ref<test.main.User, borrowed, 'a, readonly>;
}

function test.main.retain<'a>(v0: test.main.View<'a>): test.main.View<'a> {
    local l0: test.main.View<'a>

entry(v0: test.main.View<'a>):
    store l0, v0
    v1: test.main.View<'a> = load l0
    return v1
}

/// @layout.struct name=test.main.View<'a> size=8 align=8
/// @layout.field owner=test.main.View<'a> index=0 name=user offset=0 size=8 align=8
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.get",
        r#"
type test.main.User {
    id: int32;
}

type test.main.View<'a> {
    user: ref<test.main.User, borrowed, 'a, readonly>;
}

function test.main.get<'a>(v0: test.main.View<'a>): ref<test.main.User, borrowed, 'a, readonly> {
    local l0: test.main.View<'a>

entry(v0: test.main.View<'a>):
    store l0, v0
    v1: ref<test.main.User, borrowed, 'a, readonly> = load (l0).0
    return v1
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
/// @layout.struct name=test.main.View<'a> size=8 align=8
/// @layout.field owner=test.main.View<'a> index=0 name=user offset=0 size=8 align=8
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.retainStatic",
        r#"
type test.main.View<'a> {
    user: ref<test.main.User, borrowed, 'a, readonly>;
}

function test.main.retainStatic(v0: test.main.View<'static>): test.main.View<'static> {
    local l0: test.main.View<'static>

entry(v0: test.main.View<'static>):
    store l0, v0
    v1: test.main.View<'static> = load l0
    return v1
}

/// @layout.struct name=test.main.View<'static> size=8 align=8
/// @layout.field owner=test.main.View<'static> index=0 name=user offset=0 size=8 align=8
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
        "main.tspp",
        "test.main.retain",
        r#"
type test.main.Holder<'a> {
    view: test.main.View<'a>;
}

function test.main.retain<'a>(v0: test.main.Holder<'a>): test.main.Holder<'a> {
    local l0: test.main.Holder<'a>

entry(v0: test.main.Holder<'a>):
    store l0, v0
    v1: test.main.Holder<'a> = load l0
    return v1
}

/// @layout.struct name=test.main.Holder<'a> size=8 align=8
/// @layout.field owner=test.main.Holder<'a> index=0 name=view offset=0 size=8 align=8
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
        "main.tspp",
        "test.main.User.constructor",
        r#"
@nocopy
type test.main.User {
    id: int32;
}

constructor test.main.User.constructor<'a>(v0: ref<uninit<test.main.User>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.User>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.User>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.User>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.inspectBorrowed",
        r#"
@nocopy
type test.main.User {
    id: int32;
}

function test.main.inspectBorrowed<'a>(v0: int32, v1: ref<test.main.User, borrowed, 'a, readonly>): int32 {
    local l0: int32
    local l1: ref<test.main.User, borrowed, 'a, readonly>

entry(v0: int32, v1: ref<test.main.User, borrowed, 'a, readonly>):
    store l0, v0
    store l1, v1
    v2: int32 = load l0
    v3: ref<test.main.User, borrowed, 'a, readonly> = load l1
    v4: int32 = call test.main.inspect<int32>(v2, v3): (int32, ref<test.main.User, borrowed, 'a, readonly>) => int32
    return v4
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.inspectManaged", r#"
@nocopy
type test.main.User {
    id: int32;
}

function test.main.inspectManaged(v0: int32, v1: ref<test.main.User, managed, mutable, local>): int32 {
    local l0: int32
    local l1: ref<test.main.User, managed, mutable, local>

entry(v0: int32, v1: ref<test.main.User, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: int32 = load l0
    v3: ref<test.main.User, managed, mutable, local> = load l1
    v4: ref<test.main.User, borrowed, 'managed, readonly> = cast.bit v3 -> ref<test.main.User, borrowed, 'managed, readonly>
    v5: int32 = call test.main.inspect<int32>(v2, v4): (int32, ref<test.main.User, borrowed, 'managed, readonly>) => int32
    return v5
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.inspect<int32>", r#"
@nocopy
type test.main.User {
    id: int32;
}

shared function test.main.inspect<int32, 'a>(v0: int32, v1: ref<test.main.User, borrowed, 'a, readonly>): int32;

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

    session.assert_mir_function("main.tspp", "test.main.identity", r#"
type test.main.User {
    id: int32;
}

function test.main.identity<'a, 'b>(v0: ref<test.main.User, borrowed, 'a | 'b, readonly>): ref<test.main.User, borrowed, 'a | 'b, readonly> {
    local l0: ref<test.main.User, borrowed, 'a | 'b, readonly>

entry(v0: ref<test.main.User, borrowed, 'a | 'b, readonly>):
    store l0, v0
    v1: ref<test.main.User, borrowed, 'a | 'b, readonly> = load l0
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
import { Cow } from "tspp:memory";
import { StringSlice } from "tspp:string";

function wrap(text: &immutable StringSlice): Cow<StringSlice> {
    Cow.borrowed(text)
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.wrap", r#"
@languageItem("string.StringSlice")
type StringSlice;

@languageItem("memory.Cow")
type Cow<'a, T: ToOwned, P0>;

@nocopy
@languageItem("convert.ToOwned")
type ToOwned;

@nocopy
@languageItem("string.String")
type String;

function test.main.wrap<'a>(v0: slice<uint16, borrowed, 'a, immutable>): Cow<'a, StringSlice, String> {
    local l0: slice<uint16, borrowed, 'a, immutable>

entry(v0: slice<uint16, borrowed, 'a, immutable>):
    store l0, v0
    v1: slice<uint16, borrowed, 'a, immutable> = load l0
    v2: Cow<'a, StringSlice, witness<StringSlice, ToOwned, Owned>> = call Cow.borrowed<'a, StringSlice>(v1): (ref<StringSlice, borrowed, 'a, immutable>) => Cow<'a, StringSlice, witness<StringSlice, ToOwned, Owned>>
    return v2
}
"#);
}

#[test]
fn test_lower_an_elided_wrapper_region_from_an_implicit_receiver() {
    let session = TestSession::single(
        r#"
import { Error } from "tspp:error";

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
    display(&immutable this): ^string {
        this.message.clone()
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.ParseError.Error.display",
        r#"
type test.main.ParseError {
    kind: test.main.Kind;
    message: ref<String, managed, mutable, local>;
    offset: variant<uint1> { 0uint1 = usize; 1uint1 = void; };
}

@nocopy
@languageItem("string.String")
type String;

function test.main.ParseError.Error.display<'a>(v0: ref<test.main.ParseError, borrowed, 'a, immutable>): String {
    local l0: ref<test.main.ParseError, borrowed, 'a, immutable>

entry(v0: ref<test.main.ParseError, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.ParseError, borrowed, 'a, immutable> = load l0
    v2: ref<ref<String, managed, mutable, local>, borrowed, 'a, immutable> = address (*v1).1
    v3: ref<String, managed, mutable, local> = load (*v2)
    v4: ref<String, borrowed, 'a, immutable> = cast.bit v3 -> ref<String, borrowed, 'a, immutable>
    v5: String = call String.Clone.clone(v4): (ref<String, borrowed, 'a, immutable>) => String
    return v5
}

/// @layout.struct name=test.main.ParseError size=32 align=8
/// @layout.field owner=test.main.ParseError index=0 name=kind offset=24 size=1 align=1
/// @layout.field owner=test.main.ParseError index=1 name=message offset=0 size=8 align=8
/// @layout.field owner=test.main.ParseError index=2 name=offset offset=8 size=16 align=8
"#,
    );
    session.assert_mir_function("main.tspp", "test.main.ParseError.Error.display", r#"
type test.main.ParseError {
    kind: test.main.Kind;
    message: ref<String, managed, mutable, local>;
    offset: variant<uint1> { 0uint1 = usize; 1uint1 = void; };
}

@nocopy
@languageItem("string.String")
type String;

function test.main.ParseError.Error.display<'a>(v0: ref<test.main.ParseError, borrowed, 'a, immutable>): String {
    local l0: ref<test.main.ParseError, borrowed, 'a, immutable>

entry(v0: ref<test.main.ParseError, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.ParseError, borrowed, 'a, immutable> = load l0
    v2: ref<ref<String, managed, mutable, local>, borrowed, 'a, immutable> = address (*v1).1
    v3: ref<String, managed, mutable, local> = load (*v2)
    v4: ref<String, borrowed, 'a, immutable> = cast.bit v3 -> ref<String, borrowed, 'a, immutable>
    v5: String = call String.Clone.clone(v4): (ref<String, borrowed, 'a, immutable>) => String
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

    session.assert_mir_function("main.tspp", "test.main.label", r#"
@nocopy
type test.main.Label {
    text: ref<String, managed, mutable, local>;
}

@nocopy
@languageItem("string.String")
type String;

function test.main.label<'a>(v0: ref<String, borrowed, 'a, readonly>): ref<test.main.Label, managed, mutable, local> {
    local l0: ref<String, borrowed, 'a, readonly>

entry(v0: ref<String, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<String, borrowed, 'a, readonly> = load l0
    v2: ref<test.main.Label, managed, mutable, local> = new.zeroed test.main.Label, local
    v3: ref<uninit<test.main.Label>, borrowed, 'managed, mutable> = cast.bit v2 -> ref<uninit<test.main.Label>, borrowed, 'managed, mutable>
    call test.main.Label.constructor(v3, v1): <'a_1>(ref<uninit<test.main.Label>, borrowed, 'managed, mutable>, ref<String, borrowed, 'a_1, readonly>) => void
    return v2
}

/// @layout.struct name=test.main.Label size=8 align=8
/// @layout.field owner=test.main.Label index=0 name=text offset=0 size=8 align=8
"#);
    session.assert_mir_function("main.tspp", "test.main.hold", r#"
@nocopy
type test.main.Holder<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test.main.hold<'a>(v0: ref<int32, borrowed, 'a, readonly>): ref<test.main.Holder<'a>, managed, mutable, local> {
    local l0: ref<int32, borrowed, 'a, readonly>

entry(v0: ref<int32, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<int32, borrowed, 'a, readonly> = load l0
    v2: ref<test.main.Holder<'a>, managed, mutable, local> = new.zeroed test.main.Holder<'a>, local
    v3: ref<uninit<test.main.Holder<'a>>, borrowed, 'managed, mutable> = cast.bit v2 -> ref<uninit<test.main.Holder<'a>>, borrowed, 'managed, mutable>
    call test.main.Holder.constructor<'a>(v3, v1): <'l0>(ref<uninit<test.main.Holder<'l0>>, borrowed, 'managed, mutable>, ref<int32, borrowed, 'l0, readonly>) => void
    return v2
}

/// @layout.struct name=test.main.Holder<'a> size=8 align=8
/// @layout.field owner=test.main.Holder<'a> index=0 name=value offset=0 size=8 align=8
"#);
    session.assert_mir_function("main.tspp", "test.main.read", r#"
@nocopy
type test.main.Holder<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test.main.read<'a, 'b>(v0: ref<test.main.Holder<'a>, borrowed, 'b, readonly>): ref<int32, borrowed, 'a, readonly> {
    local l0: ref<test.main.Holder<'a>, borrowed, 'b, readonly>

entry(v0: ref<test.main.Holder<'a>, borrowed, 'b, readonly>):
    store l0, v0
    v1: ref<test.main.Holder<'a>, borrowed, 'b, readonly> = load l0
    v2: ref<int32, borrowed, 'b, readonly> = call test.main.Holder.get<'a>(v1): (ref<test.main.Holder<'b>, borrowed, 'b, readonly>) => ref<int32, borrowed, 'b, readonly>
    return v2
}

/// @layout.struct name=test.main.Holder<'a> size=8 align=8
/// @layout.field owner=test.main.Holder<'a> index=0 name=value offset=0 size=8 align=8
/// @layout.struct name=test.main.Holder<'b> size=8 align=8
/// @layout.field owner=test.main.Holder<'b> index=0 name=value offset=0 size=8 align=8
"#);
}

/// An elided return borrow through a value receiver lowers at the receiver's region.
#[test]
fn test_lower_an_elided_field_borrow_through_a_value_receiver() {
    let session = TestSession::single(
        r#"
struct Pair<T> {
    start: T;
    end: T;
}

newtype Bound<T> =
    | { kind: "included"; value: T }
    | { kind: "unbounded" };

extension<T> of Bound<T> {
    static included(value: T): Bound<T> {
        Bound({ kind: "included", value })
    }
}

extension<T: Copy> of Pair<T> {
    startBound(&immutable this): Bound<&immutable T> {
        Bound.included(&immutable this.start)
    }
}
"#,
    );

    session.assert_mir_lowered("main.tspp", r#"
type test.main.Bound<T> = newtype<variant<uint1> { 0uint1 = ref<{ kind: literal.string.included, value: T }, managed, mutable, local>; 1uint1 = ref<{ kind: literal.string.unbounded }, managed, mutable, local>; }>;

type literal.string.included { }

type literal.string.unbounded { }

type test.main.Pair<T> {
    start: T;
    end: T;
}

@nocopy
@languageItem("memory.Copy")
type Copy extends Clone { }

@nocopy
@languageItem("memory.Clone")
type Clone { }

function test.main.Pair.startBound<T: Copy, 'a>(v0: ref<test.main.Pair<T>, borrowed, 'a, immutable>): test.main.Bound<ref<?T, borrowed, 'a, immutable>> {
    local l0: ref<test.main.Pair<T>, borrowed, 'a, immutable>

entry(v0: ref<test.main.Pair<T>, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Pair<T>, borrowed, 'a, immutable> = load l0
    v2: ref<?T, borrowed, 'a, immutable> = address (*v1).0
    v3: test.main.Bound<ref<?T, borrowed, 'a, immutable>> = call test.main.Bound.included<ref<?T, borrowed, 'a, immutable>>(v2): (ref<?T, borrowed, 'a, immutable>) => test.main.Bound<ref<?T, borrowed, 'a, immutable>>
    return v3
}

function test.main.Bound.included<T>(v0: T): test.main.Bound<T> {
    local l0: T

entry(v0: T):
    store l0, v0
    v1: literal.string.included = zeroed
    v2: T = load l0
    v3: { kind: literal.string.included, value: T } = aggregate (v1, v2)
    v4: ref<{ kind: literal.string.included, value: T }, managed, mutable, local> = new.complete v3
    v5: variant<uint1> { 0uint1 = ref<{ kind: literal.string.included, value: T }, managed, mutable, local>; 1uint1 = ref<{ kind: literal.string.unbounded }, managed, mutable, local>; } = variant.new 0, v4
    v6: test.main.Bound<T> = aggregate (v5)
    return v6
}

/// @dispatch.shape constraint=type@19 function=clone function=cloneFrom
"#);
}

/// An extension bounded by a borrowed lifetime parameter lowers its bound under its own slots.
#[test]
fn test_lower_an_extension_bounded_by_a_borrowed_lifetime_parameter() {
    let session = TestSession::single(
        r#"
interface Iterator<out T> {
    type Return;

    next(&this): T | undefined;
}

struct CopyIterator<I, out T> {
    private iterator: I;
}

extension<T: Copy, 'a, const A: Access, I: Iterator<Borrowed<T, 'a, A>>> of CopyIterator<I, T>
    implements Iterator<T>
{
    type Return = I.Return;

    next(&this): T | undefined {
        const next = this.iterator.next();
        if (next === undefined) {
            return undefined;
        }
        return *next;
    }
}
"#,
    );

    session.assert_mir_lowered("main.tspp", r#"
type test.main.CopyIterator<I, T> {
    iterator: I;
}

@nocopy
@languageItem("memory.Copy")
type Copy extends Clone { }

@nocopy
@languageItem("memory.Clone")
type Clone { }

@nocopy
type test.main.Iterator<T> { }

function test.main.CopyIterator.Iterator.next<T: Copy, 'a, A: Access, I: test.main.Iterator<ref<?T, borrowed, 'a, A>>, 'a>(v0: ref<test.main.CopyIterator<I, T>, borrowed, 'a, mutable>): variant<uint1> { 0uint1 = T; 1uint1 = void; } {
    local l0: ref<test.main.CopyIterator<I, T>, borrowed, 'a, mutable>
    local l1: variant<uint1> { 0uint1 = ref<?T, borrowed, 'a, A>; 1uint1 = void; }

entry(v0: ref<test.main.CopyIterator<I, T>, borrowed, 'a, mutable>):
    store l0, v0
    v1: ref<test.main.CopyIterator<I, T>, borrowed, 'a, mutable> = load l0
    v2: ref<?I, borrowed, 'a, mutable> = address (*v1).0
    v3: variant<uint1> { 0uint1 = ref<?T, borrowed, 'a, A>; 1uint1 = void; } = call.witness I, test.main.Iterator<ref<?T, borrowed, '_, A>>, test.main.Iterator.next(v2): (ref<?I, borrowed, 'a, mutable>) => variant<uint1> { 0uint1 = ref<?T, borrowed, 'a, A>; 1uint1 = void; }
    store l1, v3
    v4: uint1 = variant.tag.load l1
    v5: uint1 = 1
    v6: boolean = eq v4, v5
    branch v6 => b1 | b2

b1:
    v7: void = zeroed
    v8: variant<uint1> { 0uint1 = T; 1uint1 = void; } = variant.new 1
    return v8

b2:
    v9: ref<?T, borrowed, 'a, A> = address (*(l1 as 0))
    v10: ?T = load (*v9)
    v11: T = new.complete v10
    v12: variant<uint1> { 0uint1 = T; 1uint1 = void; } = variant.new 0, v11
    return v12
}

external function test.main.Iterator.next<T, this: test.main.Iterator<T>, 'a>(ref<?this, borrowed, 'a, mutable>): variant<uint1> { 0uint1 = T; 1uint1 = void; }

/// @dispatch.shape constraint=type@9 function=clone function=cloneFrom
/// @dispatch.shape constraint=type@19 function=next
"#);
}

/// A struct method taking this at the struct's own lifetime lowers its receiver instance.
#[test]
fn test_lower_a_struct_method_taking_this_at_the_struct_lifetime() {
    let session = TestSession::single(
        r#"
struct Expectation<'a, T> {
    value: Borrowed<T, 'a, "immutable">;

    isPositive(this: Expectation<'a, int32>): boolean {
        *this.value > 0
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
type test.main.Expectation<'a, T> {
    value: ref<?T, borrowed, 'a, immutable>;
}

function test.main.Expectation.isPositive<'a, T>(v0: test.main.Expectation<'a, int32>): boolean {
    local l0: test.main.Expectation<'a, int32>

entry(v0: test.main.Expectation<'a, int32>):
    store l0, v0
    v1: ref<int32, borrowed, 'a, immutable> = load (l0).0
    v2: int32 = load (*v1)
    v3: int32 = 0
    v4: boolean = gt v2, v3
    return v4
}
"#,
    );
}
