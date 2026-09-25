use crate::tests::TestSession;

#[test]
fn test_lower_inferred_generic_calls_to_concrete_instances() {
    let session = TestSession::single(
        r#"
function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}

function choose(low: int32, high: int32, flag: boolean): float64 {
    let first = pick(low, high, flag);
    return pick(1.5, 2.5, flag);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.choose",
        r#"
function test.main.choose(v0: int32, v1: int32, v2: boolean): float64 {
    local l0: int32
    local l1: int32
    local l2: boolean
    local l3: int32

entry(v0: int32, v1: int32, v2: boolean):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: int32 = load l0
    v4: int32 = load l1
    v5: boolean = load l2
    v6: int32 = call test.main.pick<int32>(v3, v4, v5): (int32, int32, boolean) => int32
    store l3, v6
    v7: float64 = 1.5
    v8: float64 = 2.5
    v9: boolean = load l2
    v10: float64 = call test.main.pick<float64>(v7, v8, v9): (float64, float64, boolean) => float64
    return v10
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.pick<int32>",
        r#"
shared function test.main.pick<int32>(v0: int32, v1: int32, v2: boolean): int32;
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.pick<float64>",
        r#"
shared function test.main.pick<float64>(v0: float64, v1: float64, v2: boolean): float64;
"#,
    );

    // concrete arguments distinguish both textual and persistent identities
    let lowered = session.mir_lowered("main.tspp");
    let strings = session.repository().string_pool();
    let instances: Vec<_> = lowered
        .tree
        .iter_nodes::<tspp_mir::Function>()
        .filter_map(|(_, function)| {
            (strings.get(function.name) == "test.main.pick" && function.generics.is_empty())
                .then_some((&function.arguments, function.symbol))
        })
        .collect();
    assert_eq!(instances.len(), 2);
    assert_ne!(instances[0].0, instances[1].0);
    assert_ne!(instances[0].1, instances[1].1);
}

#[test]
fn test_lower_generic_function_over_structural_representation() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

function keep(value: (int32, boolean)): (int32, boolean) {
    return identity(value);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.keep", r#"
function test.main.keep(v0: (int32, boolean)): (int32, boolean) {
    local l0: (int32, boolean)

entry(v0: (int32, boolean)):
    store l0, v0
    v1: (int32, boolean) = load l0
    v2: (int32, boolean) = call test.main.identity<(int32, boolean)>(v1): ((int32, boolean)) => (int32, boolean)
    return v2
}

/// @layout.tuple name=type@2 size=8 align=4
/// @layout.element owner=type@2 index=0 offset=0 size=4 align=4
/// @layout.element owner=type@2 index=1 offset=4 size=1 align=1
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.identity<(int32, boolean)>",
        r#"
shared function test.main.identity<(int32, boolean)>(v0: (int32, boolean)): (int32, boolean);

/// @layout.tuple name=type@2 size=8 align=4
/// @layout.element owner=type@2 index=0 offset=0 size=4 align=4
/// @layout.element owner=type@2 index=1 offset=4 size=1 align=1
"#,
    );
}

#[test]
fn test_lower_generic_struct_arguments_to_distinct_instances() {
    let session = TestSession::single(
        r#"
struct Box<T> {
    value: ^T;
}

function readInt(value: Box<int32>): int32 {
    return value.value;
}

function readFloat(value: Box<float64>): float64 {
    return value.value;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.readInt",
        r#"
type test.main.Box<T> {
    value: ?T;
}

function test.main.readInt(v0: test.main.Box<int32>): int32 {
    local l0: test.main.Box<int32>

entry(v0: test.main.Box<int32>):
    store l0, v0
    v1: int32 = load (l0).0
    return v1
}

/// @layout.struct name=test.main.Box<int32> size=4 align=4
/// @layout.field owner=test.main.Box<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.readFloat",
        r#"
type test.main.Box<T> {
    value: ?T;
}

function test.main.readFloat(v0: test.main.Box<float64>): float64 {
    local l0: test.main.Box<float64>

entry(v0: test.main.Box<float64>):
    store l0, v0
    v1: float64 = load (l0).0
    return v1
}

/// @layout.struct name=test.main.Box<float64> size=8 align=8
/// @layout.field owner=test.main.Box<float64> index=0 name=value offset=0 size=8 align=8
"#,
    );

    // assert one template declares Box and two closed applications apply it
    let lowered = session.mir_lowered("main.tspp");
    let strings = session.repository().string_pool();
    let templates: Vec<_> = lowered
        .tree
        .iter_nodes::<tspp_mir::TypeDeclaration>()
        .filter(|(_, declaration)| {
            declaration
                .name
                .is_some_and(|name| strings.get(name) == "test.main.Box")
        })
        .map(|(_, declaration)| {
            lowered
                .tree
                .identified_type(declaration.symbol)
                .expect("declaration has a type")
        })
        .collect();
    let [template] = templates.as_slice() else {
        panic!("Box declares one template, found {}", templates.len());
    };
    let applications: tspp_core::FxIndexSet<_> = lowered
        .tree
        .types()
        .filter_map(|(id, ty)| match ty {
            tspp_mir::Type::Application {
                base, arguments, ..
            } if *base == *template && !arguments.is_empty() => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(applications.len(), 2);
}

#[test]
fn test_lower_repeated_instantiations_to_one_shared_instance() {
    let session = TestSession::single(
        r#"
function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}

function narrow(flag: boolean): float64 {
    return pick(1, 2, flag);
}

function wide(flag: boolean): float64 {
    return pick(30.5, 40.5, flag);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.narrow",
        r#"
function test.main.narrow(v0: boolean): float64 {
    local l0: boolean

entry(v0: boolean):
    store l0, v0
    v1: float64 = 1
    v2: float64 = 2
    v3: boolean = load l0
    v4: float64 = call test.main.pick<float64>(v1, v2, v3): (float64, float64, boolean) => float64
    return v4
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.wide",
        r#"
function test.main.wide(v0: boolean): float64 {
    local l0: boolean

entry(v0: boolean):
    store l0, v0
    v1: float64 = 30.5
    v2: float64 = 40.5
    v3: boolean = load l0
    v4: float64 = call test.main.pick<float64>(v1, v2, v3): (float64, float64, boolean) => float64
    return v4
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.pick<float64>",
        r#"
shared function test.main.pick<float64>(v0: float64, v1: float64, v2: boolean): float64;
"#,
    );
}

#[test]
fn test_lower_transitive_generic_calls_through_instance_substitution() {
    let session = TestSession::single(
        r#"
function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}

function retry<T>(value: T, fallback: T, flag: boolean): T {
    return pick(value, fallback, flag);
}

function settle(count: int32, limit: int32, flag: boolean): int32 {
    return retry(count, limit, flag);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.settle",
        r#"
function test.main.settle(v0: int32, v1: int32, v2: boolean): int32 {
    local l0: int32
    local l1: int32
    local l2: boolean

entry(v0: int32, v1: int32, v2: boolean):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: int32 = load l0
    v4: int32 = load l1
    v5: boolean = load l2
    v6: int32 = call test.main.retry<int32>(v3, v4, v5): (int32, int32, boolean) => int32
    return v6
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.retry<int32>",
        r#"
shared function test.main.retry<int32>(v0: int32, v1: int32, v2: boolean): int32;
"#,
    );
}

#[test]
fn test_lower_imported_generic_calls_as_local_instance_copies() {
    let session = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { pick } from "./lib";

function choose(low: int32, high: int32, flag: boolean): int32 {
    return pick(low, high, flag);
}
"#,
        )
        .build();

    session.assert_mir_function(
        "main.tspp",
        "test.main.choose",
        r#"
function test.main.choose(v0: int32, v1: int32, v2: boolean): int32 {
    local l0: int32
    local l1: int32
    local l2: boolean

entry(v0: int32, v1: int32, v2: boolean):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: int32 = load l0
    v4: int32 = load l1
    v5: boolean = load l2
    v6: int32 = call test.lib.pick<int32>(v3, v4, v5): (int32, int32, boolean) => int32
    return v6
}
"#,
    );

    session.assert_mir_lowered(
        "lib.tspp",
        r#"
function test.lib.pick<T>(v0: T, v1: T, v2: boolean): T {
    local l0: T
    local l1: T
    local l2: boolean

entry(v0: T, v1: T, v2: boolean):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: boolean = load l2
    branch v3 => b1 | b2

b1:
    v4: T = load l0
    return v4

b2:
    v5: T = load l1
    return v5
}
"#,
    );
}

#[test]
fn test_lower_transitive_imported_instances_through_foreign_bodies() {
    let session = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}

export function retry<T>(value: T, fallback: T, flag: boolean): T {
    return pick(value, fallback, flag);
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { retry } from "./lib";

function settle(count: int32, limit: int32, flag: boolean): int32 {
    return retry(count, limit, flag);
}
"#,
        )
        .build();

    session.assert_mir_function(
        "main.tspp",
        "test.main.settle",
        r#"
function test.main.settle(v0: int32, v1: int32, v2: boolean): int32 {
    local l0: int32
    local l1: int32
    local l2: boolean

entry(v0: int32, v1: int32, v2: boolean):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: int32 = load l0
    v4: int32 = load l1
    v5: boolean = load l2
    v6: int32 = call test.lib.retry<int32>(v3, v4, v5): (int32, int32, boolean) => int32
    return v6
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.lib.retry<int32>",
        r#"
shared function test.lib.retry<int32>(v0: int32, v1: int32, v2: boolean): int32;
"#,
    );
}

#[test]
fn test_lower_instance_names_under_a_named_package() {
    let session = TestSession::builder()
        .module(
            "package.json",
            r#"{
  "packageManager": "tspp@2026.9.0",
  "name": "app"
}"#,
        )
        .module(
            "main.tspp",
            r#"
function pick<T>(chosen: T, other: T, flag: boolean): T {
    if (flag) {
        return chosen;
    }
    return other;
}

function choose(low: int32, high: int32, flag: boolean): int32 {
    return pick(low, high, flag);
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.tspp",
        r#"
function app.main.choose(v0: int32, v1: int32, v2: boolean): int32 {
    local l0: int32
    local l1: int32
    local l2: boolean

entry(v0: int32, v1: int32, v2: boolean):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: int32 = load l0
    v4: int32 = load l1
    v5: boolean = load l2
    v6: int32 = call app.main.pick<int32>(v3, v4, v5): (int32, int32, boolean) => int32
    return v6
}

function app.main.pick<T>(v0: T, v1: T, v2: boolean): T {
    local l0: T
    local l1: T
    local l2: boolean

entry(v0: T, v1: T, v2: boolean):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: boolean = load l2
    branch v3 => b1 | b2

b1:
    v4: T = load l0
    return v4

b2:
    v5: T = load l1
    return v5
}

shared function app.main.pick<int32>(v0: int32, v1: int32, v2: boolean): int32;
"#,
    );
}

#[test]
fn test_bind_a_generic_function_value_to_a_concrete_instance() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

function apply(f: (x: int32) => int32, v: int32): int32 {
    return f(v);
}

function run(): int32 {
    return apply(identity, 7);
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.apply", r#"
function test.main.apply(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32): int32 {
    local l0: function<(int32) => int32, repeatable, managed, mutable, local>
    local l1: int32

entry(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32):
    store l0, v0
    store l1, v1
    v2: function<(int32) => int32, repeatable, managed, mutable, local> = load l0
    v3: int32 = load l1
    v4: function<(int32) => int32, repeatable, borrowed, 'managed, mutable> = cast.bit v2 -> function<(int32) => int32, repeatable, borrowed, 'managed, mutable>
    v5: int32 = call.indirect v4(v3): (int32) => int32
    return v5
}
"#);

    session.assert_mir_function("main.tspp", "test.main.run", r#"
function test.main.run(): int32 {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind test.main.identity<int32>, v0
    v2: int32 = 7
    v3: int32 = call test.main.apply(v1, v2): (function<(int32) => int32, repeatable, managed, mutable, local>, int32) => int32
    return v3
}
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.identity<int32>",
        r#"
shared function test.main.identity<int32>(v0: int32): int32;
"#,
    );
}

#[test]
fn test_call_a_generic_method_at_its_argument_instance() {
    let session = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export class Channel {
    value: int32 = 0;

    send<T>(this, value: T): void {}
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Channel } from "./lib";

function notify(channel: Channel): void {
    channel.send<int32>(1);
}
"#,
        )
        .build();

    session.assert_mir_function("main.tspp", "test.main.notify", r#"
@nocopy
type test.lib.Channel;

function test.main.notify(v0: ref<test.lib.Channel, managed, mutable, local>): void {
    local l0: ref<test.lib.Channel, managed, mutable, local>

entry(v0: ref<test.lib.Channel, managed, mutable, local>):
    store l0, v0
    v1: ref<test.lib.Channel, managed, mutable, local> = load l0
    v2: int32 = 1
    call test.lib.Channel.send<int32>(v1, v2): (ref<test.lib.Channel, managed, mutable, local>, int32) => void
    return
}
"#);

    session.assert_mir_function("main.tspp", "test.lib.Channel.send<int32>", r#"
@nocopy
type test.lib.Channel;

shared function test.lib.Channel.send<int32>(v0: ref<test.lib.Channel, managed, mutable, local>, v1: int32): void;
"#);
}

#[test]
fn test_call_a_generic_owner_method_at_its_instance() {
    let session = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    read(this): T {
        return this.value;
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Box } from "./lib";

function unwrap(box: Box<int32>): int32 {
    return box.read();
}
"#,
        )
        .build();

    session.assert_mir_function("main.tspp", "test.main.unwrap", r#"
@nocopy
type test.lib.Box<T>;

function test.main.unwrap(v0: ref<test.lib.Box<int32>, managed, mutable, local>): int32 {
    local l0: ref<test.lib.Box<int32>, managed, mutable, local>

entry(v0: ref<test.lib.Box<int32>, managed, mutable, local>):
    store l0, v0
    v1: ref<test.lib.Box<int32>, managed, mutable, local> = load l0
    v2: int32 = call test.lib.Box.read<int32>(v1): (ref<test.lib.Box<int32>, managed, mutable, local>) => int32
    return v2
}
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.lib.Box.read<int32>",
        r#"
@nocopy
type test.lib.Box<T>;

shared function test.lib.Box.read<int32>(v0: ref<test.lib.Box<int32>, managed, mutable, local>): int32;
"#,
    );
}

#[test]
fn test_call_an_inherited_generic_method_at_the_base_instance() {
    let session = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export class Source<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    read(this): T {
        return this.value;
    }
}

export class Tap<T> extends Source<T> {
    constructor(value: T) {
        super(value);
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Tap } from "./lib";

function drain(tap: Tap<int32>): int32 {
    return tap.read();
}
"#,
        )
        .build();

    session.assert_mir_function("main.tspp", "test.main.drain", r#"
@nocopy
type test.lib.Tap<T>;

@nocopy
type test.lib.Source<T>;

function test.main.drain(v0: ref<test.lib.Tap<int32>, managed, mutable, local>): int32 {
    local l0: ref<test.lib.Tap<int32>, managed, mutable, local>

entry(v0: ref<test.lib.Tap<int32>, managed, mutable, local>):
    store l0, v0
    v1: ref<test.lib.Tap<int32>, managed, mutable, local> = load l0
    v2: ref<test.lib.Source<int32>, managed, mutable, local> = cast.bit v1 -> ref<test.lib.Source<int32>, managed, mutable, local>
    v3: int32 = call test.lib.Source.read<int32>(v2): (ref<test.lib.Source<int32>, managed, mutable, local>) => int32
    return v3
}
"#);

    session.assert_mir_function("main.tspp", "test.lib.Source.read<int32>", r#"
@nocopy
type test.lib.Source<T>;

shared function test.lib.Source.read<int32>(v0: ref<test.lib.Source<int32>, managed, mutable, local>): int32;
"#);
}

#[test]
fn test_lower_isomorphic_newtype_instances_separately() {
    let session = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export newtype AId = int32;
export newtype ARef = AId;
export newtype BId = int32;
export newtype BRef = BId;

export struct Pair<T> {
    value: T;
}

export function wrap<T>(value: T): Pair<T> {
    Pair { value }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { AId, ARef, BId, BRef, wrap } from "./lib";

function run(): int32 {
    const a = wrap(ARef(AId(1)));
    const b = wrap(BRef(BId(2)));
    return 0;
}
"#,
        )
        .build();

    session.assert_mir_function("main.tspp", "test.main.run", r#"
type test.lib.ARef;

type test.lib.AId;

type test.lib.Pair<T>;

type test.lib.BRef;

type test.lib.BId;

function test.main.run(): int32 {
    local l0: test.lib.Pair<test.lib.ARef>
    local l1: test.lib.Pair<test.lib.BRef>

entry:
    v0: int32 = 1
    v1: test.lib.AId = aggregate (v0)
    v2: test.lib.ARef = aggregate (v1)
    v3: test.lib.Pair<test.lib.ARef> = call test.lib.wrap<test.lib.ARef>(v2): (test.lib.ARef) => test.lib.Pair<test.lib.ARef>
    store l0, v3
    v4: int32 = 2
    v5: test.lib.BId = aggregate (v4)
    v6: test.lib.BRef = aggregate (v5)
    v7: test.lib.Pair<test.lib.BRef> = call test.lib.wrap<test.lib.BRef>(v6): (test.lib.BRef) => test.lib.Pair<test.lib.BRef>
    store l1, v7
    v8: int32 = 0
    return v8
}
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.lib.wrap<test.lib.ARef>",
        r#"
type test.lib.ARef;

type test.lib.Pair<T>;

shared function test.lib.wrap<test.lib.ARef>(v0: test.lib.ARef): test.lib.Pair<test.lib.ARef>;
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.lib.wrap<test.lib.BRef>",
        r#"
type test.lib.Pair<T>;

type test.lib.BRef;

shared function test.lib.wrap<test.lib.BRef>(v0: test.lib.BRef): test.lib.Pair<test.lib.BRef>;
"#,
    );
}

#[test]
fn test_call_a_generic_extension_static_across_modules() {
    let session = TestSession::builder()
        .module(
            "lib.tspp",
            r#"
export struct Wrap<'a, T> {
    value: &'a readonly T;
}

export type Alias<'a, T> = Wrap<'a, T>;

export extension<'a, T> of Wrap<'a, T> {
    static make(value: &'a readonly T): Wrap<'a, T> {
        return Wrap { value };
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Wrap } from "./lib";

function build(message: &readonly int32): Wrap<int32> {
    return Wrap.make(message);
}
"#,
        )
        .build();

    session.assert_mir_function("main.tspp", "test.main.build", r#"
type test.lib.Wrap<'a, T>;

function test.main.build<'a>(v0: ref<int32, borrowed, 'a, readonly>): test.lib.Wrap<'a, int32> {
    local l0: ref<int32, borrowed, 'a, readonly>

entry(v0: ref<int32, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<int32, borrowed, 'a, readonly> = load l0
    v2: test.lib.Wrap<'a, int32> = call test.lib.Wrap.make<'a, int32>(v1): (ref<int32, borrowed, 'a, readonly>) => test.lib.Wrap<'a, int32>
    return v2
}
"#);
}

#[test]
fn test_call_a_witness_member_at_a_closed_newtype_argument() {
    let session = TestSession::single(
        r#"
import { IoError } from "tspp:error";

interface Awaitable<T> {
    park(&this): T;
}

struct Task<T> implements Awaitable<T>, Drop {
    value: T;

    park(&this): T {
        return this.value;
    }

    drop(&this): void {}
}

interface Reader {
    read(): Task<Result<usize, IoError>>;
}

function take(reader: Reader): void {
    let task = reader.read();
    const value = task.park();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.take",
        r#"
@nocopy
type test.main.Reader { }

@languageItem("error.IoError")
type IoError;

@languageItem("error.Result")
type Result<T, E>;

@nocopy
type test.main.Task<T> {
    value: T;
}

function test.main.take(v0: dynamic<test.main.Reader, managed, mutable, local>): void {
    local l0: dynamic<test.main.Reader, managed, mutable, local>
    local l1: test.main.Task<Result<usize, IoError>>
    local l2: Result<usize, IoError>

entry(v0: dynamic<test.main.Reader, managed, mutable, local>):
    store l0, v0
    v1: dynamic<test.main.Reader, managed, mutable, local> = load l0
    v2: test.main.Task<Result<usize, IoError>> = call.dynamic v1, test.main.Reader, 0(): () => test.main.Task<Result<usize, IoError>>
    store l1, v2
    v3: ref<test.main.Task<Result<usize, IoError>>, borrowed, 'frame, mutable> = address l1
    v4: Result<usize, IoError> = call test.main.Task.park<Result<usize, IoError>>(v3): (ref<test.main.Task<Result<usize, IoError>>, borrowed, 'frame, mutable>) => Result<usize, IoError>
    store l2, v4
    return
}

/// @layout.struct name=test.main.Reader size=0 align=1
/// @layout.struct name=test.main.Task<Result<usize, IoError>> size=48 align=8
/// @layout.field owner=test.main.Task<Result<usize, IoError>> index=0 name=value offset=0 size=48 align=8
"#,
    );
}

#[test]
fn test_lower_an_access_parameter_borrow_through_a_delegating_accessor() {
    let session = TestSession::single(
        r#"
struct Cell<T> {
    value: T;

    get<const A: Access = "readonly">(this: WithAccess<&Cell<T>, A>): WithAccess<&T, A> | undefined {
        todo("Cell.get")
    }

    index<const A: Access = "readonly">(this: WithAccess<&Cell<T>, A>): WithAccess<&T, A> | undefined {
        this.get()
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Cell.index",
        r#"
type test.main.Cell<T> {
    value: T;
}

function test.main.Cell.index<T, A: Access, 'a>(v0: ref<test.main.Cell<T>, borrowed, 'a, A>): variant<uint1> { 0uint1 = ref<?T, borrowed, 'a, A>; 1uint1 = void; } {
    local l0: ref<test.main.Cell<T>, borrowed, 'a, A>

entry(v0: ref<test.main.Cell<T>, borrowed, 'a, A>):
    store l0, v0
    v1: ref<test.main.Cell<T>, borrowed, 'a, A> = address (*l0)
    v2: variant<uint1> { 0uint1 = ref<?T, borrowed, 'a, A>; 1uint1 = void; } = call test.main.Cell.get<T, A>(v1): (ref<test.main.Cell<T>, borrowed, 'a, A>) => variant<uint1> { 0uint1 = ref<?T, borrowed, 'a, A>; 1uint1 = void; }
    return v2
}
"#,
    );
}

#[test]
fn test_lower_an_access_parameter_borrow_delegated_to_a_borrowed_extension() {
    let session = TestSession::single(
        r#"
struct Cell<T> {
    value: T;

    index<const A: Access = "readonly">(this: WithAccess<&Cell<T>, A>): WithAccess<&T, A> | undefined {
        this.get()
    }
}

export extension<T, 'a, const A: Access> of Borrowed<Cell<T>, 'a, A> {
    get(): Borrowed<T, 'a, A> | undefined {
        todo("Cell.get")
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Cell.index",
        r#"
type test.main.Cell<T> {
    value: T;
}

function test.main.Cell.index<T, A: Access, 'a>(v0: ref<test.main.Cell<T>, borrowed, 'a, A>): variant<uint1> { 0uint1 = ref<?T, borrowed, 'a, A>; 1uint1 = void; } {
    local l0: ref<test.main.Cell<T>, borrowed, 'a, A>

entry(v0: ref<test.main.Cell<T>, borrowed, 'a, A>):
    store l0, v0
    v1: ref<test.main.Cell<T>, borrowed, 'a, A> = address (*l0)
    v2: variant<uint1> { 0uint1 = ref<?T, borrowed, 'a, A>; 1uint1 = void; } = call test.main.Cell.get<T, 'a, A>(v1): (ref<test.main.Cell<T>, borrowed, 'a, A>) => variant<uint1> { 0uint1 = ref<?T, borrowed, 'a, A>; 1uint1 = void; }
    return v2
}
"#,
    );
}

#[test]
fn test_copy_a_parameter_through_its_integer_bound() {
    let session = TestSession::single(
        r#"
import { Integer } from "tspp:math";

export function twice<T: Integer>(value: &T): T {
    const first: T = *value;
    const second: T = *value;
    return first;
}
"#,
    );
    session.assert_mir_lowered(
        "main.tspp",
        r#"
@nocopy
@languageItem("math.Integer")
type Integer extends Concrete, Copy, IntegerDomain, Zero, One { }

@nocopy
@languageItem("memory.Concrete")
type Concrete { }

@nocopy
@languageItem("memory.Copy")
type Copy extends Clone { }

@nocopy
@languageItem("memory.Clone")
type Clone { }

@nocopy
@languageItem("math.IntegerDomain")
type IntegerDomain { }

@nocopy
@languageItem("math.Zero")
type Zero { }

@nocopy
@languageItem("math.One")
type One { }

function test.main.twice<T: Integer, 'a>(v0: ref<?T, borrowed, 'a, mutable>): T {
    local l0: ref<?T, borrowed, 'a, mutable>
    local l1: T
    local l2: T

entry(v0: ref<?T, borrowed, 'a, mutable>):
    store l0, v0
    v1: ref<?T, borrowed, 'a, mutable> = load l0
    v2: ?T = load (*v1)
    v3: T = new.complete v2
    store l1, v3
    v4: ref<?T, borrowed, 'a, mutable> = load l0
    v5: ?T = load (*v4)
    v6: T = new.complete v5
    store l2, v6
    v7: T = load l1
    return v7
}

/// @dispatch.shape constraint=type@3 function=clone function=cloneFrom function=zero function=one
"#,
    );
}

#[test]
fn test_reborrow_a_reference_receiver_for_a_borrowed_default_member() {
    let session = TestSession::single(
        r#"
newtype interface Dup {
    dup(&readonly this): ^this;
    dupFrom(&this, source: &readonly this): void {
        *this = source.dup();
    }
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.Dup.dupFrom",
        r#"
@nocopy
type test.main.Dup { }

function test.main.Dup.dupFrom<this: test.main.Dup, 'a, 'b>(v0: ref<?this, borrowed, 'a, mutable>, v1: ref<?this, borrowed, 'b, readonly>): void {
    local l0: ref<?this, borrowed, 'b, readonly>
    local l1: ref<?this, borrowed, 'a, mutable>

entry(v0: ref<?this, borrowed, 'a, mutable>, v1: ref<?this, borrowed, 'b, readonly>):
    store l0, v1
    store l1, v0
    v2: ref<?this, borrowed, 'a, mutable> = load l1
    v3: ref<?this, borrowed, 'b, readonly> = load l0
    v4: ?this = call.witness this, test.main.Dup, test.main.Dup.dup(v3): (ref<?this, borrowed, 'b, readonly>) => ?this
    store (*v2), v4
    return
}
"#,
    );
}

#[test]
fn test_replace_through_an_exclusive_reference_at_an_open_type() {
    let session = TestSession::single(
        r#"
import { replace } from "tspp:memory";

struct Cell<T> {
    value: T;
}

export extension<T> of Cell<T> {
    index(&exclusive this): &exclusive T {
        return &exclusive this.value;
    }

    set(&exclusive this, value: T): void {
        replace(this.index(), value);
    }
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.Cell.set",
        r#"
type test.main.Cell<T> {
    value: T;
}

function test.main.Cell.set<T, 'a>(v0: ref<test.main.Cell<T>, borrowed, 'a, exclusive>, v1: T): void {
    local l0: T
    local l1: ref<test.main.Cell<T>, borrowed, 'a, exclusive>

entry(v0: ref<test.main.Cell<T>, borrowed, 'a, exclusive>, v1: T):
    store l0, v1
    store l1, v0
    v2: ref<test.main.Cell<T>, borrowed, 'a, exclusive> = address (*l1)
    v3: ref<?T, borrowed, 'a, exclusive> = call test.main.Cell.index<T>(v2): (ref<test.main.Cell<T>, borrowed, 'a, exclusive>) => ref<?T, borrowed, 'a, exclusive>
    v4: ref<T, raw, exclusive> = cast.bit v3 -> ref<T, raw, exclusive>
    v5: T = load l0
    v6: T = load (*v4)
    store (*v4), v5
    return
}
"#,
    );
}

#[test]
fn test_address_a_slice_element_at_the_slice_access() {
    let session = TestSession::single(
        r#"
import { Slice } from "tspp:collections";
import { Copy } from "tspp:memory";

@unsafe
@intrinsic("collections.slice.get")
declare function sliceGet2<T: Copy>(slice: &readonly [T], index: usize): T;

extension<T> of Slice<T> {
    @unsafe
    unsafeGet2(&readonly this, index: usize): T where T: Copy {
        return sliceGet2<T>(this, index);
    }
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.Slice.unsafeGet2",
        r#"
function test.main.Slice.unsafeGet2<T: Copy, 'a>(v0: slice<T, borrowed, 'a, readonly>, v1: usize): T {
    local l0: usize
    local l1: slice<T, borrowed, 'a, readonly>

entry(v0: slice<T, borrowed, 'a, readonly>, v1: usize):
    store l0, v1
    store l1, v0
    v2: slice<T, borrowed, 'a, readonly> = load l1
    v3: usize = load l0
    v4: T = load (*v2)[v3]
    return v4
}
"#,
    );
}

/// Complete a generic default into a static call argument.
#[test]
fn test_complete_a_generic_default_into_a_static_call_argument() {
    let session = TestSession::single(
        r#"
struct Slot<T> {
    value: T;
}

export extension<T> of Slot<T> {
    static of(value: T): Slot<T> {
        Slot { value }
    }
}

function fill<T: Default>(): Slot<T> {
    return Slot.of(T.default());
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.fill",
        r#"
type test.main.Slot<T> {
    value: T;
}

@nocopy
@languageItem("memory.Default")
type Default;

function test.main.fill<T: Default>(): test.main.Slot<T> {
entry:
    v0: ?T = call.witness T, Default, Default.default(): () => ?T
    v1: T = new.complete v0
    v2: test.main.Slot<T> = call test.main.Slot.of<T>(v1): (T) => test.main.Slot<T>
    return v2
}
"#,
    );
}

/// Compare a borrowed field with a borrowed parameter at an open type.
#[test]
fn test_compare_a_borrowed_field_with_a_borrowed_parameter_at_an_open_type() {
    let session = TestSession::single(
        r#"
struct Holder<'a, T> {
    value: Borrowed<T, 'a, "immutable">;
}

export extension<'a, T> of Holder<'a, T> {
    same(this, other: &immutable T): boolean where T: StrictEqual<T> {
        return *this.value === *other;
    }
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.Holder.same",
        r#"
type test.main.Holder<'a, T> {
    value: ref<?T, borrowed, 'a, immutable>;
}

function test.main.Holder.same<'a, T: StrictEqual<T>, 'a>(v0: test.main.Holder<'a, T>, v1: ref<?T, borrowed, 'a, immutable>): boolean {
    local l0: ref<?T, borrowed, 'a, immutable>
    local l1: test.main.Holder<'a, T>

entry(v0: test.main.Holder<'a, T>, v1: ref<?T, borrowed, 'a, immutable>):
    store l0, v1
    store l1, v0
    v2: ref<?T, borrowed, 'a, immutable> = load (l1).0
    v3: ?T = load (*v2)
    v4: ref<?T, borrowed, 'a, immutable> = load l0
    v5: ?T = load (*v4)
    v6: boolean = eq v3, v5
    return v6
}
"#,
    );
}
