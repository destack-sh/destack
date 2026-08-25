use crate::tests::{DirRows, TestSession};

/// Select an extension static through a sized type literal receiver.
#[test]
fn test_selects_integer_static_on_sized_literal() {
    let session = TestSession::single(
        r#"
const greatest = int32.maximum();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const greatest: int32 = int32.maximum<int32>();

=== dir ===
const greatest = int32.maximum();
/// @type.symbol symbol=greatest source=greatest type=int32
/// @resolution.pattern source=greatest kind=binding target=greatest
/// @resolution.name source=int32 target=int32 kind=type
/// @resolution.member source=int32.maximum receiver=Type<int32> type=() => int32 kind=symbol target_receiver=Type<int32> adjustments=(newtype.payload(Type, intrinsic)) target=maximum
/// @resolution.call source=int32.maximum() parameters=() return=int32 kind=symbol target=maximum instance=int32.<extension#1>.maximum
/// @generic.instantiation id=Type<int32> template=Type arguments=(int32)
/// @generic.instantiation id=maximum<int32> template=maximum arguments=(int32)
/// @generic.instance id=Type<int32> template=Type arguments=(int32)
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const least: uint8 = uint8.minimum<uint8>();

=== dir ===
const least = uint8.minimum();
/// @type.symbol symbol=least source=least type=uint8
/// @resolution.pattern source=least kind=binding target=least
/// @resolution.name source=uint8 target=uint8 kind=type
/// @resolution.member source=uint8.minimum receiver=Type<uint8> type=() => uint8 kind=symbol target_receiver=Type<uint8> adjustments=(newtype.payload(Type, intrinsic)) target=minimum
/// @resolution.call source=uint8.minimum() parameters=() return=uint8 kind=symbol target=minimum instance=uint8.<extension#1>.minimum
/// @generic.instantiation id=Type<uint8> template=Type arguments=(uint8)
/// @generic.instantiation id=minimum<uint8> template=minimum arguments=(uint8)
/// @generic.instance id=Type<uint8> template=Type arguments=(uint8)
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const greatest: int64 = int.maximum<int64>();

=== dir ===
const greatest = int.maximum();
/// @type.symbol symbol=greatest source=greatest type=int64
/// @resolution.pattern source=greatest kind=binding target=greatest
/// @resolution.name source=int target=int64 kind=type
/// @resolution.member source=int.maximum receiver=Type<int64> type=() => int64 kind=symbol target_receiver=Type<int64> adjustments=(newtype.payload(Type, intrinsic)) target=maximum
/// @resolution.call source=int.maximum() parameters=() return=int64 kind=symbol target=maximum instance=int64.<extension#1>.maximum
/// @generic.instantiation id=Type<int64> template=Type arguments=(int64)
/// @generic.instantiation id=maximum<int64> template=maximum arguments=(int64)
/// @generic.instance id=Type<int64> template=Type arguments=(int64)
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
        "main.ds",
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
/// @resolution.member source=float.length receiver="measure" type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=float placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=float root=float
/// @generic.instantiation id="length<\"local\">" template=length arguments=("local")
/// @generic.instance id="length<\"local\">" template=length arguments=("local")
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
        "main.ds",
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
/// @diagnostic.error id=missing-member message="member 'nonsense' does not exist on type 'Type<int32>'"
/// @diagnostic.label line=2 column=21 span="nonsense" line_source="const value = int32.nonsense();"
"#,
    );
}

/// Bind a bare contextual type literal name to its Type<T> value.
#[test]
fn test_binds_bare_type_literal_name_to_reflected_value() {
    let session = TestSession::single(
        r#"
const meta = int;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const meta: Type<int64> = int;

=== dir ===
const meta = int;
/// @type.symbol symbol=meta source=meta type=Type<int64>
/// @resolution.pattern source=meta kind=binding target=meta
/// @generic.instance id=Type<int64> template=Type arguments=(int64)
/// @resolution.name source=int target=int64 kind=type
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
        "main.ds",
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

/// Select an extension static through a type-valued binding.
#[test]
fn test_selects_integer_static_through_type_valued_binding() {
    let session = TestSession::single(
        r#"
const meta = int32;
const greatest = meta.maximum();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const meta: Type<int32> = type int32;
const greatest: int32 = meta.maximum<int32>();

=== dir ===
const meta = int32;
/// @type.symbol symbol=meta source=meta type=Type<int32>
/// @resolution.pattern source=meta kind=binding target=meta
/// @generic.instance id=Type<int32> template=Type arguments=(int32)

const greatest = meta.maximum();
/// @type.symbol symbol=greatest source=greatest type=int32
/// @resolution.pattern source=greatest kind=binding target=greatest
/// @resolution.name source=meta target=meta
/// @resolution.member source=meta.maximum receiver=Type<int32> type=() => int32 kind=symbol target_receiver=Type<int32> adjustments=(newtype.payload(Type, intrinsic)) target=maximum
/// @resolution.call source=meta.maximum() parameters=() return=int32 kind=symbol target=maximum instance=int32.<extension#1>.maximum
/// @resolution.place source=meta placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=meta root=meta
/// @generic.instantiation id=Type<int32> template=Type arguments=(int32)
/// @generic.instantiation id=maximum<int32> template=maximum arguments=(int32)
/// @generic.instance id=maximum<int32> template=maximum arguments=(int32)
"#,
    );
}
