use crate::tests::{DirRows, TestSession};

/// A Record over a finite key union builds one field per key.
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true, b: false };

flags.a satisfies boolean;
flags.b satisfies boolean;

=== dir ===
type Flags = Record<"a" | "b", boolean>;
/// @type.symbol symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" type={ a: boolean; b: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" value=Record<"a" | "b", boolean>
/// @resolution.name source=Record target=Record

const flags: Flags = { a: true, b: false };
/// @type.symbol symbol=flags source=flags type=Flags
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags

flags.a satisfies boolean;
/// @resolution.name source=flags target=flags
/// @resolution.member source=flags.a receiver=Flags type=boolean kind=field target_receiver=Flags key=a target_type=boolean
/// @resolution.place source=flags placement="local" lifetime="static" access="immutable"
/// @resolution.access source=flags root=flags
/// @resolution.place source=flags.a placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=flags.a root=flags keys=[a]

flags.b satisfies boolean;
/// @resolution.name source=flags target=flags
/// @resolution.member source=flags.b receiver=Flags type=boolean kind=field target_receiver=Flags key=b target_type=boolean
/// @resolution.place source=flags placement="local" lifetime="static" access="immutable"
/// @resolution.access source=flags root=flags
/// @resolution.place source=flags.b placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=flags.b root=flags keys=[b]
"#,
    );
}

/// A Record over usize literal keys builds one field per key.
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags = Record<1 | 2, string>;

const flags: Flags = { 1: "one", 2: "two" };

flags[1] satisfies string;
flags[2] satisfies string;

=== dir ===
type Flags = Record<1 | 2, string>;
/// @type.symbol symbol=Flags source="type Flags = Record<1 | 2, string>" type={ 1: string; 2: string }
/// @definition.type symbol=Flags source="type Flags = Record<1 | 2, string>" value=Record<1 | 2, string>
/// @resolution.name source=Record target=Record

const flags: Flags = { 1: "one", 2: "two" };
/// @type.symbol symbol=flags source=flags type=Flags
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags

flags[1] satisfies string;
/// @resolution.name source=flags target=flags
/// @resolution.place source=flags placement="local" lifetime="static" access="immutable"
/// @resolution.access source=flags root=flags
/// @resolution.place source=flags[1] placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=flags[1] root=flags keys=[1]
/// @resolution.subscript source=flags[1] type=string kind=member target="receiver=Flags, target=field(receiver=Flags, target=1, type=string), type=string"

flags[2] satisfies string;
/// @resolution.name source=flags target=flags
/// @resolution.place source=flags placement="local" lifetime="static" access="immutable"
/// @resolution.access source=flags root=flags
/// @resolution.place source=flags[2] placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=flags[2] root=flags keys=[2]
/// @resolution.subscript source=flags[2] type=string kind=member target="receiver=Flags, target=field(receiver=Flags, target=2, type=string), type=string"
"#,
    );
}

/// A literal missing one Record key reports a diagnostic.
#[test]
fn test_record_finite_key_union_rejects_missing_key() {
    let session = TestSession::single(
        r#"
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true };

=== dir ===
type Flags = Record<"a" | "b", boolean>;
/// @type.symbol symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" type={ a: boolean; b: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" value=Record<"a" | "b", boolean>
/// @resolution.name source=Record target=Record

const flags: Flags = { a: true };
/// @type.symbol symbol=flags source=flags type=Flags
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags
"#,
        r#"
/// @diagnostic.error id=missing-required-property message="missing required property 'b' for type 'Flags'"
/// @diagnostic.label line=4 column=22 span="{ a: true }" line_source="const flags: Flags = { a: true };"
/// @diagnostic.related line=4 column=14 span="Flags" line_source="const flags: Flags = { a: true };" message="expected due to this annotation"
"#,
    );
}

/// A literal missing one usize Record key reports a diagnostic.
#[test]
fn test_record_usize_literal_keys_reject_missing_key() {
    let session = TestSession::single(
        r#"
type Flags = Record<1 | 2, string>;

const flags: Flags = { 1: "one" };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags = Record<1 | 2, string>;

const flags: Flags = { 1: "one" };

=== dir ===
type Flags = Record<1 | 2, string>;
/// @type.symbol symbol=Flags source="type Flags = Record<1 | 2, string>" type={ 1: string; 2: string }
/// @definition.type symbol=Flags source="type Flags = Record<1 | 2, string>" value=Record<1 | 2, string>
/// @resolution.name source=Record target=Record

const flags: Flags = { 1: "one" };
/// @type.symbol symbol=flags source=flags type=Flags
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags
"#,
        r#"
/// @diagnostic.error id=missing-required-property message="missing required property '2' for type 'Flags'"
/// @diagnostic.label line=4 column=22 span="{ 1: \"one\" }" line_source="const flags: Flags = { 1: \"one\" };"
/// @diagnostic.related line=4 column=14 span="Flags" line_source="const flags: Flags = { 1: \"one\" };" message="expected due to this annotation"
"#,
    );
}

/// A literal with a key outside the Record union reports a diagnostic.
#[test]
fn test_record_finite_key_union_rejects_extra_key() {
    let session = TestSession::single(
        r#"
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true, b: false, c: true };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true, b: false, c: true };

=== dir ===
type Flags = Record<"a" | "b", boolean>;
/// @type.symbol symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" type={ a: boolean; b: boolean }
/// @definition.type symbol=Flags source="type Flags = Record<\"a\" | \"b\", boolean>" value=Record<"a" | "b", boolean>
/// @resolution.name source=Record target=Record

const flags: Flags = { a: true, b: false, c: true };
/// @type.symbol symbol=flags source=flags type=Flags
/// @resolution.pattern source=flags kind=binding target=flags
/// @resolution.name source=Flags target=Flags
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'c' in object literal for type 'Flags'"
/// @diagnostic.label line=4 column=22 span="{ a: true, b: false, c: true }" line_source="const flags: Flags = { a: true, b: false, c: true };"
/// @diagnostic.related line=4 column=14 span="Flags" line_source="const flags: Flags = { a: true, b: false, c: true };" message="expected due to this annotation"
/// @diagnostic.note message="object literals may only specify known properties"
"#,
    );
}

/// A Record over an object key type reports a diagnostic.
#[test]
fn test_record_rejects_invalid_key_type() {
    let session = TestSession::single(
        r#"
type Bad = Record<{ name: string }, boolean>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bad = Record<{ name: string }, boolean>;

=== dir ===
type Bad = Record<{ name: string }, boolean>;
/// @type.symbol symbol=Bad source="type Bad = Record<{ name: string }, boolean>" type={ [P in { name: string }]: boolean }
/// @definition.type symbol=Bad source="type Bad = Record<{ name: string }, boolean>" value=Record<{ name: string }, boolean>
/// @resolution.name source=Record target=Record
/// @type.symbol symbol=Bad.name source="name: string" type=string
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type '{ name: string }' does not satisfy 'PropertyKey'"
/// @diagnostic.label line=2 column=19 span="{ name: string }" line_source="type Bad = Record<{ name: string }, boolean>;"
/// @diagnostic.related file="object.ds" line=7 column=20 span="K" line_source="export type Record<K: PropertyKey, V> = {" message="required by this bound on 'K'"
/// @diagnostic.note message="'PropertyKey' reduces to 'string | usize'"
"#,
    );
}

/// A Record over string keys accepts a finite object and widens misses to undefined.
#[test]
fn test_record_string_key_constraint_accepts_finite_object() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare function read(bag: Bag): int32 | undefined;

const point: { x: int32 } = { x: 1 };
const value = read(point);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare function read(bag: Bag): int32 | undefined;

const point: { x: int32 } = { x: 1 };
const value: int32 | undefined = read(point as Bag);

=== dir ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32>
/// @resolution.name source=Record target=Record

declare function read(bag: Bag): int32 | undefined;
/// @type.symbol symbol=read source="declare function read(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
/// @resolution.name source=Bag target=Bag

const point: { x: int32 } = { x: 1 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x source="x: int32" type=int32

const value = read(point);
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=read target=read
/// @resolution.call source=read(point) parameters=(Bag) arguments=(provided(point) as Bag) return=int32 | undefined kind=symbol target=read
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
"#,
        r#"

"#,
    );
}

/// A Record over string keys accepts an object literal argument.
#[test]
fn test_record_string_key_constraint_accepts_object_literal() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare function read(bag: Bag): int32 | undefined;

const value = read({ x: 1, y: 2 });
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare function read(bag: Bag): int32 | undefined;

const value: int32 | undefined = read({ x: 1, y: 2 } as Bag);

=== dir ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32>
/// @resolution.name source=Record target=Record

declare function read(bag: Bag): int32 | undefined;
/// @type.symbol symbol=read source="declare function read(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
/// @resolution.name source=Bag target=Bag

const value = read({ x: 1, y: 2 });
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=read target=read
/// @resolution.call source="read({ x: 1, y: 2 })" parameters=(Bag) arguments=(provided({ x: 1, y: 2 }) as Bag) return=int32 | undefined kind=symbol target=read
"#,
    );
}

/// A Record over string keys reads through bracket access and widens misses to undefined.
#[test]
fn test_record_string_key_constraint_is_indexed() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare const bag: Bag;

bag["missing"] satisfies int32 | undefined;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare const bag: Bag;

bag["missing"] satisfies int32 | undefined;

=== dir ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32>
/// @resolution.name source=Record target=Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

bag["missing"] satisfies int32 | undefined;
/// @resolution.name source=bag target=bag
/// @resolution.place source="bag[\"missing\"]" placement="local" lifetime="managed" access="mutable"
/// @resolution.access source="bag[\"missing\"]" root=bag keys=[missing]
/// @resolution.subscript source="bag[\"missing\"]" type=int32 | undefined kind=member target="receiver=Bag, target=index(string), type=int32 | undefined"
/// @resolution.place source=bag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bag root=bag
"#,
    );
}

/// A Record over string keys accepts a Map.
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare const map: Map<string, int32, Equality<string>>;
const bag: Bag = map as Bag;
const value: int32 | undefined = bag["missing"];

value satisfies int32 | undefined;

=== dir ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32>
/// @resolution.name source=Record target=Record

declare const map: Map<string, int32>;
/// @type.symbol symbol=map source=map type=Map<string, int32, Equality<string>>
/// @resolution.pattern source=map kind=binding target=map
/// @generic.instance id="Map<string, int32, Equality<string>>" template=Map arguments=(string, int32, Equality<string>)
/// @generic.instance id="sliceAssumeInit<MaybeUninit<MapSlot<string, int32>>>" template=sliceAssumeInit arguments=(MaybeUninit<MapSlot<string, int32>>)
/// @generic.instance id="sliceUninit<MaybeUninit<MapSlot<string, int32>>>" template=sliceUninit arguments=(MaybeUninit<MapSlot<string, int32>>)
/// @generic.instance id=Equality<string> template=Equality arguments=(string)
/// @generic.instance id=newPhantom<string> template=newPhantom arguments=(string)
/// @generic.instance id=sliceAssumeInit<uint32> template=sliceAssumeInit arguments=(uint32)
/// @generic.instance id=sliceUninit<uint32> template=sliceUninit arguments=(uint32)
/// @resolution.name source=Map target=Map

const bag: Bag = map;
/// @type.symbol symbol=bag source=bag type=Bag
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=map target=map
/// @resolution.place source=map placement="local" lifetime="static" access="immutable"
/// @resolution.access source=map root=map

const value = bag["missing"];
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=bag target=bag
/// @resolution.access source="bag[\"missing\"]" root=bag keys=[missing]
/// @resolution.subscript source="bag[\"missing\"]" type=int32 | undefined kind=member target="receiver=Bag, target=index(string), type=int32 | undefined"
/// @resolution.place source=bag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bag root=bag

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

/// A Record over never keys reduces to the empty object type.
#[test]
fn test_record_never_key_yields_empty_object() {
    let session = TestSession::single(
        r#"
type Empty = Record<never, boolean>;

const empty: Empty = {};
empty satisfies Empty;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Empty = Record<never, boolean>;

const empty: Empty = {};
empty satisfies Empty;

=== dir ===
type Empty = Record<never, boolean>;
/// @type.symbol symbol=Empty source="type Empty = Record<never, boolean>" type={}
/// @definition.type symbol=Empty source="type Empty = Record<never, boolean>" value=Record<never, boolean>
/// @resolution.name source=Record target=Record

const empty: Empty = {};
/// @type.symbol symbol=empty source=empty type=Empty
/// @resolution.pattern source=empty kind=binding target=empty
/// @resolution.name source=Empty target=Empty

empty satisfies Empty;
/// @resolution.name source=empty target=empty
/// @resolution.place source=empty placement="local" lifetime="static" access="immutable"
/// @resolution.access source=empty root=empty
/// @resolution.name source=Empty target=Empty
"#,
    );
}

/// A field on a Record over never keys reports a diagnostic.
#[test]
fn test_record_never_key_rejects_extra_field() {
    let session = TestSession::single(
        r#"
type Empty = Record<never, boolean>;

const empty: Empty = { value: true };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Empty = Record<never, boolean>;

const empty: Empty = { value: true };

=== dir ===
type Empty = Record<never, boolean>;
/// @type.symbol symbol=Empty source="type Empty = Record<never, boolean>" type={}
/// @definition.type symbol=Empty source="type Empty = Record<never, boolean>" value=Record<never, boolean>
/// @resolution.name source=Record target=Record

const empty: Empty = { value: true };
/// @type.symbol symbol=empty source=empty type=Empty
/// @resolution.pattern source=empty kind=binding target=empty
/// @resolution.name source=Empty target=Empty
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'value' in object literal for type 'Empty'"
/// @diagnostic.label line=4 column=22 span="{ value: true }" line_source="const empty: Empty = { value: true };"
/// @diagnostic.related line=4 column=14 span="Empty" line_source="const empty: Empty = { value: true };" message="expected due to this annotation"
/// @diagnostic.note message="object literals may only specify known properties"
"#,
    );
}
