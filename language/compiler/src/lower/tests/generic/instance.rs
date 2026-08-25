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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.choose(v0: int32, v1: int32, v2: boolean): float64 {
    local l0: int32

entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.main.pick<int32>(v0, v1, v2): (int32, int32, boolean) => int32
    local.set l0, v3
    v4: float64 = 1.5
    v5: float64 = 2.5
    v6: float64 = call test.main.pick<float64>(v4, v5, v2): (float64, float64, boolean) => float64
    return v6
}

function test.main.pick<int32>(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2 => b1 | b2

b1:
    return v0

b2:
    return v1
}

function test.main.pick<float64>(v0: float64, v1: float64, v2: boolean): float64 {
entry(v0: float64, v1: float64, v2: boolean):
    branch v2 => b1 | b2

b1:
    return v0

b2:
    return v1
}
"#,
    );

    // concrete arguments distinguish both textual and persistent identities
    let lowered = session.mir_lowered("main.ds");
    let strings = session.repository().string_pool();
    let instances: Vec<_> = lowered
        .tree
        .iter_nodes::<destack_mir::Function>()
        .filter_map(|(_, function)| {
            (strings.get(function.name) == "test.main.pick")
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.keep(v0: (int32, boolean)): (int32, boolean) {
entry(v0: (int32, boolean)):
    v1: (int32, boolean) = call test.main.identity<type (int32, boolean)>(v0): ((int32, boolean)) => (int32, boolean)
    return v1
}

function test.main.identity<type (int32, boolean)>(v0: (int32, boolean)): (int32, boolean) {
entry(v0: (int32, boolean)):
    return v0
}

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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Box<int32> {
    value: int32;
}

@copy
type Box<float64> {
    value: float64;
}

function test.main.readInt(v0: Box<int32>): int32 {
entry(v0: Box<int32>):
    v1: int32 = field.get v0, 0
    return v1
}

function test.main.readFloat(v0: Box<float64>): float64 {
entry(v0: Box<float64>):
    v1: float64 = field.get v0, 0
    return v1
}

/// @layout.struct name=Box<int32> size=4 align=4
/// @layout.field owner=Box<int32> index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=Box<float64> size=8 align=8
/// @layout.field owner=Box<float64> index=0 name=value offset=0 size=8 align=8
"#,
    );

    // assert nominal specializations keep independent identities
    let lowered = session.mir_lowered("main.ds");
    let strings = session.repository().string_pool();
    let symbols: Vec<_> = lowered
        .tree
        .iter_nodes::<destack_mir::TypeDeclaration>()
        .filter_map(|(_, declaration)| {
            (strings.get(declaration.name) == "Box")
                .then(|| lowered.tree.type_symbol(declaration.ty))
                .flatten()
        })
        .collect();
    assert_eq!(symbols.len(), 2);
    assert_ne!(symbols[0], symbols[1]);
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.narrow(v0: boolean): float64 {
entry(v0: boolean):
    v1: float64 = 1
    v2: float64 = 2
    v3: float64 = call test.main.pick<float64>(v1, v2, v0): (float64, float64, boolean) => float64
    return v3
}

function test.main.wide(v0: boolean): float64 {
entry(v0: boolean):
    v1: float64 = 30.5
    v2: float64 = 40.5
    v3: float64 = call test.main.pick<float64>(v1, v2, v0): (float64, float64, boolean) => float64
    return v3
}

function test.main.pick<float64>(v0: float64, v1: float64, v2: boolean): float64 {
entry(v0: float64, v1: float64, v2: boolean):
    branch v2 => b1 | b2

b1:
    return v0

b2:
    return v1
}
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.settle(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.main.retry<int32>(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function test.main.retry<int32>(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.main.pick<int32>(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function test.main.pick<int32>(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2 => b1 | b2

b1:
    return v0

b2:
    return v1
}
"#,
    );
}

#[test]
fn test_lower_imported_generic_calls_as_local_instance_copies() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
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
            "main.ds",
            r#"
import { pick } from "./lib";

function choose(low: int32, high: int32, flag: boolean): int32 {
    return pick(low, high, flag);
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.choose(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.lib.pick<int32>(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function test.lib.pick<int32>(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2 => b1 | b2

b1:
    return v0

b2:
    return v1
}
"#,
    );

    session.assert_mir_lowered(
        "lib.ds", r#"
"#,
    );
}

#[test]
fn test_lower_transitive_imported_instances_through_foreign_bodies() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
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
            "main.ds",
            r#"
import { retry } from "./lib";

function settle(count: int32, limit: int32, flag: boolean): int32 {
    return retry(count, limit, flag);
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.settle(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.lib.retry<int32>(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function test.lib.retry<int32>(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call test.lib.pick<int32>(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function test.lib.pick<int32>(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2 => b1 | b2

b1:
    return v0

b2:
    return v1
}
"#,
    );
}

#[test]
fn test_lower_instance_names_under_a_named_package() {
    let session = TestSession::builder()
        .module(
            "destack.json",
            r#"{
  "name": "app"
}"#,
        )
        .module(
            "main.ds",
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
        "main.ds",
        r#"
function app.main.choose(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call app.main.pick<int32>(v0, v1, v2): (int32, int32, boolean) => int32
    return v3
}

function app.main.pick<int32>(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2 => b1 | b2

b1:
    return v0

b2:
    return v1
}
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.apply(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32): int32 {
entry(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) => int32
    return v2
}

function test.main.run(): int32 {
entry:
    v0: ref<void, managed, mutable, nullable, local> = null
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind test.main.identity<int32>, v0
    v2: int32 = 7
    v3: int32 = call test.main.apply(v1, v2): (function<(int32) => int32, repeatable, managed, mutable, local>, int32) => int32
    return v3
}

function test.main.identity<int32>(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
}

#[test]
fn test_call_a_generic_method_at_its_argument_instance() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
            r#"
export class Channel {
    value: int32 = 0;

    send<T>(this, value: T): void {}
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Channel } from "./lib";

function notify(channel: &Channel): void {
    channel.send<int32>(1);
}
"#,
        )
        .build();

    session.assert_mir_lowered("main.ds", r#"
type test.lib.Channel {
    value: int32;
}

function test.main.notify<'a>(v0: ref<test.lib.Channel, borrowed, 'a, mutable, local>): void {
entry(v0: ref<test.lib.Channel, borrowed, 'a, mutable, local>):
    v1: ref<test.lib.Channel, managed, mutable, local> = load v0
    v2: int32 = 1
    call test.lib.Channel.send<int32>(v1, v2): (ref<test.lib.Channel, managed, mutable, local>, int32) => void
    return
}

function test.lib.Channel.send<int32>(v0: ref<test.lib.Channel, managed, mutable, local>, v1: int32): void {
entry(v0: ref<test.lib.Channel, managed, mutable, local>, v1: int32):
    return
}

/// @layout.struct name=test.lib.Channel size=4 align=4
/// @layout.field owner=test.lib.Channel index=0 name=value offset=0 size=4 align=4
"#);
}

#[test]
fn test_call_a_generic_owner_method_at_its_instance() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
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
            "main.ds",
            r#"
import { Box } from "./lib";

function unwrap(box: &Box<int32>): int32 {
    return box.read();
}
"#,
        )
        .build();

    session.assert_mir_lowered("main.ds", r#"
type test.lib.Box<int32> {
    value: int32;
}

function test.main.unwrap<'a>(v0: ref<test.lib.Box<int32>, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<test.lib.Box<int32>, borrowed, 'a, mutable, local>):
    v1: ref<test.lib.Box<int32>, managed, mutable, local> = load v0
    v2: int32 = call test.lib.Box.read<int32>(v1): (ref<test.lib.Box<int32>, managed, mutable, local>) => int32
    return v2
}

function test.lib.Box.read<int32>(v0: ref<test.lib.Box<int32>, managed, mutable, local>): int32 {
entry(v0: ref<test.lib.Box<int32>, managed, mutable, local>):
    v1: ref<int32, borrowed, mutable, local> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

/// @layout.struct name=test.lib.Box<int32> size=4 align=4
/// @layout.field owner=test.lib.Box<int32> index=0 name=value offset=0 size=4 align=4
"#);
}

#[test]
fn test_call_an_inherited_generic_method_at_the_base_instance() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
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
            "main.ds",
            r#"
import { Tap } from "./lib";

function drain(tap: &Tap<int32>): int32 {
    return tap.read();
}
"#,
        )
        .build();

    session.assert_mir_lowered("main.ds", r#"
type test.lib.Source<int32> {
    value: int32;
}

type test.lib.Tap<int32> {
    value: int32;
}

function test.main.drain<'a>(v0: ref<test.lib.Tap<int32>, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<test.lib.Tap<int32>, borrowed, 'a, mutable, local>):
    v1: ref<test.lib.Tap<int32>, managed, mutable, local> = load v0
    v2: int32 = call test.lib.Source.read<int32>(v1): (ref<test.lib.Source<int32>, managed, mutable, local>) => int32
    return v2
}

function test.lib.Source.read<int32>(v0: ref<test.lib.Source<int32>, managed, mutable, local>): int32 {
entry(v0: ref<test.lib.Source<int32>, managed, mutable, local>):
    v1: ref<int32, borrowed, mutable, local> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

/// @layout.struct name=test.lib.Source<int32> size=4 align=4
/// @layout.field owner=test.lib.Source<int32> index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=test.lib.Tap<int32> size=4 align=4
/// @layout.field owner=test.lib.Tap<int32> index=0 name=value offset=0 size=4 align=4
"#);
}

#[test]
fn test_lower_isomorphic_newtype_instances_separately() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
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
            "main.ds",
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

    session.assert_mir_lowered("main.ds", r#"
@copy
type test.lib.AId = newtype<int32>;

@copy
type test.lib.ARef = newtype<test.lib.AId>;

@copy
type test.lib.Pair<test.lib.ARef> {
    value: test.lib.ARef;
}

@copy
type test.lib.BId = newtype<int32>;

@copy
type test.lib.BRef = newtype<test.lib.BId>;

@copy
type test.lib.Pair<test.lib.BRef> {
    value: test.lib.BRef;
}

function test.main.run(): int32 {
entry:
    v0: int32 = 1
    v1: test.lib.AId = aggregate (v0)
    v2: test.lib.ARef = aggregate (v1)
    v3: test.lib.Pair<test.lib.ARef> = call test.lib.wrap<test.lib.ARef>(v2): (test.lib.ARef) => test.lib.Pair<test.lib.ARef>
    v4: int32 = 2
    v5: test.lib.BId = aggregate (v4)
    v6: test.lib.BRef = aggregate (v5)
    v7: test.lib.Pair<test.lib.BRef> = call test.lib.wrap<test.lib.BRef>(v6): (test.lib.BRef) => test.lib.Pair<test.lib.BRef>
    v8: int32 = 0
    return v8
}

function test.lib.wrap<test.lib.ARef>(v0: test.lib.ARef): test.lib.Pair<test.lib.ARef> {
entry(v0: test.lib.ARef):
    v1: test.lib.Pair<test.lib.ARef> = aggregate (v0)
    return v1
}

function test.lib.wrap<test.lib.BRef>(v0: test.lib.BRef): test.lib.Pair<test.lib.BRef> {
entry(v0: test.lib.BRef):
    v1: test.lib.Pair<test.lib.BRef> = aggregate (v0)
    return v1
}

/// @layout.struct name=test.lib.Pair<test.lib.ARef> size=4 align=4
/// @layout.field owner=test.lib.Pair<test.lib.ARef> index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=test.lib.Pair<test.lib.BRef> size=4 align=4
/// @layout.field owner=test.lib.Pair<test.lib.BRef> index=0 name=value offset=0 size=4 align=4
"#);
}

#[test]
fn test_call_a_generic_extension_static_across_modules() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
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
            "main.ds",
            r#"
import { Wrap } from "./lib";

function build(message: &readonly int32): Wrap<int32> {
    return Wrap.make(message);
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds", r#"
@copy
type test.lib.Wrap<int32, 'a> {
    value: ref<int32, borrowed, 'a, readonly, local>;
}

function test.main.build<'a>(v0: ref<int32, borrowed, 'a, readonly, local>): test.lib.Wrap<int32> {
entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    v1: test.lib.Wrap<int32> = call test.lib.make<int32>(v0): (ref<int32, borrowed, readonly, local>) => test.lib.Wrap<int32>
    return v1
}

function test.lib.make<int32>(v0: ref<int32, borrowed, readonly, local>): test.lib.Wrap<int32> {
entry(v0: ref<int32, borrowed, readonly, local>):
    v1: test.lib.Wrap<int32> = aggregate (v0)
    return v1
}

/// @layout.struct name=test.lib.Wrap<int32> size=8 align=8
/// @layout.field owner=test.lib.Wrap<int32> index=0 name=value offset=0 size=8 align=8
"#,
    );
}
