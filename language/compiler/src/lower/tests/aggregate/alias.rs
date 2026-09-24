use crate::tests::TestSession;

#[test]
fn test_lower_borrowed_signature_alias_fields_without_arguments() {
    let session = TestSession::single(
        r#"
type Predicate = (value: &readonly int32) => boolean;

struct Rule {
    accept: Predicate;
}

function test(rule: &readonly Rule): int32 {
    return 3;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.test",
        r#"
type test.main.Rule {
    accept: function<<'a>(ref<int32, borrowed, 'a, readonly>) => boolean, repeatable, managed, mutable, local>;
}

function test.main.test<'a>(v0: ref<test.main.Rule, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Rule, borrowed, 'a, readonly>

entry(v0: ref<test.main.Rule, borrowed, 'a, readonly>):
    store l0, v0
    v1: int32 = 3
    return v1
}

/// @layout.struct name=test.main.Rule size=16 align=8
/// @layout.field owner=test.main.Rule index=0 name=accept offset=0 size=16 align=8
"#,
    );
}

#[test]
fn test_lower_an_intersection_alias_of_object_aliases_to_the_merged_struct() {
    let session = TestSession::single(
        r#"
type Overflow = { overflow: int32 };
type Offset = { offset: int32 };
type Options = Overflow & Offset;

function total(options: Options): int32 {
    return options.overflow + options.offset;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.total",
        r#"
function test.main.total(v0: ref<{ overflow: int32, offset: int32 }, managed, mutable, local>): int32 {
    local l0: ref<{ overflow: int32, offset: int32 }, managed, mutable, local>

entry(v0: ref<{ overflow: int32, offset: int32 }, managed, mutable, local>):
    store l0, v0
    v1: ref<{ overflow: int32, offset: int32 }, managed, mutable, local> = load l0
    v2: int32 = load (*v1).0
    v3: ref<{ overflow: int32, offset: int32 }, managed, mutable, local> = load l0
    v4: int32 = load (*v3).1
    v5: int32 = add v2, v4
    return v5
}

/// @layout.struct name=type@1 size=8 align=4
/// @layout.field owner=type@1 index=0 name=overflow offset=0 size=4 align=4
/// @layout.field owner=type@1 index=1 name=offset offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_an_alias_of_a_defaulted_intrinsic_application() {
    let session = TestSession::single(
        r#"
type Reaction = Function<(int32,), void>;

function run(reaction: Reaction): void {
    reaction(1);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.run",
        r#"
function test.main.run(v0: function<(int32) => void, repeatable, managed, mutable, local>): void {
    local l0: function<(int32) => void, repeatable, managed, mutable, local>

entry(v0: function<(int32) => void, repeatable, managed, mutable, local>):
    store l0, v0
    v1: function<(int32) => void, repeatable, managed, mutable, local> = load l0
    v2: int32 = 1
    call.indirect v1(v2): (int32) => void
    return
}
"#,
    );
}

#[test]
fn test_lower_an_intersection_alias_over_optional_object_properties() {
    let session = TestSession::single(
        r#"
type Unit<T> = T | "auto";

type Base = {
    digits?: "auto" | 0 | 1;
    unit?: Unit<"second" | "minute">;
};

type Options = Base & {
    zone?: string;
};

function pick(options: Options): void {}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.pick",
        r#"
type literal.string.auto { }

type literal.integer.0 { }

type literal.integer.1 { }

@nocopy
@languageItem("string.String")
type String;

function test.main.pick(v0: ref<{ digits: variant<uint2> { 0uint2 = literal.string.auto; 1uint2 = literal.integer.0; 2uint2 = literal.integer.1; 3uint2 = void; }, unit: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }, zone: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>): void {
    local l0: ref<{ digits: variant<uint2> { 0uint2 = literal.string.auto; 1uint2 = literal.integer.0; 2uint2 = literal.integer.1; 3uint2 = void; }, unit: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }, zone: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>

entry(v0: ref<{ digits: variant<uint2> { 0uint2 = literal.string.auto; 1uint2 = literal.integer.0; 2uint2 = literal.integer.1; 3uint2 = void; }, unit: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }, zone: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>):
    store l0, v0
    return
}

/// @layout.struct name=literal.string.auto size=0 align=1
/// @layout.struct name=literal.integer.0 size=0 align=1
/// @layout.struct name=literal.integer.1 size=0 align=1
/// @layout.variant name=type@7 size=1 align=1
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=1
/// @layout.case owner=type@7 index=2 discriminant=2 payload_offset=1
/// @layout.case owner=type@7 index=3 discriminant=3 payload_offset=1
/// @layout.variant name=type@14 size=8 align=8
/// @layout.discriminant owner=type@14 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@14 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@14 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@15 size=24 align=8
/// @layout.field owner=type@15 index=0 name=digits offset=16 size=1 align=1
/// @layout.field owner=type@15 index=1 name=unit offset=0 size=8 align=8
/// @layout.field owner=type@15 index=2 name=zone offset=8 size=8 align=8
"#,
    );
}

#[test]
fn test_lower_an_intersection_over_a_bare_defaulted_alias_reference() {
    let session = TestSession::single(
        r#"
type Formatting<T = string> = {
    style?: T;
};

function render(options: Formatting & { zone?: string }): void {}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.render",
        r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.render(v0: ref<{ style: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }, zone: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>): void {
    local l0: ref<{ style: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }, zone: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>

entry(v0: ref<{ style: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }, zone: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>):
    store l0, v0
    return
}

/// @layout.variant name=type@7 size=8 align=8
/// @layout.discriminant owner=type@7 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@8 size=16 align=8
/// @layout.field owner=type@8 index=0 name=style offset=0 size=8 align=8
/// @layout.field owner=type@8 index=1 name=zone offset=8 size=8 align=8
"#,
    );
}

#[test]
fn test_lower_an_imported_intersection_alias_over_optional_fields() {
    let session = TestSession::builder()
        .module(
            "fields.ds",
            r#"
export type Fields = {
    year?: number;
    month?: number;
};

export type Input = Fields & {
    offset?: string;
};
"#,
        )
        .module(
            "main.ds",
            r#"
import { Input } from "./fields.ds";

function read(input: Input): void {}
"#,
        )
        .build();

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.read(v0: ref<{ year: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, month: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, offset: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>): void {
    local l0: ref<{ year: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, month: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, offset: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>

entry(v0: ref<{ year: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, month: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, offset: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>):
    store l0, v0
    return
}

/// @layout.variant name=type@3 size=16 align=8
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=8
/// @layout.variant name=type@9 size=8 align=8
/// @layout.discriminant owner=type@9 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@9 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@9 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@10 size=40 align=8
/// @layout.field owner=type@10 index=0 name=year offset=0 size=16 align=8
/// @layout.field owner=type@10 index=1 name=month offset=16 size=16 align=8
/// @layout.field owner=type@10 index=2 name=offset offset=32 size=8 align=8
"#,
    );
}

#[test]
fn test_lower_an_intersection_over_an_imported_alias() {
    let session = TestSession::builder()
        .module(
            "fields.ds",
            r#"
export type Fields = {
    year?: number;
    month?: number;
};
"#,
        )
        .module(
            "main.ds",
            r#"
import { Fields } from "./fields.ds";

export type Input = Fields & {
    offset?: string;
};

function read(input: Input): void {}
"#,
        )
        .build();

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.read(v0: ref<{ year: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, month: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, offset: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>): void {
    local l0: ref<{ year: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, month: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, offset: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>

entry(v0: ref<{ year: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, month: variant<uint1> { 0uint1 = float64; 1uint1 = void; }, offset: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } }, managed, mutable, local>):
    store l0, v0
    return
}

/// @layout.variant name=type@3 size=16 align=8
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=8
/// @layout.variant name=type@9 size=8 align=8
/// @layout.discriminant owner=type@9 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@9 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@9 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@10 size=40 align=8
/// @layout.field owner=type@10 index=0 name=year offset=0 size=16 align=8
/// @layout.field owner=type@10 index=1 name=month offset=16 size=16 align=8
/// @layout.field owner=type@10 index=2 name=offset offset=32 size=8 align=8
"#,
    );
}

#[test]
fn test_lower_a_region_elided_alias_application() {
    let session = TestSession::single(
        r#"
import { Cow } from "destack:memory";
import { StringSlice } from "destack:string";

function keep(value: Cow<StringSlice>): void {}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.keep",
        r#"
@languageItem("string.StringSlice")
type StringSlice;

@languageItem("memory.Cow")
type Cow<'a, T: ToOwned, P0>;

@nocopy
@languageItem("string.String")
type String;

function test.main.keep<'a>(v0: Cow<'a, StringSlice, String>): void {
    local l0: Cow<'a, StringSlice, String>

entry(v0: Cow<'a, StringSlice, String>):
    store l0, v0
    return
}
"#,
    );
}

#[test]
fn test_lower_a_region_elided_alias_argument_of_a_newtype_application() {
    let session = TestSession::single(
        r#"
import { Error, Result } from "destack:error";
import { Cow } from "destack:memory";
import { StringSlice } from "destack:string";

declare function read(): Result<Cow<StringSlice>, Error>;

function keep(): void {
    read();
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.keep",
        r#"
@languageItem("string.StringSlice")
type StringSlice;

@nocopy
@languageItem("string.String")
type String;

@languageItem("memory.Cow")
type Cow<'a, T: ToOwned, P0>;

@nocopy
@languageItem("error.Error")
type Error;

@languageItem("error.Result")
type Result<T, E>;

function test.main.keep(): void {
entry:
    v0: Result<Cow<'managed, StringSlice, String>, dynamic<Error, managed, mutable, local>> = call test.main.read(): () => Result<Cow<'managed, StringSlice, String>, dynamic<Error, managed, mutable, local>>
    return
}
"#,
    );
}
