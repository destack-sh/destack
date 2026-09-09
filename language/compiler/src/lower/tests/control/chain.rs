use crate::tests::TestSession;

#[test]
fn test_lower_an_optional_chain_to_a_short_circuit() {
    let session = TestSession::single(
        r#"
struct Options {
    retries: int32;
}

function read(options: Options | undefined): int32 | undefined {
    return options?.retries;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.read", r#"
@copy
type test.main.Options {
    retries: int32;
}

function test.main.read(v0: variant<uint1> { 0uint1 = void; 1uint1 = test.main.Options; }): variant<uint1> { 0uint1 = void; 1uint1 = int32; } {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = test.main.Options; }
    local l1: variant<uint1> { 0uint1 = void; 1uint1 = int32; }

entry(v0: variant<uint1> { 0uint1 = void; 1uint1 = test.main.Options; }):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = test.main.Options; } = local.get l0
    v2: test.main.Options = variant.payload v1, 1
    v3: int32 = field.get v2, 0
    v4: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 1, v3
    local.set l1, v4
    jump b1

b1:
    v5: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = local.get l1
    return v5
}

/// @layout.struct name=test.main.Options size=4 align=4
/// @layout.field owner=test.main.Options index=0 name=retries offset=0 size=4 align=4
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@7 size=8 align=4
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_a_chained_method_call_through_the_guard() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    total(this): int32 {
        this.value
    }
}

function read(counter: Counter | undefined): int32 | undefined {
    return counter?.total();
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Counter.total",
        r#"
@copy
type test.main.Counter {
    value: int32;
}

function test.main.Counter.total(v0: test.main.Counter): int32 {
    local l0: test.main.Counter

entry(v0: test.main.Counter):
    local.set l0, v0
    v1: test.main.Counter = local.get l0
    v2: int32 = field.get v1, 0
    return v2
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.read", r#"
@copy
type test.main.Counter {
    value: int32;
}

function test.main.read(v0: variant<uint1> { 0uint1 = void; 1uint1 = test.main.Counter; }): variant<uint1> { 0uint1 = void; 1uint1 = int32; } {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = test.main.Counter; }
    local l1: variant<uint1> { 0uint1 = void; 1uint1 = int32; }

entry(v0: variant<uint1> { 0uint1 = void; 1uint1 = test.main.Counter; }):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = test.main.Counter; } = local.get l0
    variant.switch v1, 0 => b3, else b2

b1:
    v6: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = local.get l1
    return v6

b2:
    v3: test.main.Counter = variant.payload v1, 1
    v4: int32 = call test.main.Counter.total(v3): (test.main.Counter) => int32
    v5: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 1, v4
    local.set l1, v5
    jump b1

b3:
    v2: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 0
    local.set l1, v2
    jump b1
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=0 size=4 align=4
/// @layout.variant name=type@7 size=8 align=4
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@8 size=8 align=4
/// @layout.discriminant owner=type@8 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_is_on_a_union_case() {
    let session = TestSession::single(
        r#"
type Message = string | ^Function<(), string, "once">;

function evaluate(message: Message | undefined): string | undefined {
    if (message is ^Function<(), string, "once">) {
        return message();
    }

    return message;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.evaluate", r#"
@languageItem("string.String")
type String;

function test.main.evaluate(v0: variant<uint2> { 0uint2 = ref<String, managed, mutable, local>; 1uint2 = void; 2uint2 = function<() => ref<String, managed, mutable, local>, once, unique, mutable, local>; }): variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } {
    local l0: variant<uint2> { 0uint2 = ref<String, managed, mutable, local>; 1uint2 = void; 2uint2 = function<() => ref<String, managed, mutable, local>, once, unique, mutable, local>; }
    local l1: boolean, readonly
    local l2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }

entry(v0: variant<uint2> { 0uint2 = ref<String, managed, mutable, local>; 1uint2 = void; 2uint2 = function<() => ref<String, managed, mutable, local>, once, unique, mutable, local>; }):
    local.set l0, v0
    v1: ref<variant<uint2> { 0uint2 = ref<String, managed, mutable, local>; 1uint2 = void; 2uint2 = function<() => ref<String, managed, mutable, local>, once, unique, mutable, local>; }, borrowed, 'frame, readonly, frame> = local.project l0
    v2: uint2 = variant.tag.load v1
    switch v2, b1, 2 => b3

b1:
    v4: boolean = false
    local.set l1, v4
    jump b2

b2:
    v5: boolean = local.get l1
    branch v5 => b4 | b5

b3:
    v3: boolean = true
    local.set l1, v3
    jump b2

b4:
    v6: ref<variant<uint2> { 0uint2 = ref<String, managed, mutable, local>; 1uint2 = void; 2uint2 = function<() => ref<String, managed, mutable, local>, once, unique, mutable, local>; }, borrowed, 'frame, readonly, frame> = local.project l0
    v7: ref<function<() => ref<String, managed, mutable, local>, once, unique, mutable, local>, borrowed, 'frame, readonly, frame> = variant.payload.project v6, 2
    v8: function<() => ref<String, managed, mutable, local>, once, unique, mutable, local> = load v7
    v9: ref<String, managed, mutable, local> = call.indirect v8(): () => ref<String, managed, mutable, local>
    v10: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v9
    return v10

b5:
    v11: variant<uint2> { 0uint2 = ref<String, managed, mutable, local>; 1uint2 = void; 2uint2 = function<() => ref<String, managed, mutable, local>, once, unique, mutable, local>; } = local.get l0
    variant.switch v11, 0 => b8, 1 => b9, else b7

b6:
    v15: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = local.get l2
    return v15

b7:
    panic

b8:
    v12: ref<String, managed, mutable, local> = variant.payload v11, 0
    v13: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v12
    local.set l2, v13
    jump b6

b9:
    v14: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = variant.new 1
    local.set l2, v14
    jump b6
}

/// @layout.variant name=type@14 size=24 align=8
/// @layout.discriminant owner=type@14 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@14 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@14 index=1 discriminant=1 payload_offset=8
/// @layout.case owner=type@14 index=2 discriminant=2 payload_offset=8
/// @layout.variant name=type@16 size=8 align=8
/// @layout.discriminant owner=type@16 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@16 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@16 index=1 discriminant=1 payload_offset=0
"#);
}

#[test]
fn test_lower_a_try_projection_propagating_its_residual() {
    let session = TestSession::single(
        r#"
import { Result } from "destack:error";

function read(): Result<int32, string> {
    return Result.ok(1);
}

function twice(): Result<int32, string> {
    let value = read()?;

    return Result.ok(value + value);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.read", r#"
@languageItem("string.String")
type String;

@copy
@languageItem("error.Result")
type Result<T, E>;

function test.main.read(): Result<int32, ref<String, managed, mutable, local>> {
entry:
    v0: int32 = 1
    v1: Result<int32, ref<String, managed, mutable, local>> = call Result.ok<int32, ref<String, managed, mutable, local>>(v0): (int32) => Result<int32, ref<String, managed, mutable, local>>
    return v1
}
"#);

    session.assert_mir_function("main.ds", "test.main.twice", r#"
@languageItem("string.String")
type String;

@copy
@languageItem("error.Result")
type Result<T, E>;

@copy
@languageItem("ops.Continue")
type Continue<C>;

@copy
@languageItem("ops.Break")
type Break<B>;

@copy
@languageItem("ops.ControlFlow")
type ControlFlow<B, C>;

function test.main.twice(): Result<int32, ref<String, managed, mutable, local>> {
    local l0: int32

entry:
    v0: Result<int32, ref<String, managed, mutable, local>> = call test.main.read(): () => Result<int32, ref<String, managed, mutable, local>>
    v1: ControlFlow<ref<String, managed, mutable, local>, int32> = call Result.Try.branch<int32, ref<String, managed, mutable, local>>(v0): (Result<int32, ref<String, managed, mutable, local>>) => ControlFlow<ref<String, managed, mutable, local>, int32>
    v2: variant<uint1> { 0uint1 = Break<ref<String, managed, mutable, local>>; 1uint1 = Continue<int32>; } = field.get v1, 0
    variant.switch v2, 0 => b1, 1 => b2

b1:
    v3: Break<ref<String, managed, mutable, local>> = variant.payload v2, 0
    v4: ref<String, managed, mutable, local> = field.get v3, 1
    v5: Result<int32, ref<String, managed, mutable, local>> = call Result.FromResidual.fromResidual<int32, ref<String, managed, mutable, local>, ref<String, managed, mutable, local>>(v4): (ref<String, managed, mutable, local>) => Result<int32, ref<String, managed, mutable, local>>
    return v5

b2:
    v6: Continue<int32> = variant.payload v2, 1
    v7: int32 = field.get v6, 1
    local.set l0, v7
    v8: int32 = local.get l0
    v9: int32 = local.get l0
    v10: int32 = add v8, v9
    v11: Result<int32, ref<String, managed, mutable, local>> = call Result.ok<int32, ref<String, managed, mutable, local>>(v10): (int32) => Result<int32, ref<String, managed, mutable, local>>
    return v11
}

/// @layout.variant name=type@99 size=16 align=8
/// @layout.discriminant owner=type@99 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@99 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@99 index=1 discriminant=1 payload_offset=8
"#);
}
