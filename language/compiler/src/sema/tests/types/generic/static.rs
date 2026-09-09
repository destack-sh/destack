use crate::tests::{DirRows, TestSession};

/// An associated const may reference its owner's type parameter.
#[test]
fn test_declare_an_associated_const_on_a_generic_struct() {
    let session = TestSession::single(
        r#"
struct Holder<T: Copy> {
    value: T;

    const zero: int32 = 0;
}

const read = Holder<int32>.zero;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Holder<out T: Copy> {
    value: T;

    const zero: int32 = 0;
}

const read: int32 = Holder<int32>.zero;

=== dir ===
struct Holder<T: Copy> {
/// @generic.template symbol=Holder parameters=(out T: Copy)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T: Copy)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @definition.associated.const symbol=Holder.zero source="const zero: int32 = 0" key=zero type=int32
/// @type.symbol symbol=Holder.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

    const zero: int32 = 0;
    /// @type.symbol symbol=Holder.zero source="const zero: int32 = 0" type=int32

}

const read = Holder<int32>.zero;
/// @type.symbol symbol=read source=read type=int32
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=Holder target=Holder
/// @resolution.member source=Holder<int32>.zero receiver=Holder<int32> type=int32 kind=symbol target_receiver=Holder<int32> target=Holder.zero
/// @resolution.function source=Holder<int32> type=Holder<int32> target=Holder instance=Holder<int32>
/// @generic.instantiation id=Holder.zero<int32> template=Holder.zero arguments=(int32)
/// @generic.instantiation id=Holder<int32> template=Holder arguments=(int32)
"#, r#""#);
}

/// A static field may hold its owner's type parameter.
#[test]
fn test_declare_a_static_field_of_the_owner_type_parameter() {
    let session = TestSession::single(
        r#"
struct Registry<T: Copy> {
    value: T;

    static fallback: T | undefined = undefined;
}

const read = Registry<int32>.fallback;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Registry<out T: Copy> {
    value: T;

    static fallback: T | undefined = undefined as T | undefined;
}

const read: int32 | undefined = Registry<int32>.fallback;

=== dir ===
struct Registry<T: Copy> {
/// @generic.template symbol=Registry parameters=(out T: Copy)
/// @type.symbol symbol=Registry type=Registry
/// @definition.struct symbol=Registry template=(out T: Copy)
/// @definition.field symbol=Registry.fallback source="static fallback: T | undefined = undefined" key=fallback static=true type=T | undefined
/// @definition.field symbol=Registry.value source="value: T" key=value type=T
/// @type.symbol symbol=Registry.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Registry.value source="value: T" type=T
    /// @resolution.name source=T target=Registry.T

    static fallback: T | undefined = undefined;
    /// @type.symbol symbol=Registry.fallback source="static fallback: T | undefined = undefined" type=T | undefined
    /// @resolution.name source=T target=Registry.T

}

const read = Registry<int32>.fallback;
/// @type.symbol symbol=read source=read type=int32 | undefined
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=Registry target=Registry
/// @resolution.member source=Registry<int32>.fallback receiver=Registry<int32> type=int32 | undefined kind=field target_receiver=Registry<int32> key=fallback target=Registry.fallback target_type=int32 | undefined
/// @resolution.function source=Registry<int32> type=Registry<int32> target=Registry instance=Registry<int32>
/// @generic.instantiation id=Registry<int32> template=Registry arguments=(int32)
"#, r#""#);
}

/// An associated const rejects a runtime value.
#[test]
fn test_reject_a_runtime_value_in_an_associated_const() {
    let session = TestSession::single(
        r#"
declare function measure(): int32;

struct Meter {
    const width: int32 = measure();
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
declare function measure(): int32;

struct Meter {
    const width: int32 = measure();
}

=== dir ===
declare function measure(): int32;
/// @type.symbol symbol=measure source="declare function measure(): int32" type=() => int32

struct Meter {
/// @type.symbol symbol=Meter type=Meter
/// @definition.struct symbol=Meter
/// @definition.associated.const symbol=Meter.width source="const width: int32 = measure()" key=width type=int32

    const width: int32 = measure();
    /// @type.symbol symbol=Meter.width source="const width: int32 = measure()" type=int32

}
"#, r#"
/// @diagnostic.error id=undecidable-static-value message="static value must be statically decidable"
/// @diagnostic.label line=5 column=26 span="measure()" line_source="const width: int32 = measure();"
"#);
}

/// An associated const may value itself with the owner's type parameter.
#[test]
fn test_declare_an_associated_const_of_the_owner_type_parameter() {
    let session = TestSession::single(
        r#"
struct Wrapper<T: Copy> {
    value: T;

    const fallback: T | undefined = undefined;
}

const read = Wrapper<int32>.fallback;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Wrapper<out T: Copy> {
    value: T;

    const fallback: T | undefined = undefined;
}

const read: int32 | undefined = Wrapper<int32>.fallback;

=== dir ===
struct Wrapper<T: Copy> {
/// @generic.template symbol=Wrapper parameters=(out T: Copy)
/// @type.symbol symbol=Wrapper type=Wrapper
/// @definition.struct symbol=Wrapper template=(out T: Copy)
/// @definition.associated.const symbol=Wrapper.fallback source="const fallback: T | undefined = undefined" key=fallback type=T | undefined
/// @definition.field symbol=Wrapper.value source="value: T" key=value type=T
/// @type.symbol symbol=Wrapper.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Wrapper.value source="value: T" type=T
    /// @resolution.name source=T target=Wrapper.T

    const fallback: T | undefined = undefined;
    /// @type.symbol symbol=Wrapper.fallback source="const fallback: T | undefined = undefined" type=T | undefined
    /// @resolution.name source=T target=Wrapper.T

}

const read = Wrapper<int32>.fallback;
/// @type.symbol symbol=read source=read type=int32 | undefined
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=Wrapper target=Wrapper
/// @resolution.member source=Wrapper<int32>.fallback receiver=Wrapper<int32> type=int32 | undefined kind=symbol target_receiver=Wrapper<int32> target=Wrapper.fallback
/// @resolution.function source=Wrapper<int32> type=Wrapper<int32> target=Wrapper instance=Wrapper<int32>
/// @generic.instantiation id=Wrapper.fallback<int32> template=Wrapper.fallback arguments=(int32)
/// @generic.instantiation id=Wrapper<int32> template=Wrapper arguments=(int32)
"#, r#""#);
}

/// A static read through a bare generic template cannot infer its owner arguments.
#[test]
fn test_reject_a_static_read_through_a_bare_generic_template() {
    let session = TestSession::single(
        r#"
struct Holder<T: Copy> {
    value: T;

    const zero: int32 = 0;
}

const read = Holder.zero;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Holder<out T: Copy> {
    value: T;

    const zero: int32 = 0;
}

const read: int32 = Holder.zero;

=== dir ===
struct Holder<T: Copy> {
/// @generic.template symbol=Holder parameters=(out T: Copy)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T: Copy)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @definition.associated.const symbol=Holder.zero source="const zero: int32 = 0" key=zero type=int32
/// @type.symbol symbol=Holder.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

    const zero: int32 = 0;
    /// @type.symbol symbol=Holder.zero source="const zero: int32 = 0" type=int32

}

const read = Holder.zero;
/// @type.symbol symbol=read source=read type=int32
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=Holder target=Holder
/// @resolution.member source=Holder.zero receiver=Holder type=int32 kind=symbol target_receiver=Holder target=Holder.zero
/// @generic.instantiation id=Holder.zero<Copy> template=Holder.zero arguments=(Copy)
"#, r#"

"#);
}

/// A static read infers its owner arguments from the annotation.
#[test]
fn test_infer_static_owner_arguments_from_the_annotation() {
    let session = TestSession::single(
        r#"
struct Registry<T: Copy> {
    value: T;

    static fallback: T | undefined = undefined;
}

const fallback: int32 | undefined = Registry.fallback;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Registry<out T: Copy> {
    value: T;

    static fallback: T | undefined = undefined as T | undefined;
}

const fallback: int32 | undefined = Registry.fallback;

=== dir ===
struct Registry<T: Copy> {
/// @generic.template symbol=Registry parameters=(out T: Copy)
/// @type.symbol symbol=Registry type=Registry
/// @definition.struct symbol=Registry template=(out T: Copy)
/// @definition.field symbol=Registry.fallback source="static fallback: T | undefined = undefined" key=fallback static=true type=T | undefined
/// @definition.field symbol=Registry.value source="value: T" key=value type=T
/// @type.symbol symbol=Registry.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Registry.value source="value: T" type=T
    /// @resolution.name source=T target=Registry.T

    static fallback: T | undefined = undefined;
    /// @type.symbol symbol=Registry.fallback source="static fallback: T | undefined = undefined" type=T | undefined
    /// @resolution.name source=T target=Registry.T

}

const fallback: int32 | undefined = Registry.fallback;
/// @type.symbol symbol=fallback source=fallback type=int32 | undefined
/// @resolution.pattern source=fallback kind=binding target=fallback
/// @resolution.name source=Registry target=Registry
/// @resolution.member source=Registry.fallback receiver=Registry type=int32 | undefined kind=field target_receiver=Registry key=fallback target=Registry.fallback target_type=int32 | undefined
/// @resolution.place source=Registry.fallback placement="local" lifetime="static" access="mutable"
/// @resolution.access source=Registry.fallback root=Registry keys=[fallback]
"#, r#""#);
}
