use crate::tests::TestSession;

#[test]
fn test_lower_enum_members_to_variant_cases() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function pick(flag: boolean): Mode {
    if (flag) {
        return Mode.Read;
    }
    return Mode.Write;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.pick",
        r#"
type test.main.Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

export function test.main.pick(v0: boolean): test.main.Mode {
    local l0: boolean

entry(v0: boolean):
    store l0, v0
    v1: boolean = load l0
    branch v1 => b1 | b2

b1:
    v2: test.main.Mode = variant.new 0
    return v2

b2:
    v3: test.main.Mode = variant.new 1
    return v3
}

/// @layout.variant name=test.main.Mode size=1 align=1
/// @layout.discriminant owner=test.main.Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Mode index=1 discriminant=2 payload_offset=1
"#,
    );
}

#[test]
fn test_lower_enum_match_to_a_variant_switch() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): int32 {
    match (mode) {
        Mode.Read => 10,
        Mode.Write => 20,
    }
}

function fallback(mode: Mode): int32 {
    match (mode) {
        Mode.Read => 10,
        _ => 0,
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.describe",
        r#"
type test.main.Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

export function test.main.describe(v0: test.main.Mode): int32 {
    local l0: test.main.Mode
    local l1: int32

entry(v0: test.main.Mode):
    store l0, v0
    v1: uint8 = variant.tag.load l0
    switch v1, b3, 0 => b1, 1 => b2

b1:
    v2: int32 = 10
    store l1, v2
    jump b4

b2:
    v3: int32 = 20
    store l1, v3
    jump b4

b3:
    unreachable

b4:
    v4: int32 = load l1
    return v4
}

/// @layout.variant name=test.main.Mode size=1 align=1
/// @layout.discriminant owner=test.main.Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Mode index=1 discriminant=2 payload_offset=1
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.fallback",
        r#"
type test.main.Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

export function test.main.fallback(v0: test.main.Mode): int32 {
    local l0: test.main.Mode
    local l1: int32

entry(v0: test.main.Mode):
    store l0, v0
    v1: uint8 = variant.tag.load l0
    switch v1, b2, 0 => b1

b1:
    v2: int32 = 10
    store l1, v2
    jump b4

b2:
    v3: int32 = 0
    store l1, v3
    jump b4

b3:
    unreachable

b4:
    v4: int32 = load l1
    return v4
}

/// @layout.variant name=test.main.Mode size=1 align=1
/// @layout.discriminant owner=test.main.Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Mode index=1 discriminant=2 payload_offset=1
"#,
    );
}

/// Lower string enum cases to their constant String objects.
#[test]
fn test_lower_string_enum_cases_to_constant_strings() {
    let session = TestSession::single(
        r#"
enum Level {
    Info = "info",
    Error = "error",
}

function isError(level: Level): boolean {
    level === Level.Error
}

function rank(level: Level): int32 {
    match (level) {
        Level.Info => 1,
        Level.Error => 2,
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
type test.main.Level = newtype<ref<String, managed, mutable, local>>;

@nocopy
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

shared constant string.0: String = "error"
shared constant string.1: String = "info"

export function test.main.isError(v0: test.main.Level): boolean {
    local l0: test.main.Level

entry(v0: test.main.Level):
    store l0, v0
    v1: test.main.Level = load l0
    v2: ref<String, managed, mutable, local> = field.get v1, 0
    v3: ref<String, managed, mutable, local> = address @string.0
    v4: test.main.Level = aggregate (v3)
    v5: ref<String, managed, mutable, local> = field.get v4, 0
    v6: boolean = eq v2, v5
    return v6
}

export function test.main.rank(v0: test.main.Level): int32 {
    local l0: test.main.Level
    local l1: int32

entry(v0: test.main.Level):
    store l0, v0
    jump b1

b1:
    v1: test.main.Level = load l0
    v2: ref<String, managed, mutable, local> = field.get v1, 0
    v3: ref<String, managed, mutable, local> = address @string.1
    v4: boolean = eq v2, v3
    branch v4 => b5 | b2

b2:
    v6: test.main.Level = load l0
    v7: ref<String, managed, mutable, local> = field.get v6, 0
    v8: ref<String, managed, mutable, local> = address @string.0
    v9: boolean = eq v7, v8
    branch v9 => b6 | b3

b3:
    unreachable

b4:
    v11: int32 = load l1
    return v11

b5:
    v5: int32 = 1
    store l1, v5
    jump b4

b6:
    v10: int32 = 2
    store l1, v10
    jump b4
}

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=type@5 size=16 align=8
/// @layout.field owner=type@5 index=0 name=codeUnits offset=0 size=16 align=8
"#,
    );
}

/// Cast enum cases to their declared values: an integer enum's tag, a string enum's String.
#[test]
fn test_lower_enum_casts_to_their_declared_values() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

enum Level {
    Info = "info",
}

function code(mode: Mode): int64 {
    mode as int64
}

function text(level: Level): string {
    level as string
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.code",
        r#"
type test.main.Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

export function test.main.code(v0: test.main.Mode): int64 {
    local l0: test.main.Mode

entry(v0: test.main.Mode):
    store l0, v0
    v1: test.main.Mode = load l0
    v2: uint8 = variant.tag v1
    v3: int64 = cast.intToInt v2 -> int64
    return v3
}

/// @layout.variant name=test.main.Mode size=1 align=1
/// @layout.discriminant owner=test.main.Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Mode index=1 discriminant=2 payload_offset=1
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.text",
        r#"
type test.main.Level = newtype<ref<String, managed, mutable, local>>;

@nocopy
@languageItem("string.String")
type String;

export function test.main.text(v0: test.main.Level): ref<String, managed, mutable, local> {
    local l0: test.main.Level

entry(v0: test.main.Level):
    store l0, v0
    v1: test.main.Level = load l0
    v2: ref<String, managed, mutable, local> = field.get v1, 0
    return v2
}
"#,
    );
}

/// Encode enum constants as static cases, singletons in no bytes, and computed constants at init.
#[test]
fn test_lower_module_constants_of_enum_and_literal_types() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

enum Level {
    Info = "info",
}

const mode = Mode.Write;
const level = Level.Info;
const word = "hi";
const count = 3;
const code = mode as int64;
const next = count + 1;
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
type test.main.Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

type test.main.Level = newtype<ref<String, managed, mutable, local>>;

@nocopy
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

type literal.string.hi { }

type literal.integer.3 { }

type literal.integer.4 { }

type literal.integer.1 { }

export constant test.main.mode: test.main.Mode = variant 1
shared constant string.0: String = "info"
export constant test.main.level: test.main.Level = {globalAddress string.0}
export constant test.main.word: literal.string.hi = zeroinit
export constant test.main.count: literal.integer.3 = zeroinit
export global test.main.code: int64 = zeroinit
export global test.main.next: literal.integer.4 = zeroinit

export park function test.main.@init(): void {
entry:
    v0: test.main.Mode = load @test.main.mode
    v1: uint8 = variant.tag v0
    v2: int64 = cast.intToInt v1 -> int64
    store @test.main.code, v2
    v3: literal.integer.3 = load @test.main.count
    v4: literal.integer.3 = zeroed
    v5: literal.integer.1 = zeroed
    v6: literal.integer.4 = zeroed
    v7: literal.integer.4 = zeroed
    store @test.main.next, v7
    return
}

/// @layout.variant name=test.main.Mode size=1 align=1
/// @layout.discriminant owner=test.main.Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Mode index=1 discriminant=2 payload_offset=1
/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=literal.string.hi size=0 align=1
/// @layout.struct name=literal.integer.3 size=0 align=1
/// @layout.struct name=literal.integer.4 size=0 align=1
/// @layout.struct name=literal.integer.1 size=0 align=1
/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=1 payload_offset=1
/// @layout.case owner=type@3 index=1 discriminant=2 payload_offset=1
/// @layout.struct name=type@9 size=16 align=8
/// @layout.field owner=type@9 index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=type@12 size=0 align=1
"#,
    );
}
