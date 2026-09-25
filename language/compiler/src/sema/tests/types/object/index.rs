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
        "main.tspp",
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
/// @type.symbol symbol=read.bag source="bag: Bag" type=Bag
/// @resolution.name source=Bag target=Bag

    const x = bag["x"];
    /// @type.symbol symbol=read.x source=x type=int32 | undefined
    /// @resolution.pattern source=x kind=binding target=read.x
    /// @resolution.name source=bag target=read.bag
    /// @resolution.access source="bag[\"x\"]" root=read.bag keys=[x]
    /// @resolution.subscript source="bag[\"x\"]" type=int32 | undefined kind=member target="receiver=Bag, target=index(string), type=int32 | undefined"
    /// @resolution.place source=bag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=bag root=read.bag

    const missing = bag["missing"];
    /// @type.symbol symbol=read.missing source=missing type=int32 | undefined
    /// @resolution.pattern source=missing kind=binding target=read.missing
    /// @resolution.name source=bag target=read.bag
    /// @resolution.access source="bag[\"missing\"]" root=read.bag keys=[missing]
    /// @resolution.subscript source="bag[\"missing\"]" type=int32 | undefined kind=member target="receiver=Bag, target=index(string), type=int32 | undefined"
    /// @resolution.place source=bag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=bag root=read.bag

    missing satisfies int32 | undefined;
    /// @resolution.name source=missing target=read.missing
    /// @resolution.place source=missing placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=missing root=read.missing

    return x;
    /// @resolution.name source=x target=read.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="immutable"
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
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point

x satisfies int32 | undefined;
/// @resolution.name source=x target=x#2
/// @resolution.place source=x placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @resolution.place source=mixed placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @resolution.place source=checked placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { [key: string]: int32 };

function write(bag: Bag): int32 | undefined {
    bag["x"] = 1;
    return bag["x"];
}

declare const map: Map<string, int32, Equality<string>>;
const value: int32 | undefined = write(map as Bag);

value satisfies int32 | undefined;

=== dir ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

function write(bag: Bag): int32 | undefined {
/// @type.symbol symbol=write type=(Bag) => int32 | undefined
/// @type.symbol symbol=write.bag source="bag: Bag" type=Bag
/// @resolution.name source=Bag target=Bag

    bag["x"] = 1;
    /// @resolution.name source=bag target=write.bag
    /// @resolution.pattern.assign source="bag[\"x\"]" kind=place
    /// @resolution.place source="bag[\"x\"]" placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source="bag[\"x\"]" root=write.bag keys=[x]
    /// @resolution.assignment source="bag[\"x\"]" write="member(receiver=Bag, target=index(string), type=int32)" type=int32
    /// @resolution.place source=bag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=bag root=write.bag

    return bag["x"];
    /// @resolution.name source=bag target=write.bag
    /// @resolution.place source="bag[\"x\"]" placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source="bag[\"x\"]" root=write.bag keys=[x]
    /// @resolution.subscript source="bag[\"x\"]" type=int32 | undefined kind=member target="receiver=Bag, target=index(string), type=int32 | undefined"
    /// @resolution.place source=bag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=bag root=write.bag

}

declare const map: Map<string, int32>;
/// @type.symbol symbol=map source=map type=Map<string, int32, Equality<string>>
/// @resolution.pattern source=map kind=binding target=map
/// @resolution.name source=Map target=Map

const value = write(map);
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=write target=write
/// @resolution.call source=write(map) parameters=(Bag) arguments=(provided(map) as Bag) return=int32 | undefined kind=symbol target=write
/// @resolution.name source=map target=map
/// @resolution.place source=map placement="local" lifetime="static" access="immutable"
/// @resolution.access source=map root=map

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
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
    type Output<const R: Region> = int32;

    type Missing = undefined;

    index<const R: Region>(this: Borrowed<this, R, "readonly">, key: string): int32 | undefined {
        return this.storage[key];
    }

    indexSet(&this, key: string, value: int32): void {
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { [key: string]: int32 };

struct Store {
    storage: Map<string, int32, Equality<string>>;
}

extension of Store implements Index<string>, IndexSet<string, int32> {
    type Output<const R: Region> = int32;

    type Missing = undefined;

    index<const R: Region>(this: Borrowed<this, R, "readonly">, key: string): int32 | undefined {
        return this.storage[key as &'managed immutable string];
    }

    indexSet(&this, key: string, value: int32): void {
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
/// @definition.field symbol=Store.storage source="storage: Map<string, int32>" key=storage type=Map<string, int32, Equality<string>>

    storage: Map<string, int32>;
    /// @type.symbol symbol=Store.storage source="storage: Map<string, int32>" type=Map<string, int32, Equality<string>>
    /// @generic.instance id="Map<string, int32, Equality<string>>" template=Map arguments=(string, int32, Equality<string>)
    /// @generic.instance id="sliceAssumeInit<MaybeUninit<MapSlot<string, int32>>>" template=sliceAssumeInit arguments=(MaybeUninit<MapSlot<string, int32>>)
    /// @generic.instance id="sliceUninit<MaybeUninit<MapSlot<string, int32>>>" template=sliceUninit arguments=(MaybeUninit<MapSlot<string, int32>>)
    /// @generic.instance id=Equality<string> template=Equality arguments=(string)
    /// @generic.instance id=newPhantom<string> template=newPhantom arguments=(string)
    /// @generic.instance id=sliceAssumeInit<uint32> template=sliceAssumeInit arguments=(uint32)
    /// @generic.instance id=sliceUninit<uint32> template=sliceUninit arguments=(uint32)
    /// @resolution.name source=Map target=Map

}

extension of Store implements Index<string>, IndexSet<string, int32> {
/// @generic.instance id="Index<string, \"readonly\">" template=Index arguments=(string, "readonly")
/// @generic.instance id="IndexSet<string, int32, \"mutable\">" template=IndexSet arguments=(string, int32, "mutable")
/// @definition.extension symbol=<module>#2 form=local target=Store
/// @definition.implements symbol=<module>#2 source="IndexSet<string, int32>" target="IndexSet<string, int32>"
/// @definition.implements symbol=<module>#2 source=Index<string> target=Index<string>
/// @definition.associated.type symbol=Missing source="type Missing = undefined" key=Missing value=undefined
/// @definition.associated.type symbol=Output source="type Output<const R: Region> = int32" key=Output value=int32
/// @definition.method symbol=index slot=index type=<const R#2>(this: Borrowed<Store, R#2, "readonly">, string) => int32 | undefined
/// @definition.method symbol=indexSet slot=indexSet type=<indexSet.'a>(this: &indexSet.'a Store, string, int32) => void
/// @definition.conformance symbol=<module>#2 member=Missing requirement=Index.Missing
/// @definition.conformance symbol=<module>#2 member=Output requirement=Index.Output
/// @definition.conformance symbol=<module>#2 member=index requirement=Index.index
/// @definition.conformance symbol=<module>#2 member=indexSet requirement=IndexSet.indexSet
/// @resolution.name source=Store target=Store
/// @resolution.name source=Index target=Index
/// @resolution.name source=IndexSet target=IndexSet

    type Output<const R: Region> = int32;
    /// @generic.template symbol=Output parent=template#0 parameters=(const R#1: Region)
    /// @type.symbol symbol=Output source="type Output<const R: Region> = int32" type=int32
    /// @type.symbol symbol=Output.R source="const R: Region" type=R#1
    /// @resolution.name source=Region target=Region

    type Missing = undefined;
    /// @type.symbol symbol=Missing source="type Missing = undefined" type=undefined

    index<const R: Region>(this: Borrowed<this, R, "readonly">, key: string): int32 | undefined {
    /// @generic.template symbol=index parent=template#0 parameters=(const R#2: Region)
    /// @type.symbol symbol=index type=<const R#2>(this: Borrowed<Store, R#2, "readonly">, string) => int32 | undefined
    /// @type.symbol symbol=index.R source="const R: Region" type=R#2
    /// @resolution.name source=Region target=Region
    /// @type.symbol symbol=index.this source="this: Borrowed<this, R, \"readonly\">" type=Borrowed<Store, R#2, "readonly">
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=R target=index.R
    /// @type.symbol symbol=index.key source="key: string" type=string

        return this.storage[key];
        /// @resolution.member source=this.storage receiver=Borrowed<Store, R#2, "readonly"> type=Map<string, int32, Equality<string>> kind=field target_receiver=Borrowed<Store, R#2, "readonly"> key=storage target=Store.storage target_type=Map<string, int32, Equality<string>>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Store, R#2, "readonly">
        /// @resolution.place source=this placement=R#2 lifetime=R#2 access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.storage placement="local" lifetime=R#2 access="readonly"
        /// @resolution.access source=this.storage root=this keys=[storage]
        /// @resolution.subscript source=this.storage[key] type=int32 | undefined kind=call target="index#1(parameters=(&'managed immutable string), arguments=(provided(key) as &'managed immutable string), return=int32 | undefined, regions=(\"frame\", \"managed\" & \"local\", \"managed\" & \"local\"))"
        /// @generic.instantiation id="index#1<string, int32, Equality<string>, string, \"frame\", \"managed\" & \"local\", \"managed\" & \"local\">" template=index#1 arguments=(string, int32, Equality<string>, string, "frame", "managed" & "local", "managed" & "local")
        /// @generic.instance id="index#1<string, int32, Equality<string>, string, \"frame\", \"bound1\" & \"local\", \"bound2\" & \"local\">" template=index#1 arguments=(string, int32, Equality<string>, string, "frame", "bound1" & "local", "bound2" & "local")
        /// @resolution.name source=key target=index.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=index.key

    }

    indexSet(&this, key: string, value: int32): void {
    /// @generic.template symbol=indexSet parent=template#0 parameters=('a)
    /// @type.symbol symbol=indexSet type=<indexSet.'a>(this: &indexSet.'a Store, string, int32) => void
    /// @type.symbol symbol=indexSet.this source=&this type=&indexSet.'a Store
    /// @type.symbol symbol=indexSet.key source="key: string" type=string
    /// @type.symbol symbol=indexSet.value source="value: int32" type=int32

        this.storage[key] = value;
        /// @resolution.member source=this.storage receiver=&indexSet.'a Store type=Map<string, int32, Equality<string>> kind=field target_receiver=&indexSet.'a Store key=storage target=Store.storage target_type=Map<string, int32, Equality<string>>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&indexSet.'a Store
        /// @resolution.place source=this placement=indexSet.'a lifetime=indexSet.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.storage placement="local" lifetime=indexSet.'a access="mutable"
        /// @resolution.access source=this.storage root=this keys=[storage]
        /// @resolution.pattern.assign source=this.storage[key] kind=place
        /// @resolution.assignment source=this.storage[key] write="indexSet(parameters=(string, int32), arguments=(provided(key) as string, supplied(0) as int32), return=void, regions=(\"managed\" & \"local\"))" type=int32
        /// @generic.instantiation id="indexSet<string, int32, Equality<string>, \"mutable\", \"managed\" & \"local\">" template=indexSet arguments=(string, int32, Equality<string>, "mutable", "managed" & "local")
        /// @generic.instance id="indexSet<string, int32, Equality<string>, \"mutable\", \"bound0\" & \"local\">" template=indexSet arguments=(string, int32, Equality<string>, "mutable", "bound0" & "local")
        /// @generic.instance id="set#3<string, int32, Equality<string>, \"bound0\" & \"local\", \"mutable\">" template=set#3 arguments=(string, int32, Equality<string>, "bound0" & "local", "mutable")
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
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32>
/// @resolution.name source=Record target=Record

declare function read(bag: Bag): int32 | undefined;
/// @type.symbol symbol=read source="declare function read(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
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
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32>
/// @resolution.name source=Record target=Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

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
        "main.tspp",
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
/// @definition.type symbol=Bag source="type Bag = Record<usize, int32>" value=Record<usize, int32>
/// @resolution.name source=Record target=Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

const value = bag[1];
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=bag target=bag
/// @resolution.place source=bag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bag root=bag
/// @resolution.access source=bag[1] root=bag keys=[1]
/// @resolution.subscript source=bag[1] type=int32 | undefined kind=member target="receiver=Bag, target=index(usize), type=int32 | undefined"

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare const bag: Bag;
const value = bag.missing;

=== dir ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32>
/// @resolution.name source=Record target=Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

const value = bag.missing;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=bag target=bag
/// @resolution.place source=bag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bag root=bag
/// @resolution.rejected source=bag.missing
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'missing' does not exist on type 'Bag'"
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
        "main.tspp",
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
/// @generic.template symbol=Bag parameters=(in out T, this: Bag<T>)
/// @type.symbol symbol=Bag type=Bag
/// @definition.interface symbol=Bag template=(in out T, this: Bag<T>)
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
/// @resolution.place source=bag placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @generic.template symbol=Bag parameters=(this: Bag)
/// @type.symbol symbol=Bag type=Bag
/// @definition.interface symbol=Bag template=(this: Bag)
/// @definition.where symbol=Bag relation=satisfies left=this right=Bag
/// @definition.signature kind=index source="readonly [key: string]: int32" key=string type=int32

    readonly [key: string]: int32;
}

declare const values: { readonly [key: string]: int32 };
/// @type.symbol symbol=values source=values type={ readonly [key: string]: int32 }
/// @resolution.pattern source=values kind=binding target=values

values satisfies Bag;
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @generic.template symbol=Bag parameters=(this: Bag)
/// @type.symbol symbol=Bag type=Bag
/// @definition.interface symbol=Bag template=(this: Bag)
/// @definition.where symbol=Bag relation=satisfies left=this right=Bag
/// @definition.signature kind=index source="readonly [key: string]: int32" key=string type=int32

    readonly [key: string]: int32;
}

declare const values: { readonly [key: string]: string };
/// @type.symbol symbol=values source=values type={ readonly [key: string]: string }
/// @resolution.pattern source=values kind=binding target=values

values satisfies Bag;
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Bag {
    readonly [key: string]: int32;
}

struct Values implements Bag {}

=== dir ===
interface Bag {
/// @generic.template symbol=Bag parameters=(this: Bag)
/// @type.symbol symbol=Bag type=Bag
/// @definition.interface symbol=Bag template=(this: Bag)
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
    type Output<const R: Region> = Borrowed<int32, R, "readonly">;

    index<const R: Region>(
        this: Borrowed<this, R, "readonly">,
        key: string,
    ): this.Output<R> {
        return todo("Counter.index");
    }

    indexSet(&this, key: string, value: int32 | float64): void {}
}

declare let counter: Counter;
counter["value"] += 1;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {}

extension of Counter implements Index<string>, IndexSet<string, int32 | float64> {
    type Output<const R: Region> = Borrowed<int32, R, "readonly">;

    index<const R: Region>(
        this: Borrowed<this, R, "readonly">,
        key: string,
    ): Borrowed<int32, R, "readonly"> {
        return todo("Counter.index" as string | undefined);
    }

    indexSet(&this, key: string, value: int32 | float64): void {}
}

declare let counter: Counter;
(counter["value"] += 1) as int32 | float64;

=== dir ===
struct Counter {}
/// @type.symbol symbol=Counter source="struct Counter {}" type=Counter
/// @definition.struct symbol=Counter source="struct Counter {}"

extension of Counter implements Index<string>, IndexSet<string, int32 | float64> {
/// @generic.instance id="Index<string, \"readonly\">" template=Index arguments=(string, "readonly")
/// @generic.instance id="IndexSet<string, int32 | float64, \"mutable\">" template=IndexSet arguments=(string, int32 | float64, "mutable")
/// @definition.extension symbol=<module>#2 form=local target=Counter
/// @definition.implements symbol=<module>#2 source="IndexSet<string, int32 | float64>" target="IndexSet<string, int32 | float64>"
/// @definition.implements symbol=<module>#2 source=Index<string> target=Index<string>
/// @definition.associated.type symbol=Output source="type Output<const R: Region> = Borrowed<int32, R, \"readonly\">" key=Output value="Borrowed<int32, R#1, \"readonly\">"
/// @definition.method symbol=index slot=index type=<const R#2>(this: Borrowed<Counter, R#2, "readonly">, string) => Borrowed<int32, R#2, "readonly">
/// @definition.method symbol=indexSet source="indexSet(&this, key: string, value: int32 | float64): void {}" slot=indexSet type=<indexSet.'a>(this: &indexSet.'a Counter, string, int32 | float64) => void
/// @definition.conformance symbol=<module>#2 member=Index.Missing requirement=Index.Missing
/// @definition.conformance symbol=<module>#2 member=Output requirement=Index.Output
/// @definition.conformance symbol=<module>#2 member=index requirement=Index.index
/// @definition.conformance symbol=<module>#2 member=indexSet requirement=IndexSet.indexSet
/// @resolution.name source=Counter target=Counter
/// @resolution.name source=Index target=Index
/// @resolution.name source=IndexSet target=IndexSet

    type Output<const R: Region> = Borrowed<int32, R, "readonly">;
    /// @generic.template symbol=Output parent=template#0 parameters=(const R#1: Region)
    /// @type.symbol symbol=Output source="type Output<const R: Region> = Borrowed<int32, R, \"readonly\">" type=Borrowed<int32, R#1, "readonly">
    /// @type.symbol symbol=Output.R source="const R: Region" type=R#1
    /// @resolution.name source=Region target=Region
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=R target=Output.R

    index<const R: Region>(
    /// @generic.template symbol=index parent=template#0 parameters=(const R#2: Region)
    /// @type.symbol symbol=index type=<const R#2>(this: Borrowed<Counter, R#2, "readonly">, string) => Borrowed<int32, R#2, "readonly">
    /// @type.symbol symbol=index.R source="const R: Region" type=R#2
    /// @resolution.name source=Region target=Region

        this: Borrowed<this, R, "readonly">,
        /// @type.symbol symbol=index.this source="this: Borrowed<this, R, \"readonly\">" type=Borrowed<Counter, R#2, "readonly">
        /// @resolution.name source=Borrowed target=Borrowed
        /// @resolution.name source=R target=index.R

        key: string,
        /// @type.symbol symbol=index.key source="key: string" type=string

    ): this.Output<R> {
    /// @resolution.name source=this.Output<R> target=Output
    /// @resolution.name source=R target=index.R

        return todo("Counter.index");
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Counter.index\")" parameters=(string | undefined) arguments=(provided("Counter.index") as string | undefined) return=never kind=symbol target=todo

    }

    indexSet(&this, key: string, value: int32 | float64): void {}
    /// @generic.template symbol=indexSet parent=template#0 parameters=('a)
    /// @type.symbol symbol=indexSet source="indexSet(&this, key: string, value: int32 | float64): void {}" type=<indexSet.'a>(this: &indexSet.'a Counter, string, int32 | float64) => void
    /// @type.symbol symbol=indexSet.this source=&this type=&indexSet.'a Counter
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
/// @resolution.assignment source="counter[\"value\"]" read="index(parameters=(string), arguments=(provided(\"value\") as string), return=&'static readonly int32, regions=(\"static\" & \"local\"))" write="indexSet(parameters=(string, int32 | float64), arguments=(provided(\"value\") as string, supplied(0) as int32 | float64), return=void, regions=(\"static\" & \"local\"))" type=int32 | float64
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
/// @generic.instantiation id="index<\"static\" & \"local\">" template=index arguments=("static" & "local")
/// @generic.instantiation id="indexSet<\"static\" & \"local\">" template=indexSet arguments=("static" & "local")
/// @generic.instance id="index<\"bound0\" & \"local\">" template=index arguments=("bound0" & "local")
/// @generic.instance id="indexSet<\"bound0\" & \"local\">" template=indexSet arguments=("bound0" & "local")
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
        "main.tspp",
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {}
type Factory = { name: string; new (): Counter };

=== dir ===
class Counter {}
/// @type.symbol symbol=Counter source="class Counter {}" type=typeof Counter
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

/// Pass an optional aliased index signature field read through optional chaining.
#[test]
fn test_pass_an_optional_aliased_index_signature_field_through_optional_chaining() {
    let session = TestSession::single(
        r#"
type Fields = { readonly [key: string]: string };

struct Options {
    fields?: Fields | undefined;
}

class Span {
    fields: Fields | undefined;

    constructor(name: &readonly string, fields?: Fields) {
        this.fields = fields;
    }
}

function start(name: &readonly string, options?: Options): Span {
    new Span(name, options?.fields)
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Fields = { readonly [key: string]: string };

struct Options {
    fields?: Fields | undefined;
}

class Span {
    fields: Fields | undefined;

    constructor(name: &'a readonly string, fields?: Fields) {
        this.fields = fields;
    }
}

function start<'a>(name: &'a readonly string, options?: Options): Span {
    new Span(name, options?.fields)
}

=== dir ===
type Fields = { readonly [key: string]: string };
/// @type.symbol symbol=Fields source="type Fields = { readonly [key: string]: string }" type={ readonly [key: string]: string }
/// @definition.type symbol=Fields source="type Fields = { readonly [key: string]: string }" value={ readonly [key: string]: string }

struct Options {
/// @type.symbol symbol=Options type=Options
/// @definition.struct symbol=Options
/// @definition.field symbol=Options.fields source="fields?: Fields | undefined" key=fields type={ readonly [key: string]: string } | undefined

    fields?: Fields | undefined;
    /// @type.symbol symbol=Options.fields source="fields?: Fields | undefined" type={ readonly [key: string]: string } | undefined
    /// @resolution.name source=Fields target=Fields

}

class Span {
/// @type.symbol symbol=Span type=typeof Span
/// @definition.class symbol=Span
/// @definition.field symbol=Span.fields source="fields: Fields | undefined" key=fields type={ readonly [key: string]: string } | undefined
/// @definition.method symbol=Span.constructor slot=constructor role=constructor type=<Span.constructor.'a>(this: &'managed Span, &Span.constructor.'a readonly string, { readonly [key: string]: string } | undefined?) => Span

    fields: Fields | undefined;
    /// @type.symbol symbol=Span.fields source="fields: Fields | undefined" type={ readonly [key: string]: string } | undefined
    /// @resolution.name source=Fields target=Fields

    constructor(name: &readonly string, fields?: Fields) {
    /// @generic.template symbol=Span.constructor parameters=('a)
    /// @type.symbol symbol=Span.constructor type=<Span.constructor.'a>(this: &'managed Span, &Span.constructor.'a readonly string, { readonly [key: string]: string } | undefined?) => Span
    /// @type.symbol symbol=Span.constructor.this type=&'managed Span
    /// @type.symbol symbol=Span.constructor.name source="name: &readonly string" type=&Span.constructor.'a readonly string
    /// @type.symbol symbol=Span.constructor.fields source="fields?: Fields" type={ readonly [key: string]: string } | undefined
    /// @resolution.name source=Fields target=Fields

        this.fields = fields;
        /// @resolution.receiver source=this kind=this declaration=Span type=&'managed Span
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.fields kind=place
        /// @resolution.place source=this.fields placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.fields root=this keys=[fields]
        /// @resolution.assignment source=this.fields write="receiver=&'managed Span, target=field(receiver=&'managed Span, target=Span.fields, type={ readonly [key: string]: string } | undefined), type={ readonly [key: string]: string } | undefined" type={ readonly [key: string]: string } | undefined
        /// @resolution.name source=fields target=Span.constructor.fields
        /// @resolution.place source=fields placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=fields root=Span.constructor.fields

    }
}

function start(name: &readonly string, options?: Options): Span {
/// @generic.template symbol=start parameters=('a)
/// @type.symbol symbol=start type=<start.'a>(&start.'a readonly string, Options | undefined?) => Span
/// @type.symbol symbol=start.name source="name: &readonly string" type=&start.'a readonly string
/// @type.symbol symbol=start.options source="options?: Options" type=Options | undefined
/// @resolution.name source=Options target=Options
/// @resolution.name source=Span target=Span

    new Span(name, options?.fields)
    /// @resolution.construct source="new Span(name, options?.fields)" parameters=(&start.'a readonly string, { readonly [key: string]: string } | undefined) arguments=(provided(name) as &start.'a readonly string, provided(options?.fields) as { readonly [key: string]: string } | undefined) return=Span regions=(start.'a) kind=class target=Span constructor=Span.constructor call=Span.constructor<start.'a>
    /// @generic.instantiation id=Span.constructor<start.'a> template=Span.constructor arguments=(start.'a)
    /// @resolution.name source=Span target=Span
    /// @resolution.name source=name target=start.name
    /// @resolution.place source=name placement=start.'a lifetime=start.'a access="readonly"
    /// @resolution.access source=name root=start.name
    /// @resolution.name source=options target=start.options
    /// @resolution.member source=options?.fields receiver=Options | undefined type={ readonly [key: string]: string } | undefined kind=field target_receiver=Options | undefined adjustments=(union.payload(Options | undefined, Options, Options)) key=fields target=Options.fields target_type={ readonly [key: string]: string } | undefined
    /// @resolution.place source=options placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=options root=start.options
    /// @resolution.place source=options?.fields placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=options?.fields root=start.options keys=[fields]

}
"#,
        r#"
"#,
    );
}

/// A map field indexed through a borrowed struct receiver selects the map's index operator.
#[test]
fn test_index_a_map_field_through_a_borrowed_struct_receiver() {
    let session = TestSession::single(
        r#"
struct Store {
    storage: Map<string, int32>;
}

extension of Store {
    read(&this, key: string): int32 | undefined {
        return this.storage[key];
    }
}

declare const map: Map<string, int32>;

export function read(key: string): int32 | undefined {
    return map[key];
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Store {
    storage: Map<string, int32, Equality<string>>;
}

extension of Store {
    read(&this, key: string): int32 | undefined {
        return this.storage[key as &'managed immutable string];
    }
}

declare const map: Map<string, int32, Equality<string>>;

export function read(key: string): int32 | undefined {
    return map[key as &'managed immutable string];
}

=== dir ===
struct Store {
/// @type.symbol symbol=Store type=Store
/// @definition.struct symbol=Store
/// @definition.field symbol=Store.storage source="storage: Map<string, int32>" key=storage type=Map<string, int32, Equality<string>>

    storage: Map<string, int32>;
    /// @type.symbol symbol=Store.storage source="storage: Map<string, int32>" type=Map<string, int32, Equality<string>>
    /// @generic.instance id="Map<string, int32, Equality<string>>" template=Map arguments=(string, int32, Equality<string>)
    /// @generic.instance id="sliceAssumeInit<MaybeUninit<MapSlot<string, int32>>>" template=sliceAssumeInit arguments=(MaybeUninit<MapSlot<string, int32>>)
    /// @generic.instance id="sliceUninit<MaybeUninit<MapSlot<string, int32>>>" template=sliceUninit arguments=(MaybeUninit<MapSlot<string, int32>>)
    /// @generic.instance id=Equality<string> template=Equality arguments=(string)
    /// @generic.instance id=newPhantom<string> template=newPhantom arguments=(string)
    /// @generic.instance id=sliceAssumeInit<uint32> template=sliceAssumeInit arguments=(uint32)
    /// @generic.instance id=sliceUninit<uint32> template=sliceUninit arguments=(uint32)
    /// @resolution.name source=Map target=Map

}

extension of Store {
/// @definition.extension symbol=<module>#2 form=local target=Store
/// @definition.method symbol=read#1 slot=read type=<read#1.'a>(this: &read#1.'a Store, string) => int32 | undefined
/// @resolution.name source=Store target=Store

    read(&this, key: string): int32 | undefined {
    /// @generic.template symbol=read#1 parameters=('a)
    /// @type.symbol symbol=read#1 type=<read#1.'a>(this: &read#1.'a Store, string) => int32 | undefined
    /// @type.symbol symbol=read.this source=&this type=&read#1.'a Store
    /// @type.symbol symbol=read.key#1 source="key: string" type=string

        return this.storage[key];
        /// @resolution.member source=this.storage receiver=&read#1.'a Store type=Map<string, int32, Equality<string>> kind=field target_receiver=&read#1.'a Store key=storage target=Store.storage target_type=Map<string, int32, Equality<string>>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&read#1.'a Store
        /// @resolution.place source=this placement=read#1.'a lifetime=read#1.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.storage placement="local" lifetime=read#1.'a access="mutable"
        /// @resolution.access source=this.storage root=this keys=[storage]
        /// @resolution.subscript source=this.storage[key] type=int32 | undefined kind=call target="index#1(parameters=(&'managed immutable string), arguments=(provided(key) as &'managed immutable string), return=int32 | undefined, regions=(\"frame\", \"managed\" & \"local\", \"managed\" & \"local\"))"
        /// @generic.instantiation id="index#1<string, int32, Equality<string>, string, \"frame\", \"managed\" & \"local\", \"managed\" & \"local\">" template=index#1 arguments=(string, int32, Equality<string>, string, "frame", "managed" & "local", "managed" & "local")
        /// @generic.instance id="index#1<string, int32, Equality<string>, string, \"frame\", \"bound1\" & \"local\", \"bound2\" & \"local\">" template=index#1 arguments=(string, int32, Equality<string>, string, "frame", "bound1" & "local", "bound2" & "local")
        /// @resolution.name source=key target=read.key#1
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=read.key#1

    }
}

declare const map: Map<string, int32>;
/// @type.symbol symbol=map source=map type=Map<string, int32, Equality<string>>
/// @resolution.pattern source=map kind=binding target=map
/// @resolution.name source=Map target=Map

export function read(key: string): int32 | undefined {
/// @type.symbol symbol=read type=(string) => int32 | undefined
/// @type.symbol symbol=read.key source="key: string" type=string

    return map[key];
    /// @resolution.name source=map target=map
    /// @resolution.place source=map placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=map root=map
    /// @resolution.subscript source=map[key] type=int32 | undefined kind=call target="index#1(parameters=(&'managed immutable string), arguments=(provided(key) as &'managed immutable string), return=int32 | undefined, regions=(\"frame\", \"managed\" & \"local\", \"managed\" & \"local\"))"
    /// @resolution.name source=key target=read.key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=read.key

}
"#,
    );
}
