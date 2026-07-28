use crate::tests::{DirRows, TestSession};

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

    session.assert_dir_checked(
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
const x: int32 | undefined = read(point);
x satisfies int32 | undefined;

=== checked ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

function read(bag: Bag): int32 | undefined {
/// @type.symbol symbol=read type=(Bag) => int32 | undefined
/// @type.symbol symbol=read.bag source="bag: Bag" type=Bag reduced={ readonly [key: string]: int32 }
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
    /// @resolution.place source=missing placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=missing root=read.missing

    return x;
    /// @resolution.name source=x target=read.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=read.x

}

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }
/// @resolution.pattern source=point kind=binding target=point

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
/// @resolution.place source=x placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=x root=x#2
"#,
    );
}

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { readonly [key: string]: int32 };

declare function read(bag: Bag): int32 | undefined;

const mixed: { x: int32; y: string } = { x: 1, y: "two" };
const value: int32 | undefined = read(mixed);

=== checked ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

declare function read(bag: Bag): int32 | undefined;
/// @type.symbol symbol=read source="declare function read(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
/// @type.symbol symbol=read.bag source="bag: Bag" type=Bag reduced={ readonly [key: string]: int32 }
/// @resolution.name source=Bag target=Bag

const mixed: { x: int32; y: string } = { x: 1, y: "two" };
/// @type.symbol symbol=mixed source=mixed type={ x: int32; y: string }
/// @resolution.pattern source=mixed kind=binding target=mixed

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

#[test]
fn test_satisfies_does_not_add_index_signature_members() {
    let session = TestSession::single(
        r#"
type Bag = { readonly [key: string]: int32 };

const checked = { x: 1 } satisfies Bag;
const missing = checked["missing"];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { readonly [key: string]: int32 };

const checked: { x: 1 } = { x: 1 } satisfies Bag;
const missing = checked["missing"];

=== checked ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

const checked = { x: 1 } satisfies Bag;
/// @type.symbol symbol=checked source=checked type={ x: 1 }
/// @resolution.pattern source=checked kind=binding target=checked
/// @resolution.name source=Bag target=Bag

const missing = checked["missing"];
/// @type.symbol symbol=missing source=missing type=<error>
/// @resolution.pattern source=missing kind=binding target=missing
/// @resolution.name source=checked target=checked
/// @resolution.place source=checked placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=checked root=checked
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '[]' is not defined for '{ x: 1 }' and '\"missing\"'"
/// @diagnostic.label line=5 column=17 span="checked[\"missing\"]" line_source="const missing = checked[\"missing\"];"
"#,
    );
}

#[test]
fn test_writable_index_signature_rejects_finite_object() {
    let session = TestSession::single(
        r#"
type Bag = { [key: string]: int32 };

declare function write(bag: Bag): int32 | undefined;

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
const bad = write(point);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { [key: string]: int32 };

declare function write(bag: Bag): int32 | undefined;

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
const bad: int32 | undefined = write(point);

=== checked ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

declare function write(bag: Bag): int32 | undefined;
/// @type.symbol symbol=write source="declare function write(bag: Bag): int32 | undefined" type=(Bag) => int32 | undefined
/// @type.symbol symbol=write.bag source="bag: Bag" type=Bag reduced={ [key: string]: int32 }
/// @resolution.name source=Bag target=Bag

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }
/// @resolution.pattern source=point kind=binding target=point

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
/// @diagnostic.error id=writable-index-requires-index-set message="type '{ x: int32; y: int32 }' is missing IndexSet<string> with input 'int32' for writable index signature"
/// @diagnostic.label line=7 column=19 span="point" line_source="const bad = write(point);"
/// @diagnostic.related line=7 column=13 span="write(point)" line_source="const bad = write(point);" message="in this call"
"#,
    );
}

#[test]
fn test_writable_index_signature_accepts_map() {
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

    session.assert_dir_checked(
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
const value: int32 | undefined = write(map);

value satisfies int32 | undefined;

=== checked ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

function write(bag: Bag): int32 | undefined {
/// @type.symbol symbol=write type=(Bag) => int32 | undefined
/// @type.symbol symbol=write.bag source="bag: Bag" type=Bag reduced={ [key: string]: int32 }
/// @resolution.name source=Bag target=Bag

    bag["x"] = 1;
    /// @resolution.name source=bag target=write.bag
    /// @resolution.pattern.assign source="bag[\"x\"]" kind=place
    /// @resolution.assignment source="bag[\"x\"]" write="member(receiver=Bag, target=index(string), type=int32)" type=int32
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
/// @resolution.name source=Map target=collections.map.Map

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
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

/// @generic.instance id="Map<string, int32>" template=collections.map.Map arguments=(string, int32)
"#,
    );
}

#[test]
fn test_writable_index_signature_accepts_operator_implementation() {
    let session = TestSession::single(
        r#"
type Bag = { [key: string]: int32 };

struct Store {
    storage: Map<string, int32>;
}

extension of Store implements Index<string>, IndexSet<string, int32> {
    type Output = int32 | undefined;

    index(key: string): this.Output {
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { [key: string]: int32 };

struct Store {
    storage: Map<string, int32>;
}

extension of Store implements Index<string>, IndexSet<string, int32> {
    type Output = int32 | undefined;

    index(key: string): this.Output {
        return this.storage[key];
    }

    indexSet(&exclusive this, key: string, value: int32): void {
        this.storage[key] = value;
    }
}

declare let store: Store;
declare function write(bag: Bag): int32 | undefined;

const value: int32 | undefined = write(store);

value satisfies int32 | undefined;

=== checked ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

struct Store {
/// @type.symbol symbol=Store type=Store
/// @definition.struct symbol=Store
/// @definition.field symbol=Store.storage source="storage: Map<string, int32>" key=storage type=Map<string, int32>

    storage: Map<string, int32>;
    /// @type.symbol symbol=Store.storage source="storage: Map<string, int32>" type=Map<string, int32>
    /// @resolution.name source=Map target=collections.map.Map

}

extension of Store implements Index<string>, IndexSet<string, int32> {
/// @definition.extension symbol=<module>#2 form=local target=Store
/// @definition.implements symbol=<module>#2 source="IndexSet<string, int32>" target="IndexSet<string, int32>"
/// @definition.implements symbol=<module>#2 source=Index<string> target="Index<string, \"readonly\"><type Missing = never><type Output = int32 | undefined>"
/// @definition.associated.type symbol=Output source="type Output = int32 | undefined" key=Output value="int32 | undefined"
/// @definition.method symbol=index slot=index type=(this: this, string) => this.Output
/// @definition.method symbol=indexSet slot=indexSet type=<indexSet.'a>(this: &indexSet.'a exclusive this, string, int32) => void
/// @definition.implementation symbol=<module>#2 requirement=ops.subscript.Index.Missing target=ops.subscript.Index.Missing
/// @definition.implementation symbol=<module>#2 requirement=ops.subscript.Index.Output target=Output
/// @definition.implementation symbol=<module>#2 requirement=ops.subscript.Index.index target=index
/// @definition.implementation symbol=<module>#2 requirement=ops.subscript.IndexSet.indexSet target=indexSet
/// @resolution.name source=Store target=Store
/// @resolution.name source=Index target=ops.subscript.Index
/// @resolution.name source=IndexSet target=ops.subscript.IndexSet

    type Output = int32 | undefined;
    /// @type.symbol symbol=Output source="type Output = int32 | undefined" type=int32 | undefined

    index(key: string): this.Output {
    /// @type.symbol symbol=index type=(this: this, string) => this.Output
    /// @type.symbol symbol=index.key source="key: string" type=string

        return this.storage[key];
        /// @resolution.member source=this.storage receiver=Store type=Map<string, int32> kind=field target_receiver=Store key=storage target=Store.storage target_type=Map<string, int32>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Store
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.storage placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.storage root=this keys=[storage]
        /// @resolution.subscript source=this.storage[key] type=int32 | undefined kind=call target="collections.map.index(parameters=(string), arguments=(provided(key) as string), return=memory.type.WithAccess<&'frame int32, \"exclusive\"> | undefined)"
        /// @generic.instance source=this.storage[key] id="Map<string, int32>.<extension#3>.index<\"exclusive\">"
        /// @resolution.name source=key target=index.key
        /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=key root=index.key

    }

    indexSet(&exclusive this, key: string, value: int32): void {
    /// @generic.template symbol=indexSet parent=template#0 parameters=('a)
    /// @type.symbol symbol=indexSet type=<indexSet.'a>(this: &indexSet.'a exclusive this, string, int32) => void
    /// @type.symbol symbol=indexSet.this source="&exclusive this" type=&indexSet.'a exclusive this
    /// @type.symbol symbol=indexSet.key source="key: string" type=string
    /// @type.symbol symbol=indexSet.value source="value: int32" type=int32

        this.storage[key] = value;
        /// @resolution.member source=this.storage receiver=&indexSet.'a exclusive Store type=Map<string, int32> kind=field target_receiver=&indexSet.'a exclusive Store key=storage target=Store.storage target_type=Map<string, int32>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&indexSet.'a exclusive Store
        /// @resolution.place source=this placement="local" lifetime=indexSet.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.storage placement="local" lifetime=indexSet.'a access="exclusive"
        /// @resolution.access source=this.storage root=this keys=[storage]
        /// @resolution.pattern.assign source=this.storage[key] kind=place
        /// @resolution.assignment source=this.storage[key] write="collections.map.indexSet(parameters=(string, int32), arguments=(provided(key) as string, write as int32), return=void)" type=int32
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
/// @type.symbol symbol=write.bag source="bag: Bag" type=Bag reduced={ [key: string]: int32 }
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
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

/// @generic.instance id="Map<string, int32>" template=collections.map.Map arguments=(string, int32)
/// @generic.instance id="Map<string, int32>.<extension#3>.index<\"exclusive\">" template=collections.map.index arguments=(string, int32, "exclusive")
"#,
    );
}

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare function read(bag: Bag): int32 | undefined;

const point: { x: int32 } = { x: 1 };
const bad: int32 | undefined = read(point);

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

const bad = read(point);
/// @type.symbol symbol=bad source=bad type=int32 | undefined
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=read target=read
/// @resolution.call source=read(point) parameters=(Bag) arguments=(provided(point) as Bag) return=int32 | undefined kind=symbol target=read
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point

/// @generic.instance id="Record<string, int32>" template=types.object.Record arguments=(string, int32)
"#,
        r#"
/// @diagnostic.error id=writable-index-requires-index-set message="type '{ x: int32 }' is missing IndexSet<string> with input 'int32' for writable index signature"
/// @diagnostic.label line=7 column=18 span="point" line_source="const bad = read(point);"
/// @diagnostic.related line=7 column=13 span="read(point)" line_source="const bad = read(point);" message="in this call"
"#,
    );
}

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare const bag: Bag;
const value: int32 | undefined = bag["missing"];

value satisfies int32 | undefined;

=== checked ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type=Record<string, int32> reduced={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32> reduced={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag reduced={ [P: string]: int32 }
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
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

/// @generic.instance id="Record<string, int32>" template=types.object.Record arguments=(string, int32)
"#,
    );
}

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<usize, int32>;

declare const bag: Bag;
const value: int32 | undefined = bag[1];

value satisfies int32 | undefined;

=== checked ===
type Bag = Record<usize, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<usize, int32>" type=Record<usize, int32> reduced={ [P: usize]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<usize, int32>" value=Record<usize, int32> reduced={ [P: usize]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag reduced={ [P: usize]: int32 }
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
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

/// @generic.instance id="Record<usize, int32>" template=types.object.Record arguments=(usize, int32)
"#,
    );
}

#[test]
fn test_record_string_keys_reject_dot_access() {
    let session = TestSession::single(
        r#"
type Bag = Record<string, int32>;

declare const bag: Bag;
const value = bag.missing;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = Record<string, int32>;

declare const bag: Bag;
const value = bag.missing;

=== checked ===
type Bag = Record<string, int32>;
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type=Record<string, int32> reduced={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value=Record<string, int32> reduced={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type=Bag reduced={ [P: string]: int32 }
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

const value = bag.missing;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=bag target=bag
/// @resolution.place source=bag placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bag root=bag

/// @generic.instance id="Record<string, int32>" template=types.object.Record arguments=(string, int32)
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'missing' does not exist on type 'Bag'"
/// @diagnostic.label line=5 column=19 span="missing" line_source="const value = bag.missing;"
"#,
    );
}

// TODO #Incomplete: subscript selection does not project nominal interface
//  index signatures yet, so the read below still rejects with no-matching-operator
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Bag<in out T> {
    [key: string]: T;
}

declare const bag: Dynamic<Bag<int32>>;
const value: int32 | undefined = bag["name"];

=== checked ===
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
/// @type.symbol symbol=bag source=bag type=Dynamic<Bag<int32>>
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag

const value = bag["name"];
/// @type.symbol symbol=value source=value type=int32 | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="bag[\"name\"]" type=int32 | undefined
/// @type.node source=bag type=Dynamic<Bag<int32>>
/// @resolution.name source=bag target=bag
/// @resolution.subscript source="bag[\"name\"]" type=int32 | undefined kind=call target="dynamic(Dynamic<Bag<int32>> as Bag<int32>, index.read([key: string]: T))(parameters=(string), arguments=(provided(\"name\") as string), return=int32 | undefined)"
/// @resolution.place source=bag placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bag root=bag
/// @generic.instance source=bag id=Bag<int32>
/// @type.node source="\"name\"" type="name"

/// @generic.instance id=Bag<int32> template=Bag arguments=(int32)
"#,
        r#"

"#,
    );
}
