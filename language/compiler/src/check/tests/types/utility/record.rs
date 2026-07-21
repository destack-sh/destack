use crate::tests::{DirRows, TestSession};

#[test]
fn test_record_finite_key_union_builds_exact_fields() {
    let session = TestSession::single(
        r#"
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true, b: false };

flags.a satisfies boolean;
flags.b satisfies boolean;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true, b: false };

flags.a satisfies boolean;
flags.b satisfies boolean;

=== checked ===
type Flags = Record<"a" | "b", boolean>;
/// @type.symbol symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" type=Record<"a" | "b", boolean> reduced={ a: boolean; b: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" value=Record<"a" | "b", boolean> reduced={ a: boolean; b: boolean }
/// @resolution.name source=Record target=types.object.Record

const flags: Flags = { a: true, b: false };
/// @type.symbol symbol=flags source=flags type=Flags reduced={ a: boolean; b: boolean }
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags

flags.a satisfies boolean;
/// @resolution.name source=flags target=flags
/// @resolution.member source=flags.a receiver={ a: boolean; b: boolean } kind=field key=a

flags.b satisfies boolean;
/// @resolution.name source=flags target=flags
/// @resolution.member source=flags.b receiver={ a: boolean; b: boolean } kind=field key=b

/// @generic.instance id="Record<\"a\" | \"b\", boolean>" template=types.object.Record arguments=("a" | "b", boolean)
"#,
    );
}

#[test]
fn test_record_usize_literal_keys_build_exact_fields() {
    let session = TestSession::single(
        r#"
type Flags = Record<1 | 2, string>;

const flags: Flags = { 1: "one", 2: "two" };

flags[1] satisfies string;
flags[2] satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags = Record<1 | 2, string>;

const flags: Flags = { 1: "one", 2: "two" };

flags[1] satisfies string;
flags[2] satisfies string;

=== checked ===
type Flags = Record<1 | 2, string>;
/// @type.symbol symbol=Flags source="type Flags = Record<1 | 2, string>" type=Record<1 | 2, string> reduced={ 1: string; 2: string }
/// @definition.type symbol=Flags source="type Flags = Record<1 | 2, string>" value=Record<1 | 2, string> reduced={ 1: string; 2: string }
/// @resolution.name source=Record target=types.object.Record

const flags: Flags = { 1: "one", 2: "two" };
/// @type.symbol symbol=flags source=flags type=Flags reduced={ 1: string; 2: string }
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags

flags[1] satisfies string;
/// @resolution.name source=flags target=flags
/// @resolution.member source=flags[1] receiver={ 1: string; 2: string } kind=field key=1

flags[2] satisfies string;
/// @resolution.name source=flags target=flags
/// @resolution.member source=flags[2] receiver={ 1: string; 2: string } kind=field key=2

/// @generic.instance id="Record<1 | 2, string>" template=types.object.Record arguments=(1 | 2, string)
"#,
    );
}

#[test]
fn test_record_finite_key_union_rejects_missing_key() {
    let session = TestSession::single(
        r#"
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true };

=== checked ===
type Flags = Record<"a" | "b", boolean>;
/// @type.symbol symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" type=Record<"a" | "b", boolean> reduced={ a: boolean; b: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" value=Record<"a" | "b", boolean> reduced={ a: boolean; b: boolean }
/// @resolution.name source=Record target=types.object.Record

const flags: Flags = { a: true };
/// @type.symbol symbol=flags source=flags type=Flags reduced={ a: boolean; b: boolean }
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags

/// @generic.instance id="Record<\"a\" | \"b\", boolean>" template=types.object.Record arguments=("a" | "b", boolean)
"#,
        r#"
/// @diagnostic.error id=missing-required-property message="missing required property 'b' for type 'Flags'"
/// @diagnostic.label line=4 column=22 span="{ a: true }" line_source="const flags: Flags = { a: true };"
/// @diagnostic.related line=4 column=14 span="Flags" line_source="const flags: Flags = { a: true };" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_record_usize_literal_keys_reject_missing_key() {
    let session = TestSession::single(
        r#"
type Flags = Record<1 | 2, string>;

const flags: Flags = { 1: "one" };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags = Record<1 | 2, string>;

const flags: Flags = { 1: "one" };

=== checked ===
type Flags = Record<1 | 2, string>;
/// @type.symbol symbol=Flags source="type Flags = Record<1 | 2, string>" type=Record<1 | 2, string> reduced={ 1: string; 2: string }
/// @definition.type symbol=Flags source="type Flags = Record<1 | 2, string>" value=Record<1 | 2, string> reduced={ 1: string; 2: string }
/// @resolution.name source=Record target=types.object.Record

const flags: Flags = { 1: "one" };
/// @type.symbol symbol=flags source=flags type=Flags reduced={ 1: string; 2: string }
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags

/// @generic.instance id="Record<1 | 2, string>" template=types.object.Record arguments=(1 | 2, string)
"#,
        r#"
/// @diagnostic.error id=missing-required-property message="missing required property '2' for type 'Flags'"
/// @diagnostic.label line=4 column=22 span="{ 1: \"one\" }" line_source="const flags: Flags = { 1: \"one\" };"
/// @diagnostic.related line=4 column=14 span="Flags" line_source="const flags: Flags = { 1: \"one\" };" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_record_finite_key_union_rejects_extra_key() {
    let session = TestSession::single(
        r#"
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true, b: false, c: true };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true, b: false, c: true };

=== checked ===
type Flags = Record<"a" | "b", boolean>;
/// @type.symbol symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" type=Record<"a" | "b", boolean> reduced={ a: boolean; b: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" value=Record<"a" | "b", boolean> reduced={ a: boolean; b: boolean }
/// @resolution.name source=Record target=types.object.Record

const flags: Flags = { a: true, b: false, c: true };
/// @type.symbol symbol=flags source=flags type=Flags reduced={ a: boolean; b: boolean }
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags

/// @generic.instance id="Record<\"a\" | \"b\", boolean>" template=types.object.Record arguments=("a" | "b", boolean)
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'c' in object literal for type 'Flags'"
/// @diagnostic.label line=4 column=22 span="{ a: true, b: false, c: true }" line_source="const flags: Flags = { a: true, b: false, c: true };"
/// @diagnostic.related line=4 column=14 span="Flags" line_source="const flags: Flags = { a: true, b: false, c: true };" message="expected due to this annotation"
/// @diagnostic.note message="object literals may only specify known properties"
"#,
    );
}

#[test]
fn test_record_rejects_invalid_key_type() {
    let session = TestSession::single(
        r#"
type Bad = Record<{ name: string }, boolean>;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bad = Record<{ name: string }, boolean>;

=== checked ===
type Bad = Record<{ name: string }, boolean>;
/// @type.symbol symbol=Bad source="type Bad = Record<{ name: string }, boolean>" type=<error>
/// @definition.type symbol=Bad source="type Bad = Record<{ name: string }, boolean>" value=<error>
/// @resolution.name source=Record target=types.object.Record
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type '{ name: string }' does not satisfy 'PropertyKey'"
/// @diagnostic.label line=2 column=19 span="{ name: string }" line_source="type Bad = Record<{ name: string }, boolean>;"
/// @diagnostic.related file="object.ds" message="required by this bound on 'K'"
/// @diagnostic.note message="'PropertyKey' reduces to 'string | usize | symbol'"
"#,
    );
}

#[test]
fn test_record_unique_symbol_key_builds_exact_field() {
    let session = TestSession::single(
        r#"
declare const key: unique symbol;

type Flags = Record<typeof key, boolean>;

const flags: Flags = { [key]: true };

flags[key] satisfies boolean;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const key: unique symbol;

type Flags = Record<typeof key, boolean>;

const flags: Flags = { [key]: true };

flags[key] satisfies boolean;

=== checked ===
declare const key: unique symbol;
/// @type.symbol symbol=key source=key type=unique symbol
/// @resolution.pattern source=key kind=binding target=key

type Flags = Record<typeof key, boolean>;
/// @type.symbol symbol=Flags source="type Flags = Record<typeof key, boolean>" type=Record<typeof key, boolean> reduced={ [key]: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<typeof key, boolean>" value=Record<typeof key, boolean> reduced={ [key]: boolean }
/// @resolution.name source=Record target=types.object.Record
/// @resolution.name source=key target=key

const flags: Flags = { [key]: true };
/// @type.symbol symbol=flags source=flags type=Flags reduced={ [key]: boolean }
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags
/// @resolution.name source=key target=key

flags[key] satisfies boolean;
/// @resolution.name source=flags target=flags
/// @resolution.member source=flags[key] receiver={ [key]: boolean } kind=field key=key
/// @resolution.name source=key target=key

/// @generic.instance id="Record<typeof key, boolean>" template=types.object.Record arguments=(typeof key, boolean)
"#,
    );
}

#[test]
fn test_record_string_key_constraint_rejects_finite_object() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare function read(bag: Bag): int32 | undefined;

const point: { x: int32 } = { x: 1 };
const value = read(point);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare function read(bag: Bag): int32 | undefined;

const point: { x: int32 } = { x: 1 };
const value: int32 | undefined = read(point);

=== checked ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type=Record<string, int32> reduced={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32> reduced={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare function read(bag: Bag): int32 | undefined;
/// @type.symbol symbol=read source="declare function read(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
/// @type.symbol symbol=read.bag source="bag: Bag" type=Bag reduced={ [P: string]: int32 }
/// @resolution.name source=Bag target=Bag

const point: { x: int32 } = { x: 1 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point

const value = read(point);
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=read target=read
/// @resolution.call source=read(point) parameters=(Bag) arguments=(provided(point) as Bag) return=int32 | undefined kind=symbol target=read
/// @resolution.name source=point target=point

/// @generic.instance id="Record<string, int32>" template=types.object.Record arguments=(string, int32)
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '{ x: int32 }' is not assignable to parameter of type 'Bag'"
/// @diagnostic.label line=7 column=20 span="point" line_source="const value = read(point);"
/// @diagnostic.related line=7 column=15 span="read(point)" line_source="const value = read(point);" message="in this call"
/// @diagnostic.note message="'Bag' reduces to '{ [P: string]: int32 }'"
"#,
    );
}

#[test]
fn test_record_string_key_constraint_accepts_fresh_object() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare function read(bag: Bag): int32 | undefined;

const value = read({ x: 1, y: 2 });
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare function read(bag: Bag): int32 | undefined;

const value: int32 | undefined = read({ x: 1, y: 2 });

=== checked ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type=Record<string, int32> reduced={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32> reduced={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare function read(bag: Bag): int32 | undefined;
/// @type.symbol symbol=read source="declare function read(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
/// @type.symbol symbol=read.bag source="bag: Bag" type=Bag reduced={ [P: string]: int32 }
/// @resolution.name source=Bag target=Bag

const value = read({ x: 1, y: 2 });
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=read target=read
/// @resolution.call source="read({ x: 1, y: 2 })" parameters=(Bag) arguments=(provided({ x: 1, y: 2 }) as Bag) return=int32 | undefined kind=symbol target=read

/// @generic.instance id="Record<string, int32>" template=types.object.Record arguments=(string, int32)
"#,
    );
}

#[test]
fn test_record_string_key_constraint_is_indexed() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare const bag: Bag;

bag["missing"] satisfies int32 | undefined;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare const bag: Bag;

bag["missing"] satisfies int32 | undefined;

=== checked ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type=Record<string, int32> reduced={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32> reduced={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag reduced={ [P: string]: int32 }
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

bag["missing"] satisfies int32 | undefined;
/// @resolution.name source=bag target=bag
/// @resolution.member source="bag[\"missing\"]" receiver={ [P: string]: int32 } kind=index key=string

/// @generic.instance id="Record<string, int32>" template=types.object.Record arguments=(string, int32)
"#,
    );
}

#[test]
fn test_record_string_key_constraint_accepts_map() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare const map: Map<string, int32>;
const bag: Bag = map;
const value = bag["missing"];

value satisfies int32 | undefined;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare const map: Map<string, int32>;
const bag: Bag = map;
const value: int32 | undefined = bag["missing"];

value satisfies int32 | undefined;

=== checked ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type=Record<string, int32> reduced={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32> reduced={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare const map: Map<string, int32>;
/// @type.symbol symbol=map source=map type=Map<string, int32>
/// @resolution.pattern source=map kind=binding target=map
/// @resolution.name source=Map target=collections.map.Map

const bag: Bag = map;
/// @type.symbol symbol=bag source=bag type=Bag reduced={ [P: string]: int32 }
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=map target=map

const value = bag["missing"];
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=bag target=bag
/// @resolution.member source="bag[\"missing\"]" receiver={ [P: string]: int32 } kind=index key=string

value satisfies int32 | undefined;
/// @resolution.name source=value target=value

/// @generic.instance id="Map<string, int32>" template=collections.map.Map arguments=(string, int32)
/// @generic.instance id="Record<string, int32>" template=types.object.Record arguments=(string, int32)
"#,
    );
}

#[test]
fn test_record_never_key_yields_empty_object() {
    let session = TestSession::single(
        r#"
type Empty = Record<never, boolean>;

const empty: Empty = {};
empty satisfies Empty;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Empty = Record<never, boolean>;

const empty: Empty = {};
empty satisfies Empty;

=== checked ===
type Empty = Record<never, boolean>;
/// @type.symbol symbol=Empty source="type Empty = Record<never, boolean>" type=Record<never, boolean> reduced={}
/// @definition.type symbol=Empty source="type Empty = Record<never, boolean>" value=Record<never, boolean> reduced={}
/// @resolution.name source=Record target=types.object.Record

const empty: Empty = {};
/// @type.symbol symbol=empty source=empty type=Empty reduced={}
/// @resolution.pattern source=empty kind=binding target=empty
/// @resolution.name source=Empty target=Empty

empty satisfies Empty;
/// @resolution.name source=empty target=empty
/// @resolution.name source=Empty target=Empty

/// @generic.instance id="Record<never, boolean>" template=types.object.Record arguments=(never, boolean)
"#,
    );
}

#[test]
fn test_record_never_key_rejects_extra_field() {
    let session = TestSession::single(
        r#"
type Empty = Record<never, boolean>;

const empty: Empty = { value: true };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Empty = Record<never, boolean>;

const empty: Empty = { value: true };

=== checked ===
type Empty = Record<never, boolean>;
/// @type.symbol symbol=Empty source="type Empty = Record<never, boolean>" type=Record<never, boolean> reduced={}
/// @definition.type symbol=Empty source="type Empty = Record<never, boolean>" value=Record<never, boolean> reduced={}
/// @resolution.name source=Record target=types.object.Record

const empty: Empty = { value: true };
/// @type.symbol symbol=empty source=empty type=Empty reduced={}
/// @resolution.pattern source=empty kind=binding target=empty
/// @resolution.name source=Empty target=Empty

/// @generic.instance id="Record<never, boolean>" template=types.object.Record arguments=(never, boolean)
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'value' in object literal for type 'Empty'"
/// @diagnostic.label line=4 column=22 span="{ value: true }" line_source="const empty: Empty = { value: true };"
/// @diagnostic.related line=4 column=14 span="Empty" line_source="const empty: Empty = { value: true };" message="expected due to this annotation"
/// @diagnostic.note message="object literals may only specify known properties"
"#,
    );
}
