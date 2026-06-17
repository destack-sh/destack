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

function read<T0: Bag>(bag: T0): int32 | undefined {
    const x: int32 | undefined = bag["x"];
    const missing: int32 | undefined = bag["missing"];
    missing satisfies int32 | undefined;
    return x;
}

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
const x: int32 | undefined = read<{ x: int32; y: int32 }>(point);
x satisfies int32 | undefined;

=== checked ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

function read(bag: Bag): int32 | undefined {
/// @generic.template symbol=read parameters=[T0: Bag]
/// @type.symbol symbol=read type=<read.T0: Bag>(read.T0) => int32 | undefined
/// @type.symbol symbol=bag type=read.T0
/// @resolution.name source=Bag target=Bag

    const x = bag["x"];
    /// @type.symbol symbol=x type=int32 | undefined
    /// @resolution.name source=bag target=bag
    /// @resolution.member source="bag[\"x\"]" receiver=read.T0 kind=builtin builtin=subscript.index

    const missing = bag["missing"];
    /// @type.symbol symbol=missing type=int32 | undefined
    /// @resolution.name source=bag target=bag
    /// @resolution.member source="bag[\"missing\"]" receiver=read.T0 kind=builtin builtin=subscript.index

    missing satisfies int32 | undefined;
    /// @resolution.name source=missing target=missing

    return x;
    /// @resolution.name source=x target=x

}

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }

const x = read(point);
/// @type.symbol symbol=x type=int32 | undefined
/// @resolution.name source=read target=read
/// @resolution.name source=point target=point
/// @resolution.call source=read(point) parameters=({ x: int32; y: int32 }) return=int32 | undefined kind=symbol target=read instance="read<{ x: int32; y: int32 }>"
/// @generic.instance source=read(point) id="read<{ x: int32; y: int32 }>"

x satisfies int32 | undefined;
/// @resolution.name source=x target=x
/// @generic.instance id="read<{ x: int32; y: int32 }>" symbol=read arguments=[{ x: int32; y: int32 }]
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

declare function read<T0: Bag>(bag: T0): int32 | undefined;

const mixed: { x: int32; y: string } = { x: 1, y: "two" };
const value: int32 | undefined = read<{ x: int32; y: string }>(mixed);

=== checked ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

declare function read(bag: Bag): int32 | undefined;
/// @generic.template symbol=read parameters=[T0: Bag]
/// @type.symbol symbol=read type=<read.T0: Bag>(read.T0) => int32 | undefined
/// @type.symbol symbol=bag type=read.T0
/// @resolution.name source=Bag target=Bag

const mixed: { x: int32; y: string } = { x: 1, y: "two" };
/// @type.symbol symbol=mixed source=mixed type={ x: int32; y: string }

const value = read(mixed);
/// @type.symbol symbol=value type=int32 | undefined
/// @resolution.name source=read target=read
/// @resolution.name source=mixed target=mixed
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '{ x: int32; y: string }' is not assignable to type 'Bag'"
/// @diagnostic.label line=7 column=20 source=mixed
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

const checked: { x: int32 } = { x: 1 } satisfies Bag;
const missing = checked["missing"];

=== checked ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

const checked = { x: 1 } satisfies Bag;
/// @type.symbol symbol=checked source=checked type={ x: int32 }
/// @resolution.name source=Bag target=Bag

const missing = checked["missing"];
/// @type.symbol symbol=missing type=<error>
/// @resolution.name source=checked target=checked
"#,
        r#"
/// @diagnostic.error code=EC316 message="type '{ x: int32 }' cannot be indexed by type '\"missing\"'"
/// @diagnostic.label line=5 column=17 source="const missing = checked[\"missing\"];"
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

declare function write<T0: Bag>(bag: T0): int32 | undefined;

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
const bad: int32 | undefined = write<{ x: int32; y: int32 }>(point);

=== checked ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

declare function write(bag: Bag): int32 | undefined;
/// @generic.template symbol=write parameters=[T0: Bag]
/// @type.symbol symbol=write type=<write.T0: Bag>(write.T0) => int32 | undefined
/// @type.symbol symbol=bag type=write.T0
/// @resolution.name source=Bag target=Bag

const point: { x: int32; y: int32 } = { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }

const bad = write(point);
/// @type.symbol symbol=bad type=int32 | undefined
/// @resolution.name source=write target=write
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC216 message="type '{ x: int32; y: int32 }' is missing IndexSet<string, int32> for writable index signature"
/// @diagnostic.label line=7 column=13 source="const bad = write(point);"
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

function write<T0: Bag>(bag: T0): int32 | undefined {
    bag["x"] = 1;
    return bag["x"];
}

declare const map: Map<string, int32>;
const value: int32 | undefined = write<Map<string, int32>>(map);

value satisfies int32 | undefined;

=== checked ===
type Bag = { [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { [key: string]: int32 }" type={ [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { [key: string]: int32 }" value={ [key: string]: int32 }

function write(bag: Bag): int32 | undefined {
/// @generic.template symbol=write parameters=[T0: Bag]
/// @type.symbol symbol=write type=<write.T0: Bag>(write.T0) => int32 | undefined
/// @type.symbol symbol=bag type=write.T0
/// @resolution.name source=Bag target=Bag

    bag["x"] = 1;
    /// @resolution.name source=bag target=bag
    /// @resolution.member source="bag[\"x\"]" receiver=write.T0 kind=builtin builtin=subscript.index

    return bag["x"];
    /// @resolution.name source=bag target=bag
    /// @resolution.member source="bag[\"x\"]" receiver=write.T0 kind=builtin builtin=subscript.index

}

declare const map: Map<string, int32>;
/// @type.symbol symbol=map source=map type=Map<string, int32>
/// @resolution.name source=Map target=collections.map.Map

const value = write(map);
/// @type.symbol symbol=value type=int32 | undefined
/// @resolution.name source=write target=write
/// @resolution.name source=map target=map
/// @resolution.call source=write(map) parameters=(Map<string, int32>) return=int32 | undefined kind=symbol target=write instance=write<Map<string, int32>>
/// @generic.instance source=write(map) id=write<Map<string, int32>>

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
/// @generic.instance id=write<Map<string, int32>> symbol=write arguments=[Map<string, int32>]
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

    indexSet<comptime L0: Lifetime>(this: Borrowed<Store, L0, "exclusive">, key: string, value: int32): void {
        this.storage[key] = value;
    }
}

declare let store: Store;
declare function write<T0: Bag>(bag: T0): int32 | undefined;

const value: int32 | undefined = write<Store>(store);

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
/// @definition.extension form=interface target=Store interfaces=[ops.subscript.Index<string>, ops.subscript.IndexSet<string, int32>]
/// @resolution.name source=Store target=Store
/// @resolution.name source=Index target=ops.subscript.Index
/// @resolution.name source=IndexSet target=ops.subscript.IndexSet

    type Output = int32 | undefined;
    /// @type.symbol symbol=Output type=int32 | undefined

    index(key: string): this.Output {
    /// @type.symbol symbol=index type=(this: Store, string) => int32 | undefined
    /// @type.symbol symbol=key type=string
    /// @resolution.name source=this.Output target=Output

        return this.storage[key];
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.storage receiver=Store kind=symbol target=Store.storage
        /// @resolution.name source=key target=key
        /// @resolution.member source="this.storage[key]" receiver=Map<string, int32> kind=builtin builtin=subscript.index

    }

    indexSet(&exclusive this, key: string, value: int32): void {
    /// @type.symbol symbol=indexSet type=<indexSet.L0: Lifetime>(Borrowed<Store, indexSet.L0, "exclusive">, string, int32) => void
    /// @type.symbol symbol=key type=string
    /// @type.symbol symbol=value type=int32

        this.storage[key] = value;
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.storage receiver=Borrowed<Store, indexSet.L0, "exclusive"> kind=symbol target=Store.storage
        /// @resolution.name source=key target=key
        /// @resolution.name source=value target=value
        /// @resolution.member source="this.storage[key]" receiver=Map<string, int32> kind=builtin builtin=subscript.index

    }
}

declare let store: Store;
/// @type.symbol symbol=store source=store type=Store
/// @resolution.name source=Store target=Store

declare function write(bag: Bag): int32 | undefined;
/// @generic.template symbol=write parameters=[T0: Bag]
/// @type.symbol symbol=write type=<write.T0: Bag>(write.T0) => int32 | undefined
/// @type.symbol symbol=bag type=write.T0
/// @resolution.name source=Bag target=Bag

const value = write(store);
/// @type.symbol symbol=value type=int32 | undefined
/// @resolution.name source=write target=write
/// @resolution.name source=store target=store
/// @resolution.call source=write(store) parameters=(Store) return=int32 | undefined kind=symbol target=write instance=write<Store>
/// @generic.instance source=write(store) id=write<Store>

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
/// @generic.instance id=write<Store> symbol=write arguments=[Store]
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

declare function read<T0: Bag>(bag: T0): int32 | undefined;

const point: { x: int32 } = { x: 1 };
const bad: int32 | undefined = read<{ x: int32 }>(point);

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

const bad = read(point);
/// @type.symbol symbol=bad type=int32 | undefined
/// @resolution.name source=read target=read
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC216 message="type '{ x: int32 }' is missing IndexSet<string, int32> for writable index signature"
/// @diagnostic.label line=7 column=13 source="const bad = read(point);"
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
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type={ [P: string]: int32 }
/// @resolution.name source=Bag target=Bag

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
/// @type.symbol symbol=Bag source="type Bag = Record<usize, int32>" type={ [P: usize]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<usize, int32>" value={ [P: usize]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type={ [P: usize]: int32 }
/// @resolution.name source=Bag target=Bag

const value = bag[1];
/// @type.symbol symbol=value type=int32 | undefined
/// @resolution.name source=bag target=bag
/// @resolution.member source="bag[1]" receiver={ [P: usize]: int32 } kind=builtin builtin=subscript.index

value satisfies int32 | undefined;
/// @resolution.name source=value target=value
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
/// @type.symbol symbol=Bag source="type Bag = Record<string, int32>" type={ [P: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = Record<string, int32>" value={ [P: string]: int32 }
/// @resolution.name source=Record target=types.object.Record

declare const bag: Bag;
/// @type.symbol symbol=bag source=bag type={ [P: string]: int32 }
/// @resolution.name source=Bag target=Bag

const value = bag.missing;
/// @type.symbol symbol=value type=<error>
/// @resolution.name source=bag target=bag
"#,
        r#"
/// @diagnostic.error code=EC300 message="property 'missing' is only available via index signature"
/// @diagnostic.label line=5 column=15 source="const value = bag.missing;"
"#,
    );
}
