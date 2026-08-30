use crate::tests::{DirRows, TestSession};

/// A readonly index signature reads a finite object and widens misses to undefined.
#[test]
fn test_readonly_index_signature_views_finite_object() {
    let session = TestSession::single(
        r#"
type Bag = { readonly [key: string]: int32 };

function read(bag: Bag): int32 | undefined {
    const x = bag["x"];
    const missing = bag["missing"];
    missing satisfies int32 | undefined;
    return x;
}

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
const x = read(point);
x satisfies int32 | undefined;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { readonly [key: string]: int32 };

function read(bag: Bag): int32 | undefined {
    const x: int32 | undefined = bag["x"];
    const missing: int32 | undefined = bag["missing"];
    missing satisfies int32 | undefined;
    return x;
}

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
const x: int32 | undefined = read(point as Bag);
x satisfies int32 | undefined;

=== dir ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

function read(bag: Bag): int32 | undefined {
/// @type.symbol symbol=read type=(Bag) => int32 | undefined
/// @type.symbol symbol=read.bag source="bag: Bag" type={ readonly [key: string]: int32 }
/// @resolution.name source=Bag target=Bag

    const x = bag["x"];
    /// @type.symbol symbol=read.x source=x type=int32 | undefined
    /// @resolution.pattern source=x kind=binding target=read.x
    /// @resolution.name source=bag target=read.bag
    /// @resolution.access source="bag[\"x\"]" root=read.bag keys=[x]
    /// @resolution.subscript source="bag[\"x\"]" type=int32 | undefined kind=member target="receiver={ readonly [key: string]: int32 }, target=index(string), type=int32 | undefined"
    /// @resolution.place source=bag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=bag root=read.bag

    const missing = bag["missing"];
    /// @type.symbol symbol=read.missing source=missing type=int32 | undefined
    /// @resolution.pattern source=missing kind=binding target=read.missing
    /// @resolution.name source=bag target=read.bag
    /// @resolution.access source="bag[\"missing\"]" root=read.bag keys=[missing]
    /// @resolution.subscript source="bag[\"missing\"]" type=int32 | undefined kind=member target="receiver={ readonly [key: string]: int32 }, target=index(string), type=int32 | undefined"
    /// @resolution.place source=bag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=bag root=read.bag

    missing satisfies int32 | undefined;
    /// @resolution.name source=missing target=read.missing
    /// @resolution.place source=missing placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=missing root=read.missing

    return x;
    /// @resolution.name source=x target=read.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=x root=read.x

}

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x#1 source="x: int32" type=int32
/// @type.symbol symbol=y source="y: int32" type=int32

const x = read(point);
/// @type.symbol symbol=x#2 source=x type=int32 | undefined
/// @resolution.pattern source=x kind=binding target=x#2
/// @resolution.name source=read target=read
/// @resolution.call source=read(point) parameters=(Bag) arguments=(provided(point) as Bag) return=int32 | undefined kind=symbol target=read
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point

x satisfies int32 | undefined;
/// @resolution.name source=x target=x#2
/// @resolution.place source=x placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=x root=x#2
"#,
    );
}

/// A finite object with a field of another type reports a diagnostic at an index signature.
#[test]
fn test_readonly_index_signature_rejects_incompatible_field() {
    let session = TestSession::single(
        r#"
type Bag = { readonly [key: string]: int32 };

declare function read(bag: Bag): int32 | undefined;

const mixed: { x: int32; y: string } = { x: 1, y: "two" };
const value = read(mixed);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { readonly [key: string]: int32 };

declare function read(bag: Bag): int32 | undefined;

const mixed: { x: int32; y: string } = { x: 1, y: "two" };
const value: int32 | undefined = read(mixed);

=== dir ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

declare function read(bag: Bag): int32 | undefined;
/// @type.symbol symbol=read source="declare function read(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
/// @type.symbol symbol=read.bag source="bag: Bag" type={ readonly [key: string]: int32 }
/// @resolution.name source=Bag target=Bag

const mixed: { x: int32; y: string } = { x: 1, y: "two" };
/// @type.symbol symbol=mixed source=mixed type={ x: int32; y: string }
/// @resolution.pattern source=mixed kind=binding target=mixed
/// @type.symbol symbol=x source="x: int32" type=int32
/// @type.symbol symbol=y source="y: string" type=string

const value = read(mixed);
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=read target=read
/// @resolution.call source=read(mixed) parameters=(Bag) arguments=(provided(mixed) as Bag) return=int32 | undefined kind=symbol target=read
/// @resolution.name source=mixed target=mixed
/// @resolution.place source=mixed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=mixed root=mixed
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '{ x: int32; y: string }' is not assignable to parameter of type 'Bag'"
/// @diagnostic.label line=7 column=20 span="mixed" line_source="const value = read(mixed);"
/// @diagnostic.related line=7 column=15 span="read(mixed)" line_source="const value = read(mixed);" message="in this call"
/// @diagnostic.note message="'Bag' reduces to '{ readonly [key: string]: int32 }'"
"#,
    );
}

/// A satisfies check keeps the object type its expression writes.
#[test]
fn test_satisfies_keeps_the_written_object_type() {
    let session = TestSession::single(
        r#"
type Bag = { readonly [key: string]: int32 };

const checked = { x: 1 } satisfies Bag;
const missing = checked["missing"];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { readonly [key: string]: int32 };

const checked: { x: int32 } = { x: 1 } satisfies Bag;
const missing = checked["missing"];

=== dir ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

const checked = { x: 1 } satisfies Bag;
/// @type.symbol symbol=checked source=checked type={ x: int32 }
/// @resolution.pattern source=checked kind=binding target=checked
/// @resolution.name source=Bag target=Bag

const missing = checked["missing"];
/// @type.symbol symbol=missing source=missing type=<error>
/// @resolution.pattern source=missing kind=binding target=missing
/// @resolution.name source=checked target=checked
/// @resolution.rejected source="checked[\"missing\"]"
/// @resolution.place source=checked placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=checked root=checked
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '[]' is not defined for '{ x: int32 }' and '\"missing\"'"
/// @diagnostic.label line=5 column=24 span="[" line_source="const missing = checked[\"missing\"];"
"#,
    );
}

/// A writable index signature accepts a finite object whose fields it covers.
#[test]
fn test_writable_index_signature_accepts_covered_finite_object() {
    let session = TestSession::single(
        r#"
type Bag = { [key: string]: int32 };

declare function write(bag: Bag): int32 | undefined;

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
const bad = write(point);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { [key: string]: int32 };

declare function write(bag: Bag): int32 | undefined;

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
const bad: int32 | undefined = write(point as Bag);

=== dir ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

declare function write(bag: Bag): int32 | undefined;
/// @type.symbol symbol=write source="declare function write(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
/// @type.symbol symbol=write.bag source="bag: Bag" type={ [key: string]: int32 }
/// @resolution.name source=Bag target=Bag

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x source="x: int32" type=int32
/// @type.symbol symbol=y source="y: int32" type=int32

const bad = write(point);
/// @type.symbol symbol=bad source=bad type=int32 | undefined
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=write target=write
/// @resolution.call source=write(point) parameters=(Bag) arguments=(provided(point) as Bag) return=int32 | undefined kind=symbol target=write
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
"#,
        r#"
"#,
    );
}

/// Assigning a computed key through a structural index signature reports a diagnostic.
#[test]
fn test_writable_index_signature_rejects_keyed_write() {
    let session = TestSession::single(
        r#"
type Bag = { [key: string]: int32 };

function write(bag: Bag): int32 | undefined {
    bag["x"] = 1;
    return bag["x"];
}

declare const map: Map<string, int32>;
const value = write(map);

value satisfies int32 | undefined;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { [key: string]: int32 };

function write(bag: Bag): int32 | undefined {
    bag["x"] = 1;
    return bag["x"];
}

declare const map: Map<string, int32>;
const value: int32 | undefined = write(map as Bag);

value satisfies int32 | undefined;

=== dir ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

function write(bag: Bag): int32 | undefined {
/// @type.symbol symbol=write type=(Bag) => int32 | undefined
/// @type.symbol symbol=write.bag source="bag: Bag" type={ [key: string]: int32 }
/// @resolution.name source=Bag target=Bag

    bag["x"] = 1;
    /// @resolution.name source=bag target=write.bag
    /// @resolution.pattern.assign source="bag[\"x\"]" kind=place
    /// @resolution.access source="bag[\"x\"]" root=write.bag keys=[x]
    /// @resolution.assignment source="bag[\"x\"]" write="member(receiver={ [key: string]: int32 }, target=index(string), type=int32)" type=int32
    /// @resolution.place source=bag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=bag root=write.bag

    return bag["x"];
    /// @resolution.name source=bag target=write.bag
    /// @resolution.place source="bag[\"x\"]" placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source="bag[\"x\"]" root=write.bag keys=[x]
    /// @resolution.subscript source="bag[\"x\"]" type=int32 | undefined kind=member target="receiver={ [key: string]: int32 }, target=index(string), type=int32 | undefined"
    /// @resolution.place source=bag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=bag root=write.bag

}

declare const map: Map<string, int32>;
/// @type.symbol symbol=map source=map type=Map<string, int32>
/// @resolution.pattern source=map kind=binding target=map
/// @resolution.name source=Map target=Map

const value = write(map);
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=write target=write
/// @resolution.call source=write(map) parameters=(Bag) arguments=(provided(map) as Bag) return=int32 | undefined kind=symbol target=write
/// @resolution.name source=map target=map
/// @resolution.place source=map placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=map root=map

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.error id=cannot-assign-structural-index message="cannot assign a computed key through the structural type '{ [key: string]: int32 }', type the receiver as an IndexSet implementer like Map"
/// @diagnostic.label line=5 column=8 span="[" line_source="bag[\"x\"] = 1;"
"#,
    );
}

/// A writable index signature accepts a type implementing the index operators.
#[test]
fn test_writable_index_signature_accepts_operator_implementation() {
    let session = TestSession::single(
        r#"
type Bag = { [key: string]: int32 };

struct Store {
    storage: Map<string, int32>;
}

extension of Store implements Index<string>, IndexSet<string, int32> {
    type Output = int32;

    type Missing = undefined;

    index<const R: Region>(this: Borrowed<this, R, "readonly">, key: string): Borrowed<int32, R, "readonly"> | undefined {
        return this.storage[key];
    }

    indexSet(&exclusive this, key: string, value: int32): void {
        this.storage[key] = value;
    }
}

declare let store: Store;
declare function write(bag: Bag): int32 | undefined;

const value = write(store);

value satisfies int32 | undefined;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { [key: string]: int32 };

struct Store {
    storage: Map<string, int32>;
}

extension of Store implements Index<string>, IndexSet<string, int32> {
    type Output = int32;

    type Missing = undefined;

    index<const R: Region>(
        this: Borrowed<this, R, "readonly">,
        key: string,
    ): Borrowed<int32, R, "readonly"> | undefined {
        return this.storage[key] as Borrowed<int32, R, "readonly"> | undefined;
    }

    indexSet(&exclusive this, key: string, value: int32): void {
        this.storage[key] = value;
    }
}

declare let store: Store;
declare function write(bag: Bag): int32 | undefined;

const value: int32 | undefined = write(store as Bag);

value satisfies int32 | undefined;

=== dir ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

struct Store {
/// @type.symbol symbol=Store type=Store
/// @definition.struct symbol=Store
/// @definition.field symbol=Store.storage source="storage: Map<string, int32>" key=storage type=Map<string, int32>

    storage: Map<string, int32>;
    /// @type.symbol symbol=Store.storage source="storage: Map<string, int32>" type=Map<string, int32>
    /// @generic.instance id="Map<string, int32>" template=Map arguments=(string, int32)
    /// @generic.instance id="MapEntry<string, int32>" template=MapEntry arguments=(string, int32)
    /// @generic.instance id="MapSlot<string, int32>" template=MapSlot arguments=(string, int32)
    /// @generic.instance id="MaybeUninit<MapSlot<string, int32>>" template=MaybeUninit arguments=(MapSlot<string, int32>)
    /// @generic.instance id="new<MaybeUninit<MapSlot<string, int32>>>" template=new arguments=(MaybeUninit<MapSlot<string, int32>>)
    /// @generic.instance id=new<uint32> template=new arguments=(uint32)
    /// @resolution.name source=Map target=Map

}

extension of Store implements Index<string>, IndexSet<string, int32> {
/// @generic.instance id="Index<string, \"readonly\">" template=Index arguments=(string, "readonly")
/// @generic.instance id="IndexSet<string, int32>" template=IndexSet arguments=(string, int32)
/// @definition.extension symbol=<module>#2 form=local target=Store
/// @definition.implements symbol=<module>#2 source="IndexSet<string, int32>" target="IndexSet<string, int32>"
/// @definition.implements symbol=<module>#2 source=Index<string> target="Index<string, \"readonly\">"
/// @definition.associated.type symbol=Missing source="type Missing = undefined" key=Missing value=undefined
/// @definition.associated.type symbol=Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=index slot=index type=<const R>(this: Borrowed<Store, R, "readonly">, string) => Borrowed<int32, R, "readonly"> | undefined
/// @definition.method symbol=indexSet slot=indexSet type=<indexSet.'a>(this: &indexSet.'a exclusive Store, string, int32) => void
/// @definition.conformance symbol=<module>#2 member=Missing requirement=Index.Missing
/// @definition.conformance symbol=<module>#2 member=Output requirement=Index.Output
/// @definition.conformance symbol=<module>#2 member=index requirement=Index.index
/// @definition.conformance symbol=<module>#2 member=indexSet requirement=IndexSet.indexSet
/// @resolution.name source=Store target=Store
/// @resolution.name source=Index target=Index
/// @resolution.name source=IndexSet target=IndexSet

    type Output = int32;
    /// @type.symbol symbol=Output source="type Output = int32" type=int32

    type Missing = undefined;
    /// @type.symbol symbol=Missing source="type Missing = undefined" type=undefined

    index<const R: Region>(this: Borrowed<this, R, "readonly">, key: string): Borrowed<int32, R, "readonly"> | undefined {
    /// @generic.template symbol=index parent=template#0 parameters=(const R: Region)
    /// @type.symbol symbol=index type=<const R>(this: Borrowed<Store, R, "readonly">, string) => Borrowed<int32, R, "readonly"> | undefined
    /// @type.symbol symbol=index.R source="const R: Region" type=R
    /// @resolution.name source=Region target=Region
    /// @type.symbol symbol=index.this source="this: Borrowed<this, R, \"readonly\">" type=Borrowed<this, R, "readonly">
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=R target=index.R
    /// @type.symbol symbol=index.key source="key: string" type=string
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=R target=index.R

        return this.storage[key];
        /// @resolution.member source=this.storage receiver=Borrowed<Store, R, "readonly"> type=Readonly<Map<string, int32>> kind=field target_receiver=Borrowed<Store, R, "readonly"> key=storage target=Store.storage target_type=Readonly<Map<string, int32>>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Store, R, "readonly">
        /// @resolution.place source=this placement=R lifetime=R access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.storage placement=R lifetime=R access="readonly"
        /// @resolution.access source=this.storage root=this keys=[storage]
        /// @resolution.subscript source=this.storage[key] type=int32 | undefined kind=call target="index(parameters=(Managed<string, R>), arguments=(provided(key) as Managed<string, R>), return=WithAccess<Borrowed<int32, R, \"mutable\">, \"readonly\"> | undefined)"
        /// @generic.instantiation id="index<string, int32, \"readonly\">" template=index arguments=(string, int32, "readonly")
        /// @generic.instance id="WithAccess<&'frame Map<string, int32>, \"readonly\">" template=WithAccess arguments=(&'frame Map<string, int32>, "readonly")
        /// @generic.instance id="WithAccess<&'frame int32, \"readonly\">" template=WithAccess arguments=(&'frame int32, "readonly")
        /// @generic.instance id="index<string, int32, \"readonly\">" template=index arguments=(string, int32, "readonly")
        /// @resolution.name source=key target=index.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=index.key

    }

    indexSet(&exclusive this, key: string, value: int32): void {
    /// @generic.template symbol=indexSet parent=template#0 parameters=('a)
    /// @type.symbol symbol=indexSet type=<indexSet.'a>(this: &indexSet.'a exclusive Store, string, int32) => void
    /// @type.symbol symbol=indexSet.this source="&exclusive this" type=&indexSet.'a exclusive this
    /// @type.symbol symbol=indexSet.key source="key: string" type=string
    /// @type.symbol symbol=indexSet.value source="value: int32" type=int32

        this.storage[key] = value;
        /// @resolution.member source=this.storage receiver=&indexSet.'a exclusive Store type=Map<string, int32> kind=field target_receiver=&indexSet.'a exclusive Store key=storage target=Store.storage target_type=Map<string, int32>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&indexSet.'a exclusive Store
        /// @resolution.place source=this placement=indexSet.'a lifetime=indexSet.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.storage placement=indexSet.'a lifetime=indexSet.'a access="exclusive"
        /// @resolution.access source=this.storage root=this keys=[storage]
        /// @resolution.pattern.assign source=this.storage[key] kind=place
        /// @resolution.assignment source=this.storage[key] write="indexSet(parameters=(Managed<string, indexSet.'a>, int32), arguments=(provided(key) as Managed<string, indexSet.'a>, write as int32), return=void)" type=int32
        /// @generic.instantiation id="indexSet<string, int32>" template=indexSet arguments=(string, int32)
        /// @generic.instance id="indexSet<string, int32>" template=indexSet arguments=(string, int32)
        /// @generic.instance id="set#2<string, int32>" template=set#2 arguments=(string, int32)
        /// @resolution.name source=key target=indexSet.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=indexSet.key
        /// @resolution.name source=value target=indexSet.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=indexSet.value

    }
}

declare let store: Store;
/// @type.symbol symbol=store source=store type=Store
/// @resolution.pattern source=store kind=binding target=store
/// @resolution.name source=Store target=Store

declare function write(bag: Bag): int32 | undefined;
/// @type.symbol symbol=write source="declare function write(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
/// @type.symbol symbol=write.bag source="bag: Bag" type={ [key: string]: int32 }
/// @resolution.name source=Bag target=Bag

const value = write(store);
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=write target=write
/// @resolution.call source=write(store) parameters=(Bag) arguments=(provided(store) as Bag) return=int32 | undefined kind=symbol target=write
/// @resolution.name source=store target=store
/// @resolution.place source=store placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=store root=store

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=value root=value
"#,
    );
}

/// A Record parameter accepts a finite object with matching fixed keys.
#[test]
fn test_record_string_keys_require_writable_index_access() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare function read(bag: Bag): int32 | undefined;

const point: { x: int32 } = { x: 1 };
const bad = read(point);
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
const bad: int32 | undefined = read(point as Bag);

=== dir ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value={ [P: string]: int32 }
/// @resolution.name source=Record target=Record

declare function read(bag: Bag): int32 | undefined;
/// @type.symbol symbol=read source="declare function read(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
/// @type.symbol symbol=read.bag source="bag: Bag" type={ [P: string]: int32 }
/// @resolution.name source=Bag target=Bag

const point: { x: int32 } = { x: 1 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x source="x: int32" type=int32

const bad = read(point);
/// @type.symbol symbol=bad source=bad type=int32 | undefined
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=read target=read
/// @resolution.call source=read(point) parameters=(Bag) arguments=(provided(point) as Bag) return=int32 | undefined kind=symbol target=read
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
"#,
        r#"

"#,
    );
}

/// A Record with string keys reads through bracket access.
#[test]
fn test_record_string_keys_use_bracket_access() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare const bag: Bag;
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

declare const bag: Bag;
const value: int32 | undefined = bag["missing"];

value satisfies int32 | undefined;

=== dir ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value={ [P: string]: int32 }
/// @resolution.name source=Record target=Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type={ [P: string]: int32 }
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

const value = bag["missing"];
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=bag target=bag
/// @resolution.access source="bag[\"missing\"]" root=bag keys=[missing]
/// @resolution.subscript source="bag[\"missing\"]" type=int32 | undefined kind=member target="receiver={ [P: string]: int32 }, target=index(string), type=int32 | undefined"
/// @resolution.place source=bag placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bag root=bag

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=value root=value
"#,
    );
}

/// A Record with usize keys reads through bracket access.
#[test]
fn test_record_usize_keys_use_bracket_access() {
    let session = TestSession::single(
        r#"
type Bag = Record<usize, int32>;

declare const bag: Bag;
const value = bag[1];

value satisfies int32 | undefined;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<usize, int32>;

declare const bag: Bag;
const value: int32 | undefined = bag[1];

value satisfies int32 | undefined;

=== dir ===
type Bag = Record<usize, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<usize, int32>" type={ [P: usize]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<usize, int32>" value={ [P: usize]: int32 }
/// @resolution.name source=Record target=Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type={ [P: usize]: int32 }
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

const value = bag[1];
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=bag target=bag
/// @resolution.place source=bag placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bag root=bag
/// @resolution.access source=bag[1] root=bag keys=[1]
/// @resolution.subscript source=bag[1] type=int32 | undefined kind=member target="receiver={ [P: usize]: int32 }, target=index(usize), type=int32 | undefined"

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=value root=value
"#,
    );
}

/// A Record with string keys reports a diagnostic at dot access.
#[test]
fn test_record_string_keys_reject_dot_access() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare const bag: Bag;
const value = bag.missing;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare const bag: Bag;
const value = bag.missing;

=== dir ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value={ [P: string]: int32 }
/// @resolution.name source=Record target=Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type={ [P: string]: int32 }
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

const value = bag.missing;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=bag target=bag
/// @resolution.place source=bag placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bag root=bag
/// @resolution.rejected source=bag.missing
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'missing' does not exist on type '{ [P: string]: int32 }'"
/// @diagnostic.label line=5 column=19 span="missing" line_source="const value = bag.missing;"
"#,
    );
}

/// An index signature on a generic interface keeps its key domain.
#[test]
fn test_interface_index_signature_carries_its_key_domain() {
    let session = TestSession::single(
        r#"
interface Bag<T> {
    [key: string]: T;
}

declare const bag: Bag<int32>;
const value = bag["name"];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Bag<in out T> {
    [key: string]: T;
}

declare const bag: Bag<int32>;
const value: int32 | undefined = bag["name"];

=== dir ===
interface Bag<T> {
/// @generic.template symbol=Bag parameters=(in out T)
/// @type.symbol symbol=Bag type=Bag
/// @definition.interface symbol=Bag template=(in out T)
/// @definition.where symbol=Bag relation=satisfies left=this right=Bag<T>
/// @definition.signature kind=index source="[key: string]: T" key=string type=T
/// @type.symbol symbol=Bag.T source=T type=T

    [key: string]: T;
    /// @resolution.name source=T target=Bag.T

}

declare const bag: Bag<int32>;
/// @type.symbol symbol=bag source=bag type=Bag<int32>
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

const value = bag["name"];
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="bag[\"name\"]" type=int32 | undefined
/// @type.node source=bag type=Bag<int32>
/// @resolution.name source=bag target=bag
/// @resolution.subscript source="bag[\"name\"]" type=int32 | undefined kind=call target="dynamic(Bag<int32> as Bag<int32>, index.read([key: string]: T))(parameters=(string), arguments=(provided(\"name\") as string), return=int32 | undefined)"
/// @resolution.place source=bag placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bag root=bag
/// @type.node source="\"name\"" type="name"
"#,
        r#"
"#,
    );
}

/// An object type satisfies an interface declaring a matching index signature.
#[test]
fn test_object_satisfies_interface_index_signature() {
    let session = TestSession::single(
        r#"
interface Bag {
    readonly [key: string]: int32;
}

declare const values: { readonly [key: string]: int32 };
values satisfies Bag;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Bag {
    readonly [key: string]: int32;
}

declare const values: { readonly [key: string]: int32 };
values satisfies Bag;

=== dir ===
interface Bag {
/// @type.symbol symbol=Bag type=Bag
/// @definition.interface symbol=Bag
/// @definition.where symbol=Bag relation=satisfies left=this right=Bag
/// @definition.signature kind=index source="readonly [key: string]: int32" key=string type=int32

    readonly [key: string]: int32;
}

declare const values: { readonly [key: string]: int32 };
/// @type.symbol symbol=values source=values type={ readonly [key: string]: int32 }
/// @resolution.pattern source=values kind=binding target=values

values satisfies Bag;
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.name source=Bag target=Bag
"#,
    );
}

/// An object type with another value type reports a diagnostic at an index signature.
#[test]
fn test_object_rejects_incompatible_interface_index_signature() {
    let session = TestSession::single(
        r#"
interface Bag {
    readonly [key: string]: int32;
}

declare const values: { readonly [key: string]: string };
values satisfies Bag;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Bag {
    readonly [key: string]: int32;
}

declare const values: { readonly [key: string]: string };
values satisfies Bag;

=== dir ===
interface Bag {
/// @type.symbol symbol=Bag type=Bag
/// @definition.interface symbol=Bag
/// @definition.where symbol=Bag relation=satisfies left=this right=Bag
/// @definition.signature kind=index source="readonly [key: string]: int32" key=string type=int32

    readonly [key: string]: int32;
}

declare const values: { readonly [key: string]: string };
/// @type.symbol symbol=values source=values type={ readonly [key: string]: string }
/// @resolution.pattern source=values kind=binding target=values

values satisfies Bag;
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.name source=Bag target=Bag
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type '{ readonly [key: string]: string }' does not satisfy 'Bag'"
/// @diagnostic.label line=7 column=1 span="values" line_source="values satisfies Bag;"
"#,
    );
}

/// A struct implementing an index signature interface must declare that signature.
#[test]
fn test_struct_implements_clause_requires_index_signature() {
    let session = TestSession::single(
        r#"
interface Bag {
    readonly [key: string]: int32;
}

struct Values implements Bag {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Bag {
    readonly [key: string]: int32;
}

struct Values implements Bag {}

=== dir ===
interface Bag {
/// @type.symbol symbol=Bag type=Bag
/// @definition.interface symbol=Bag
/// @definition.where symbol=Bag relation=satisfies left=this right=Bag
/// @definition.signature kind=index source="readonly [key: string]: int32" key=string type=int32

    readonly [key: string]: int32;
}

struct Values implements Bag {}
/// @type.symbol symbol=Values source="struct Values implements Bag {}" type=Values
/// @definition.struct symbol=Values source="struct Values implements Bag {}"
/// @definition.where symbol=Values source=Bag relation=satisfies left=this right=Bag
/// @definition.implements symbol=Values source=Bag target=Bag
/// @resolution.name source=Bag target=Bag
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'Values' does not implement interface 'Bag'"
/// @diagnostic.label line=6 column=26 span="Bag" line_source="struct Values implements Bag {}"
"#,
    );
}

/// A compound subscript assignment feeds the read result into the write type.
#[test]
fn test_subscript_update_assigns_read_result_to_write_type() {
    let session = TestSession::single(
        r#"
struct Counter {}

extension of Counter implements Index<string>, IndexSet<string, int32 | float64> {
    type Output = int32;

    index<const R: Lifetime>(
        this: Borrowed<this, R, "readonly">,
        key: string,
    ): Borrowed<this.Output, R, "readonly"> {
        return todo("Counter.index");
    }

    indexSet(&exclusive this, key: string, value: int32 | float64): void {}
}

declare let counter: Counter;
counter["value"] += 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {}

extension of Counter implements Index<string>, IndexSet<string, int32 | float64> {
    type Output = int32;

    index<const R: Lifetime>(
        this: Borrowed<this, R, "readonly">,
        key: string,
    ): Borrowed<this.Output, R, "readonly"> {
        return todo("Counter.index" as string | undefined);
    }

    indexSet(&exclusive this, key: string, value: int32 | float64): void {}
}

declare let counter: Counter;
(counter["value"] += 1) as int32 | float64;

=== dir ===
struct Counter {}
/// @type.symbol symbol=Counter source="struct Counter {}" type=Counter
/// @definition.struct symbol=Counter source="struct Counter {}"

extension of Counter implements Index<string>, IndexSet<string, int32 | float64> {
/// @generic.instance id="Index<string, \"readonly\">" template=Index arguments=(string, "readonly")
/// @generic.instance id="IndexSet<string, int32 | float64>" template=IndexSet arguments=(string, int32 | float64)
/// @definition.extension symbol=<module>#2 form=local target=Counter
/// @definition.implements symbol=<module>#2 source="IndexSet<string, int32 | float64>" target="IndexSet<string, int32 | float64>"
/// @definition.implements symbol=<module>#2 source=Index<string> target="Index<string, \"readonly\">"
/// @definition.associated.type symbol=Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=index slot=index type=<const R>(this: Borrowed<Counter, R, "readonly">, string) => Borrowed<int32, R, "readonly">
/// @definition.method symbol=indexSet source="indexSet(&exclusive this, key: string, value: int32 | float64): void {}" slot=indexSet type=<indexSet.'a>(this: &indexSet.'a exclusive Counter, string, int32 | float64) => void
/// @definition.conformance symbol=<module>#2 member=Index.Missing requirement=Index.Missing
/// @definition.conformance symbol=<module>#2 member=Output requirement=Index.Output
/// @definition.conformance symbol=<module>#2 member=index requirement=Index.index
/// @definition.conformance symbol=<module>#2 member=indexSet requirement=IndexSet.indexSet
/// @resolution.name source=Counter target=Counter
/// @resolution.name source=Index target=Index
/// @resolution.name source=IndexSet target=IndexSet

    type Output = int32;
    /// @type.symbol symbol=Output source="type Output = int32" type=int32

    index<const R: Lifetime>(
    /// @generic.template symbol=index parent=template#0 parameters=(const R: Lifetime)
    /// @type.symbol symbol=index type=<const R>(this: Borrowed<Counter, R, "readonly">, string) => Borrowed<int32, R, "readonly">
    /// @type.symbol symbol=index.R source="const R: Lifetime" type=R
    /// @resolution.name source=Lifetime target=Lifetime

        this: Borrowed<this, R, "readonly">,
        /// @type.symbol symbol=index.this source="this: Borrowed<this, R, \"readonly\">" type=Borrowed<this, R, "readonly">
        /// @resolution.name source=Borrowed target=Borrowed
        /// @resolution.name source=R target=index.R

        key: string,
        /// @type.symbol symbol=index.key source="key: string" type=string

    ): Borrowed<this.Output, R, "readonly"> {
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=R target=index.R

        return todo("Counter.index");
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Counter.index\")" parameters=(string | undefined) arguments=(provided("Counter.index") as string | undefined) return=never kind=symbol target=todo

    }

    indexSet(&exclusive this, key: string, value: int32 | float64): void {}
    /// @generic.template symbol=indexSet parent=template#0 parameters=('a)
    /// @type.symbol symbol=indexSet source="indexSet(&exclusive this, key: string, value: int32 | float64): void {}" type=<indexSet.'a>(this: &indexSet.'a exclusive Counter, string, int32 | float64) => void
    /// @type.symbol symbol=indexSet.this source="&exclusive this" type=&indexSet.'a exclusive this
    /// @type.symbol symbol=indexSet.key source="key: string" type=string
    /// @type.symbol symbol=indexSet.value source="value: int32 | float64" type=int32 | float64

}

declare let counter: Counter;
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

counter["value"] += 1;
/// @resolution.name source=counter target=counter
/// @resolution.operator source="counter[\"value\"] += 1" type=int32 operator="+" kind=builtin operands=[counter["value"] as int32 families=(integer), 1 as int32 families=(integer)]
/// @resolution.pattern.assign source="counter[\"value\"]" kind=place
/// @resolution.assignment source="counter[\"value\"]" read="index(parameters=(string), arguments=(provided(\"value\") as string), return=&'static readonly int32)" write="indexSet(parameters=(string, int32 | float64), arguments=(provided(\"value\") as string, write as int32 | float64), return=void)" type=int32 | float64
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
"#,
    );
}

/// A named property beside an index signature reports a diagnostic.
#[test]
fn test_reject_a_named_property_beside_an_index_signature() {
    let session = TestSession::single(
        r#"
type Row = { name: string; [key: string]: string };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Row = { name: string; [key: string]: string };

=== dir ===
type Row = { name: string; [key: string]: string };
/// @type.symbol symbol=Row source="type Row = { name: string; [key: string]: string }" type={ name: string; [key: string]: string }
/// @definition.type symbol=Row source="type Row = { name: string; [key: string]: string }" value={ name: string; [key: string]: string }
/// @type.symbol symbol=Row.name source="name: string" type=string
"#,
        r#"
/// @diagnostic.error id=mixed-object-type message="object type mixes named properties with an index signature"
/// @diagnostic.label line=2 column=12 span="{ name: string; [key: string]: string }" line_source="type Row = { name: string; [key: string]: string };"
"#,
    );
}

/// A named property beside a construct signature reports a diagnostic.
#[test]
fn test_reject_a_named_property_beside_a_construct_signature() {
    let session = TestSession::single(
        r#"
class Counter {}
type Factory = { name: string; new (): Counter };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {}
type Factory = { name: string; new (): Counter };

=== dir ===
class Counter {}
/// @type.symbol symbol=Counter source="class Counter {}" type=Counter
/// @definition.class symbol=Counter source="class Counter {}"

type Factory = { name: string; new (): Counter };
/// @type.symbol symbol=Factory source="type Factory = { name: string; new (): Counter }" type={ name: string; <new>: new () => Counter }
/// @definition.type symbol=Factory source="type Factory = { name: string; new (): Counter }" value={ name: string; <new>: new () => Counter }
/// @type.symbol symbol=Factory.name source="name: string" type=string
/// @resolution.name source=Counter target=Counter
"#,
        r#"
/// @diagnostic.error id=mixed-object-type message="object type mixes named properties with a construct signature"
/// @diagnostic.label line=3 column=16 span="{ name: string; new (): Counter }" line_source="type Factory = { name: string; new (): Counter };"
"#,
    );
}
