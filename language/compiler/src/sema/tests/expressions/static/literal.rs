use crate::tests::{DirRows, TestSession};

/// Type arguments do not turn a struct declaration into a value.
#[test]
fn test_reference_specialized_struct_as_value() {
    let session = TestSession::single(
        r#"
struct Box<out T> {}

const value = Box<int32>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Box<out T> {}

const value = Box<int32>;

=== dir ===
struct Box<out T> {}
/// @generic.template symbol=Box parameters=(out T)
/// @type.symbol symbol=Box source="struct Box<out T> {}" type=Box
/// @definition.struct symbol=Box source="struct Box<out T> {}" template=(out T)
/// @type.symbol symbol=Box.T source="out T" type=T

const value = Box<int32>;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box<int32> target=Box
"#,
        r#"
/// @diagnostic.error id=invalid-value-reference message="'Box' is not a value"
/// @diagnostic.label line=4 column=15 span="Box<int32>" line_source="const value = Box<int32>;"
/// @diagnostic.help message="construct structs with 'T { … }'"
"#,
    );
}

/// Select an extension static through a sized type literal receiver.
#[test]
fn test_selects_integer_static_on_sized_literal() {
    let session = TestSession::single(
        r#"
const greatest = int32.maximum();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const greatest: int32 = int32.maximum<int32>();

=== dir ===
const greatest = int32.maximum();
/// @type.symbol symbol=greatest source=greatest type=int32
/// @resolution.pattern source=greatest kind=binding target=greatest
/// @resolution.name source=int32 target=int32 kind=type
/// @resolution.member source=int32.maximum receiver=int32 type=() => int32 kind=symbol target_receiver=int32 target=maximum
/// @resolution.call source=int32.maximum() parameters=() return=int32 kind=symbol target=maximum instance=int32.<extension#1>.maximum
/// @generic.instantiation id=maximum<int32> template=maximum arguments=(int32)
/// @generic.instance id=maximum<int32> template=maximum arguments=(int32)
"#,
    );
}

/// Select an extension static through an unsigned type literal receiver.
#[test]
fn test_selects_integer_static_on_unsigned_literal() {
    let session = TestSession::single(
        r#"
const least = uint8.minimum();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const least: uint8 = uint8.minimum<uint8>();

=== dir ===
const least = uint8.minimum();
/// @type.symbol symbol=least source=least type=uint8
/// @resolution.pattern source=least kind=binding target=least
/// @resolution.name source=uint8 target=uint8 kind=type
/// @resolution.member source=uint8.minimum receiver=uint8 type=() => uint8 kind=symbol target_receiver=uint8 target=minimum
/// @resolution.call source=uint8.minimum() parameters=() return=uint8 kind=symbol target=minimum instance=uint8.<extension#1>.minimum
/// @generic.instantiation id=minimum<uint8> template=minimum arguments=(uint8)
/// @generic.instance id=minimum<uint8> template=minimum arguments=(uint8)
"#,
    );
}

/// Select an extension static through a widthless alias literal receiver.
#[test]
fn test_selects_integer_static_on_alias_literal() {
    let session = TestSession::single(
        r#"
const greatest = int.maximum();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const greatest: int64 = int.maximum<int64>();

=== dir ===
const greatest = int.maximum();
/// @type.symbol symbol=greatest source=greatest type=int64
/// @resolution.pattern source=greatest kind=binding target=greatest
/// @resolution.name source=int target=int64 kind=type
/// @resolution.member source=int.maximum receiver=int64 type=() => int64 kind=symbol target_receiver=int64 target=maximum
/// @resolution.call source=int.maximum() parameters=() return=int64 kind=symbol target=maximum instance=int64.<extension#1>.maximum
/// @generic.instantiation id=maximum<int64> template=maximum arguments=(int64)
/// @generic.instance id=maximum<int64> template=maximum arguments=(int64)
"#,
    );
}

/// Prefer a lexical binding over the type literal of the same name.
#[test]
fn test_prefers_lexical_binding_over_type_literal_name() {
    let session = TestSession::single(
        r#"
const float = "measure";
const size = float.length;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const float: "measure" = "measure";
const size: isize = float.length;

=== dir ===
const float = "measure";
/// @type.symbol symbol=float source=float type="measure"
/// @resolution.pattern source=float kind=binding target=float

const size = float.length;
/// @type.symbol symbol=size source=size type=isize
/// @resolution.pattern source=size kind=binding target=size
/// @resolution.name source=float target=float
/// @resolution.member source=float.length receiver="measure" type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=float placement="local" lifetime="static" access="immutable"
/// @resolution.access source=float root=float
/// @generic.instantiation id="length<\"managed\" & \"local\">" template=length arguments=("managed" & "local")
/// @generic.instance id="length<\"bound0\" & \"local\">" template=length arguments=("bound0" & "local")
"#,
    );
}

/// Reject a missing static on a type literal receiver as a member error.
#[test]
fn test_rejects_missing_static_on_type_literal() {
    let session = TestSession::single(
        r#"
const value = int32.nonsense();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const value = int32.nonsense();

=== dir ===
const value = int32.nonsense();
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=int32 target=int32 kind=type
/// @resolution.rejected source=int32.nonsense
/// @resolution.rejected source=int32.nonsense()
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'nonsense' does not exist on type 'int32'"
/// @diagnostic.label line=2 column=21 span="nonsense" line_source="const value = int32.nonsense();"
"#,
    );
}

/// A primitive type name does not produce a reflection value.
#[test]
fn test_reference_primitive_types_as_values() {
    let session = TestSession::single(
        r#"
const meta = int;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const meta = int;

=== dir ===
const meta = int;
/// @type.symbol symbol=meta source=meta type=<error>
/// @resolution.pattern source=meta kind=binding target=meta
/// @resolution.name source=int target=int64 kind=type
"#,
        r#"
/// @diagnostic.error id=invalid-value-reference message="'int64' is not a value"
/// @diagnostic.label line=2 column=14 span="int" line_source="const meta = int;"
"#,
    );
}

/// Reject an unresolved path whose tail names a type literal.
#[test]
fn test_rejects_unresolved_path_with_type_literal_tail() {
    let session = TestSession::single(
        r#"
const value = missing.int32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const value = missing.int32;

=== dir ===
const value = missing.int32;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.unresolved source=missing path=missing
/// @resolution.poisoned source=missing.int32
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'missing'"
/// @diagnostic.label line=2 column=15 span="missing" line_source="const value = missing.int32;"
"#,
    );
}

/// A primitive type name cannot initialize a receiver for associated members.
#[test]
fn test_reference_associated_members_through_invalid_values() {
    let session = TestSession::single(
        r#"
const meta = int32;

const greatest = meta.maximum();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const meta = int32;

const greatest = meta.maximum();

=== dir ===
const meta = int32;
/// @type.symbol symbol=meta source=meta type=<error>
/// @resolution.pattern source=meta kind=binding target=meta
/// @resolution.name source=int32 target=int32 kind=type

const greatest = meta.maximum();
/// @type.symbol symbol=greatest source=greatest type=<error>
/// @resolution.pattern source=greatest kind=binding target=greatest
/// @resolution.name source=meta target=meta
/// @resolution.place source=meta placement="local" lifetime="static" access="immutable"
/// @resolution.access source=meta root=meta
/// @resolution.poisoned source=meta.maximum
/// @resolution.rejected source=meta.maximum()
"#,
        r#"
/// @diagnostic.error id=invalid-value-reference message="'int32' is not a value"
/// @diagnostic.label line=2 column=14 span="int32" line_source="const meta = int32;"
"#,
    );
}
