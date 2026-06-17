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
/// @type.symbol symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" type={ a: boolean; b: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" value={ a: boolean; b: boolean }
/// @resolution.name source=Record target=types.object.Record

const flags: Flags = { a: true, b: false };
/// @type.symbol symbol=flags source=flags type={ a: boolean; b: boolean }
/// @resolution.name source=Flags target=Flags

flags.a satisfies boolean;
/// @resolution.name source=flags target=flags
/// @resolution.member source=flags.a receiver=types.object.Record<"a" | "b", boolean> kind=field key=a

flags.b satisfies boolean;
/// @resolution.name source=flags target=flags
/// @resolution.member source=flags.b receiver=types.object.Record<"a" | "b", boolean> kind=field key=b
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
/// @type.symbol symbol=Flags source="type Flags = Record<1 | 2, string>" type={ 1: string; 2: string }
/// @definition.type symbol=Flags source="type Flags = Record<1 | 2, string>" value={ 1: string; 2: string }
/// @resolution.name source=Record target=types.object.Record

const flags: Flags = { 1: "one", 2: "two" };
/// @type.symbol symbol=flags source=flags type={ 1: string; 2: string }
/// @resolution.name source=Flags target=Flags

flags[1] satisfies string;
/// @resolution.name source=flags target=flags
/// @resolution.member source="flags[1]" receiver=types.object.Record<1 | 2, string> kind=field key=1

flags[2] satisfies string;
/// @resolution.name source=flags target=flags
/// @resolution.member source="flags[2]" receiver=types.object.Record<1 | 2, string> kind=field key=2
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
/// @type.symbol symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" type={ a: boolean; b: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" value={ a: boolean; b: boolean }
/// @resolution.name source=Record target=types.object.Record

const flags: Flags = { a: true };
/// @type.symbol symbol=flags source=flags type={ a: boolean; b: boolean }
/// @resolution.name source=Flags target=Flags
"#,
        r#"
/// @diagnostic.error code=EC215 message="missing required property 'b' for type 'Flags'"
/// @diagnostic.label line=4 column=22 source="const flags: Flags = { a: true };"
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
/// @type.symbol symbol=Flags source="type Flags = Record<1 | 2, string>" type={ 1: string; 2: string }
/// @definition.type symbol=Flags source="type Flags = Record<1 | 2, string>" value={ 1: string; 2: string }
/// @resolution.name source=Record target=types.object.Record

const flags: Flags = { 1: "one" };
/// @type.symbol symbol=flags source=flags type={ 1: string; 2: string }
/// @resolution.name source=Flags target=Flags
"#,
        r#"
/// @diagnostic.error code=EC215 message="missing required property '2' for type 'Flags'"
/// @diagnostic.label line=4 column=22 source="const flags: Flags = { 1: \"one\" };"
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
/// @type.symbol symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" type={ a: boolean; b: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" value={ a: boolean; b: boolean }
/// @resolution.name source=Record target=types.object.Record

const flags: Flags = { a: true, b: false, c: true };
/// @type.symbol symbol=flags source=flags type={ a: boolean; b: boolean }
/// @resolution.name source=Flags target=Flags
"#,
        r#"
/// @diagnostic.error code=EC205 message="unknown property 'c' in object literal for type '{ a: boolean; b: boolean }'"
/// @diagnostic.label line=4 column=47 source="const flags: Flags = { a: true, b: false, c: true };"
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
/// @diagnostic.error code=EC201 message="type '{ name: string }' does not satisfy 'PropertyKey'"
/// @diagnostic.label line=2 column=12 source="type Bad = Record<{ name: string }, boolean>;"
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

type Flags = Record<typeof key, boolean>;
/// @type.symbol symbol=Flags source="type Flags = Record<typeof key, boolean>" type={ [key]: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<typeof key, boolean>" value={ [key]: boolean }
/// @resolution.name source=Record target=types.object.Record
/// @resolution.name source=key target=key

const flags: Flags = { [key]: true };
/// @type.symbol symbol=flags source=flags type={ [key]: boolean }
/// @resolution.name source=Flags target=Flags
/// @resolution.name source=key target=key

flags[key] satisfies boolean;
/// @resolution.name source=flags target=flags
/// @resolution.name source=key target=key
/// @resolution.member source="flags[key]" receiver=types.object.Record<key, boolean> kind=field key=key
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

declare function read<T0: Bag>(bag: T0): int32 | undefined;

const point: { x: int32 } = { x: 1 };
const value: int32 | undefined = read<{ x: int32 }>(point);

=== checked ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare function read(bag: Bag): int32 | undefined;
/// @generic.template symbol=read parameters=[T0: Bag]
/// @type.symbol symbol=read type=<read.T0: Bag>(read.T0) => int32 | undefined
/// @type.symbol symbol=bag type=read.T0
/// @resolution.name source=Bag target=Bag

const point: { x: int32 } = { x: 1 };
/// @type.symbol symbol=point source=point type={ x: int32 }

const value = read(point);
/// @type.symbol symbol=value type=int32 | undefined
/// @resolution.name source=read target=read
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC216 message="type '{ x: int32 }' is missing IndexSet<string, int32> for writable index signature"
/// @diagnostic.label line=7 column=15 source="const value = read(point);"
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
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type={ [P: string]: int32 }
/// @resolution.name source=Bag target=Bag

bag["missing"] satisfies int32 | undefined;
/// @resolution.name source=bag target=bag
/// @resolution.member source="bag[\"missing\"]" receiver={ [P: string]: int32 } kind=builtin builtin=subscript.index
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
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare const map: Map<string, int32>;
/// @type.symbol symbol=map source=map type=Map<string, int32>
/// @resolution.name source=Map target=collections.map.Map

const bag: Bag = map;
/// @type.symbol symbol=bag source=bag type={ [P: string]: int32 }
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=map target=map

const value = bag["missing"];
/// @type.symbol symbol=value type=int32 | undefined
/// @resolution.name source=bag target=bag
/// @resolution.member source="bag[\"missing\"]" receiver={ [P: string]: int32 } kind=builtin builtin=subscript.index

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
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
/// @type.symbol symbol=Empty source="type Empty = Record<never, boolean>" type={}
/// @definition.type symbol=Empty source="type Empty = Record<never, boolean>" value={}
/// @resolution.name source=Record target=types.object.Record

const empty: Empty = {};
/// @type.symbol symbol=empty source=empty type={}
/// @resolution.name source=Empty target=Empty

empty satisfies Empty;
/// @resolution.name source=empty target=empty
/// @resolution.name source=Empty target=Empty
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
/// @type.symbol symbol=Empty source="type Empty = Record<never, boolean>" type={}
/// @definition.type symbol=Empty source="type Empty = Record<never, boolean>" value={}
/// @resolution.name source=Record target=types.object.Record

const empty: Empty = { value: true };
/// @type.symbol symbol=empty source=empty type={}
/// @resolution.name source=Empty target=Empty
"#,
        r#"
/// @diagnostic.error code=EC205 message="unknown property 'value' in object literal for type '{}'"
/// @diagnostic.label line=4 column=24 source="const empty: Empty = { value: true };"
"#,
    );
}
