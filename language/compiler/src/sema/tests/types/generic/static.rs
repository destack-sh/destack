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
/// @resolution.name source=Holder<int32> target=Holder
/// @resolution.member source=Holder<int32>.zero receiver=Holder<int32> type=int32 kind=symbol target_receiver=Holder<int32> target=Holder.zero
/// @generic.instantiation id=Holder.zero<int32> template=Holder.zero arguments=(int32)
"#, r#"
"#);
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
/// @resolution.name source=Registry<int32> target=Registry
/// @resolution.member source=Registry<int32>.fallback receiver=Registry<int32> type=int32 | undefined kind=field target_receiver=Registry<int32> key=fallback target=Registry.fallback target_type=int32 | undefined
/// @resolution.access source=Registry<int32>.fallback root=Registry keys=[fallback]
"#, r#"
"#);
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
/// @resolution.name source=Wrapper<int32> target=Wrapper
/// @resolution.member source=Wrapper<int32>.fallback receiver=Wrapper<int32> type=int32 | undefined kind=symbol target_receiver=Wrapper<int32> target=Wrapper.fallback
/// @generic.instantiation id=Wrapper.fallback<int32> template=Wrapper.fallback arguments=(int32)
"#, r#"
"#);
}

/// A static read through a bare generic template cannot infer its owner arguments.
#[test]
fn test_read_a_static_through_a_bare_generic_template() {
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
/// @resolution.place source=Registry.fallback placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=Registry.fallback root=Registry keys=[fallback]
"#, r#"
"#);
}

/// A static member reports an owner parameter nothing at the call infers.
#[test]
fn test_reject_an_owner_parameter_a_static_call_leaves_uninferred() {
    let session = TestSession::single(
        r#"
class Pair<A, B> {
    left: A;
    right: B;

    constructor(left: A, right: B) {
        this.left = left;
        this.right = right;
    }

    static first(value: A): A {
        value
    }
}

const read = Pair.first(1);
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
class Pair<in out A, in out B> {
    left: A;
    right: B;

    constructor(left: A, right: B) {
        this.left = left;
        this.right = right;
    }

    static first(value: A): A {
        value
    }
}

const read: int64 = Pair.first(1);

=== dir ===
class Pair<A, B> {
/// @generic.template symbol=Pair parameters=(in out A, in out B)
/// @type.symbol symbol=Pair type=typeof Pair
/// @definition.class symbol=Pair template=(in out A, in out B)
/// @definition.field symbol=Pair.left source="left: A" key=left type=A
/// @definition.field symbol=Pair.right source="right: B" key=right type=B
/// @definition.method symbol=Pair.constructor slot=constructor role=constructor type=(this: &'managed Pair<A, B>, A, B) => Pair<A, B>
/// @definition.method symbol=Pair.first slot=first static=true type=(A) => A
/// @type.symbol symbol=Pair.A source=A type=A
/// @type.symbol symbol=Pair.B source=B type=B

    left: A;
    /// @type.symbol symbol=Pair.left source="left: A" type=A
    /// @resolution.name source=A target=Pair.A

    right: B;
    /// @type.symbol symbol=Pair.right source="right: B" type=B
    /// @resolution.name source=B target=Pair.B

    constructor(left: A, right: B) {
    /// @type.symbol symbol=Pair.constructor type=(this: &'managed Pair<A, B>, A, B) => Pair<A, B>
    /// @type.symbol symbol=Pair.constructor.this type=&'managed Pair<A, B>
    /// @type.symbol symbol=Pair.constructor.left source="left: A" type=A
    /// @resolution.name source=A target=Pair.A
    /// @type.symbol symbol=Pair.constructor.right source="right: B" type=B
    /// @resolution.name source=B target=Pair.B

        this.left = left;
        /// @resolution.receiver source=this kind=this declaration=Pair type=&'managed Pair<A, B>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.left kind=place
        /// @resolution.place source=this.left placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.left root=this keys=[left]
        /// @resolution.assignment source=this.left write="receiver=&'managed Pair<A, B>, target=field(receiver=&'managed Pair<A, B>, target=Pair.left, type=A), type=A" type=A
        /// @resolution.name source=left target=Pair.constructor.left
        /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=left root=Pair.constructor.left

        this.right = right;
        /// @resolution.receiver source=this kind=this declaration=Pair type=&'managed Pair<A, B>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.right kind=place
        /// @resolution.place source=this.right placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.right root=this keys=[right]
        /// @resolution.assignment source=this.right write="receiver=&'managed Pair<A, B>, target=field(receiver=&'managed Pair<A, B>, target=Pair.right, type=B), type=B" type=B
        /// @resolution.name source=right target=Pair.constructor.right
        /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=right root=Pair.constructor.right

    }

    static first(value: A): A {
    /// @type.symbol symbol=Pair.first type=(A) => A
    /// @type.symbol symbol=Pair.first.value source="value: A" type=A
    /// @resolution.name source=A target=Pair.A
    /// @resolution.name source=A target=Pair.A

        value
        /// @resolution.name source=value target=Pair.first.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Pair.first.value

    }
}

const read = Pair.first(1);
/// @type.symbol symbol=read source=read type=int64
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=Pair target=Pair
/// @resolution.member source=Pair.first receiver=typeof Pair type=(A) => A kind=symbol target_receiver=typeof Pair target=Pair.first
/// @resolution.call source=Pair.first(1) parameters=(int64) arguments=(provided(1) as int64) return=int64 kind=symbol target=Pair.first instance="Pair<int64, <error>>.first"
"#, r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=16 column=14 span="Pair.first(1)" line_source="const read = Pair.first(1);"
/// @diagnostic.help message="annotate the type explicitly"
"#);
}

/// A static member takes the declared default of an owner parameter its signature leaves unnamed.
#[test]
fn test_bind_an_unnamed_owner_parameter_to_its_default() {
    let session = TestSession::single(
        r#"
class Pair<A, B = string> {
    left: A;
    right: B;

    constructor(left: A, right: B) {
        this.left = left;
        this.right = right;
    }

    static first(value: A): A {
        value
    }
}

const read = Pair.first(1);
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
class Pair<in out A, in out B = string> {
    left: A;
    right: B;

    constructor(left: A, right: B) {
        this.left = left;
        this.right = right;
    }

    static first(value: A): A {
        value
    }
}

const read: int64 = Pair.first<int64, string>(1);

=== dir ===
class Pair<A, B = string> {
/// @generic.template symbol=Pair parameters=(in out A, in out B = string)
/// @type.symbol symbol=Pair type=typeof Pair
/// @definition.class symbol=Pair template=(in out A, in out B = string)
/// @definition.field symbol=Pair.left source="left: A" key=left type=A
/// @definition.field symbol=Pair.right source="right: B" key=right type=B
/// @definition.method symbol=Pair.constructor slot=constructor role=constructor type=(this: &'managed Pair<A, B>, A, B) => Pair<A, B>
/// @definition.method symbol=Pair.first slot=first static=true type=(A) => A
/// @type.symbol symbol=Pair.A source=A type=A
/// @type.symbol symbol=Pair.B source="B = string" type=B

    left: A;
    /// @type.symbol symbol=Pair.left source="left: A" type=A
    /// @resolution.name source=A target=Pair.A

    right: B;
    /// @type.symbol symbol=Pair.right source="right: B" type=B
    /// @resolution.name source=B target=Pair.B

    constructor(left: A, right: B) {
    /// @type.symbol symbol=Pair.constructor type=(this: &'managed Pair<A, B>, A, B) => Pair<A, B>
    /// @type.symbol symbol=Pair.constructor.this type=&'managed Pair<A, B>
    /// @type.symbol symbol=Pair.constructor.left source="left: A" type=A
    /// @resolution.name source=A target=Pair.A
    /// @type.symbol symbol=Pair.constructor.right source="right: B" type=B
    /// @resolution.name source=B target=Pair.B

        this.left = left;
        /// @resolution.receiver source=this kind=this declaration=Pair type=&'managed Pair<A, B>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.left kind=place
        /// @resolution.place source=this.left placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.left root=this keys=[left]
        /// @resolution.assignment source=this.left write="receiver=&'managed Pair<A, B>, target=field(receiver=&'managed Pair<A, B>, target=Pair.left, type=A), type=A" type=A
        /// @resolution.name source=left target=Pair.constructor.left
        /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=left root=Pair.constructor.left

        this.right = right;
        /// @resolution.receiver source=this kind=this declaration=Pair type=&'managed Pair<A, B>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.right kind=place
        /// @resolution.place source=this.right placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.right root=this keys=[right]
        /// @resolution.assignment source=this.right write="receiver=&'managed Pair<A, B>, target=field(receiver=&'managed Pair<A, B>, target=Pair.right, type=B), type=B" type=B
        /// @resolution.name source=right target=Pair.constructor.right
        /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=right root=Pair.constructor.right

    }

    static first(value: A): A {
    /// @type.symbol symbol=Pair.first type=(A) => A
    /// @type.symbol symbol=Pair.first.value source="value: A" type=A
    /// @resolution.name source=A target=Pair.A
    /// @resolution.name source=A target=Pair.A

        value
        /// @resolution.name source=value target=Pair.first.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Pair.first.value

    }
}

const read = Pair.first(1);
/// @type.symbol symbol=read source=read type=int64
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=Pair target=Pair
/// @resolution.member source=Pair.first receiver=typeof Pair type=(A) => A kind=symbol target_receiver=typeof Pair target=Pair.first
/// @resolution.call source=Pair.first(1) parameters=(int64) arguments=(provided(1) as int64) return=int64 kind=symbol target=Pair.first instance="Pair<int64, string>.first"
/// @generic.instantiation id="Pair.first<int64, string>" template=Pair.first arguments=(int64, string)
"#, r#"
"#);
}
