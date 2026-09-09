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
        "main.ds",
        "test.main.choose",
        r#"
function test.main.choose(v0: int32, v1: int32, v2: boolean): float64 {
    local l0: int32
    local l1: int32
    local l2: boolean
    local l3: int32

entry(v0: int32, v1: int32, v2: boolean):
    local.set l0, v0
    local.set l1, v1
    local.set l2, v2
    v3: int32 = local.get l0
    v4: int32 = local.get l1
    v5: boolean = local.get l2
    v6: int32 = call test.main.pick<int32>(v3, v4, v5): (int32, int32, boolean) => int32
    local.set l3, v6
    v7: float64 = 1.5
    v8: float64 = 2.5
    v9: boolean = local.get l2
    v10: float64 = call test.main.pick<float64>(v7, v8, v9): (float64, float64, boolean) => float64
    return v10
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.pick<int32>",
        r#"
shared function test.main.pick<int32>(v0: int32, v1: int32, v2: boolean): int32;
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.pick<float64>",
        r#"
shared function test.main.pick<float64>(v0: float64, v1: float64, v2: boolean): float64;
"#,
    );

    // concrete arguments distinguish both textual and persistent identities
    let lowered = session.mir_lowered("main.ds");
    let strings = session.repository().string_pool();
    let instances: Vec<_> = lowered
        .tree
        .iter_nodes::<destack_mir::Function>()
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

    session.assert_mir_function("main.ds", "test.main.keep", r#"
function test.main.keep(v0: (int32, boolean)): (int32, boolean) {
    local l0: (int32, boolean)

entry(v0: (int32, boolean)):
    local.set l0, v0
    v1: (int32, boolean) = local.get l0
    v2: (int32, boolean) = call test.main.identity<(int32, boolean)>(v1): ((int32, boolean)) => (int32, boolean)
    return v2
}

/// @layout.tuple name=type@2 size=8 align=4
/// @layout.element owner=type@2 index=0 offset=0 size=4 align=4
/// @layout.element owner=type@2 index=1 offset=4 size=1 align=1
"#);

    session.assert_mir_function(
        "main.ds",
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
        "main.ds",
        "test.main.readInt",
        r#"
@copy
type test.main.Box<T> {
    value: T;
}

function test.main.readInt(v0: test.main.Box<int32>): int32 {
    local l0: test.main.Box<int32>

entry(v0: test.main.Box<int32>):
    local.set l0, v0
    v1: test.main.Box<int32> = local.get l0
    v2: int32 = field.get v1, 0
    return v2
}

/// @layout.struct name=test.main.Box<int32> size=4 align=4
/// @layout.field owner=test.main.Box<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.readFloat",
        r#"
@copy
type test.main.Box<T> {
    value: T;
}

function test.main.readFloat(v0: test.main.Box<float64>): float64 {
    local l0: test.main.Box<float64>

entry(v0: test.main.Box<float64>):
    local.set l0, v0
    v1: test.main.Box<float64> = local.get l0
    v2: float64 = field.get v1, 0
    return v2
}

/// @layout.struct name=test.main.Box<float64> size=8 align=8
/// @layout.field owner=test.main.Box<float64> index=0 name=value offset=0 size=8 align=8
"#,
    );

    // assert one template declares Box and two closed applications apply it
    let lowered = session.mir_lowered("main.ds");
    let strings = session.repository().string_pool();
    let templates: Vec<_> = lowered
        .tree
        .iter_nodes::<destack_mir::TypeDeclaration>()
        .filter(|(_, declaration)| strings.get(declaration.name) == "test.main.Box")
        .map(|(_, declaration)| declaration.ty)
        .collect();
    let [template] = templates.as_slice() else {
        panic!("Box declares one template, found {}", templates.len());
    };
    let applications: destack_core::FxIndexSet<_> = lowered
        .tree
        .iter_nodes::<destack_mir::Type>()
        .filter_map(|(id, ty)| match ty {
            destack_mir::Type::Application {
                base, arguments, ..
            } if *base == destack_mir::TypeId::from(*template) && !arguments.is_empty() => Some(id),
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
        "main.ds",
        "test.main.narrow",
        r#"
function test.main.narrow(v0: boolean): float64 {
    local l0: boolean

entry(v0: boolean):
    local.set l0, v0
    v1: float64 = 1
    v2: float64 = 2
    v3: boolean = local.get l0
    v4: float64 = call test.main.pick<float64>(v1, v2, v3): (float64, float64, boolean) => float64
    return v4
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.wide",
        r#"
function test.main.wide(v0: boolean): float64 {
    local l0: boolean

entry(v0: boolean):
    local.set l0, v0
    v1: float64 = 30.5
    v2: float64 = 40.5
    v3: boolean = local.get l0
    v4: float64 = call test.main.pick<float64>(v1, v2, v3): (float64, float64, boolean) => float64
    return v4
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
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
        "main.ds",
        "test.main.settle",
        r#"
function test.main.settle(v0: int32, v1: int32, v2: boolean): int32 {
    local l0: int32
    local l1: int32
    local l2: boolean

entry(v0: int32, v1: int32, v2: boolean):
    local.set l0, v0
    local.set l1, v1
    local.set l2, v2
    v3: int32 = local.get l0
    v4: int32 = local.get l1
    v5: boolean = local.get l2
    v6: int32 = call test.main.retry<int32>(v3, v4, v5): (int32, int32, boolean) => int32
    return v6
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
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

    session.assert_mir_function(
        "main.ds",
        "test.main.choose",
        r#"
function test.main.choose(v0: int32, v1: int32, v2: boolean): int32 {
    local l0: int32
    local l1: int32
    local l2: boolean

entry(v0: int32, v1: int32, v2: boolean):
    local.set l0, v0
    local.set l1, v1
    local.set l2, v2
    v3: int32 = local.get l0
    v4: int32 = local.get l1
    v5: boolean = local.get l2
    v6: int32 = call test.lib.pick<int32>(v3, v4, v5): (int32, int32, boolean) => int32
    return v6
}
"#,
    );

    session.assert_mir_lowered(
        "lib.ds",
        r#"
function test.lib.pick<T>(v0: T, v1: T, v2: boolean): T {
    local l0: T
    local l1: T
    local l2: boolean

entry(v0: T, v1: T, v2: boolean):
    local.set l0, v0
    local.set l1, v1
    local.set l2, v2
    v3: boolean = local.get l2
    branch v3 => b1 | b2

b1:
    v4: T = local.get l0
    return v4

b2:
    v5: T = local.get l1
    return v5
}
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

    session.assert_mir_function(
        "main.ds",
        "test.main.settle",
        r#"
function test.main.settle(v0: int32, v1: int32, v2: boolean): int32 {
    local l0: int32
    local l1: int32
    local l2: boolean

entry(v0: int32, v1: int32, v2: boolean):
    local.set l0, v0
    local.set l1, v1
    local.set l2, v2
    v3: int32 = local.get l0
    v4: int32 = local.get l1
    v5: boolean = local.get l2
    v6: int32 = call test.lib.retry<int32>(v3, v4, v5): (int32, int32, boolean) => int32
    return v6
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
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
    local l0: int32
    local l1: int32
    local l2: boolean

entry(v0: int32, v1: int32, v2: boolean):
    local.set l0, v0
    local.set l1, v1
    local.set l2, v2
    v3: int32 = local.get l0
    v4: int32 = local.get l1
    v5: boolean = local.get l2
    v6: int32 = call app.main.pick<int32>(v3, v4, v5): (int32, int32, boolean) => int32
    return v6
}

function app.main.pick<T>(v0: T, v1: T, v2: boolean): T {
    local l0: T
    local l1: T
    local l2: boolean

entry(v0: T, v1: T, v2: boolean):
    local.set l0, v0
    local.set l1, v1
    local.set l2, v2
    v3: boolean = local.get l2
    branch v3 => b1 | b2

b1:
    v4: T = local.get l0
    return v4

b2:
    v5: T = local.get l1
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

    session.assert_mir_function("main.ds", "test.main.apply", r#"
function test.main.apply(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32): int32 {
    local l0: function<(int32) => int32, repeatable, managed, mutable, local>
    local l1: int32

entry(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32):
    local.set l0, v0
    local.set l1, v1
    v2: function<(int32) => int32, repeatable, managed, mutable, local> = local.get l0
    v3: int32 = local.get l1
    v4: function<(int32) => int32, repeatable, borrowed, 'managed, mutable, local> = cast.bit v2 -> function<(int32) => int32, repeatable, borrowed, 'managed, mutable, local>
    v5: int32 = call.indirect v4(v3): (int32) => int32
    return v5
}
"#);

    session.assert_mir_function("main.ds", "test.main.run", r#"
function test.main.run(): int32 {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind test.main.identity<int32>, v0
    v2: int32 = 7
    v3: int32 = call test.main.apply(v1, v2): (function<(int32) => int32, repeatable, managed, mutable, local>, int32) => int32
    return v3
}

/// @layout.variant name=type@25 size=8 align=8
/// @layout.discriminant owner=type@25 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@25 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@25 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function(
        "main.ds",
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

    session.assert_mir_function("main.ds", "test.main.notify", r#"
type test.lib.Channel;

function test.main.notify<'a>(v0: ref<test.lib.Channel, borrowed, 'a, mutable, local>): void {
    local l0: ref<test.lib.Channel, borrowed, 'a, mutable, local>

entry(v0: ref<test.lib.Channel, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<test.lib.Channel, borrowed, 'a, mutable, local> = local.get l0
    v2: int32 = 1
    v3: ref<test.lib.Channel, managed, mutable, local> = load v1
    call test.lib.Channel.send<int32>(v3, v2): (ref<test.lib.Channel, managed, mutable, local>, int32) => void
    return
}
"#);

    session.assert_mir_function("main.ds", "test.lib.Channel.send<int32>", r#"
type test.lib.Channel;

shared function test.lib.Channel.send<int32>(v0: ref<test.lib.Channel, managed, mutable, local>, v1: int32): void;
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

    session.assert_mir_function("main.ds", "test.main.unwrap", r#"
type test.lib.Box<T>;

function test.main.unwrap<'a>(v0: ref<test.lib.Box<int32>, borrowed, 'a, mutable, local>): int32 {
    local l0: ref<test.lib.Box<int32>, borrowed, 'a, mutable, local>

entry(v0: ref<test.lib.Box<int32>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<test.lib.Box<int32>, borrowed, 'a, mutable, local> = local.get l0
    v2: ref<test.lib.Box<int32>, managed, mutable, local> = load v1
    v3: int32 = call test.lib.Box.read<int32>(v2): (ref<test.lib.Box<int32>, managed, mutable, local>) => int32
    return v3
}
"#);

    session.assert_mir_function(
        "main.ds",
        "test.lib.Box.read<int32>",
        r#"
type test.lib.Box<T>;

shared function test.lib.Box.read<int32>(v0: ref<test.lib.Box<int32>, managed, mutable, local>): int32;
"#,
    );
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

    session.assert_mir_function("main.ds", "test.main.drain", r#"
type test.lib.Tap<T>;

type test.lib.Source<T>;

function test.main.drain<'a>(v0: ref<test.lib.Tap<int32>, borrowed, 'a, mutable, local>): int32 {
    local l0: ref<test.lib.Tap<int32>, borrowed, 'a, mutable, local>

entry(v0: ref<test.lib.Tap<int32>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<test.lib.Tap<int32>, borrowed, 'a, mutable, local> = local.get l0
    v2: ref<test.lib.Tap<int32>, managed, mutable, local> = load v1
    v3: int32 = call test.lib.Source.read<int32>(v2): (ref<test.lib.Source<int32>, managed, mutable, local>) => int32
    return v3
}
"#);

    session.assert_mir_function("main.ds", "test.lib.Source.read<int32>", r#"
type test.lib.Source<T>;

shared function test.lib.Source.read<int32>(v0: ref<test.lib.Source<int32>, managed, mutable, local>): int32;
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

    session.assert_mir_function("main.ds", "test.main.run", r#"
@copy
type test.lib.AId;

@copy
type test.lib.ARef;

@copy
type test.lib.Pair<T>;

@copy
type test.lib.BId;

@copy
type test.lib.BRef;

function test.main.run(): int32 {
    local l0: test.lib.Pair<test.lib.ARef>
    local l1: test.lib.Pair<test.lib.BRef>

entry:
    v0: int32 = 1
    v1: test.lib.AId = aggregate (v0)
    v2: test.lib.ARef = aggregate (v1)
    v3: test.lib.Pair<test.lib.ARef> = call test.lib.wrap<test.lib.ARef>(v2): (test.lib.ARef) => test.lib.Pair<test.lib.ARef>
    local.set l0, v3
    v4: int32 = 2
    v5: test.lib.BId = aggregate (v4)
    v6: test.lib.BRef = aggregate (v5)
    v7: test.lib.Pair<test.lib.BRef> = call test.lib.wrap<test.lib.BRef>(v6): (test.lib.BRef) => test.lib.Pair<test.lib.BRef>
    local.set l1, v7
    v8: int32 = 0
    return v8
}
"#);

    session.assert_mir_function(
        "main.ds",
        "test.lib.wrap<test.lib.ARef>",
        r#"
@copy
type test.lib.ARef;

@copy
type test.lib.Pair<T>;

shared function test.lib.wrap<test.lib.ARef>(v0: test.lib.ARef): test.lib.Pair<test.lib.ARef>;
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.lib.wrap<test.lib.BRef>",
        r#"
@copy
type test.lib.Pair<T>;

@copy
type test.lib.BRef;

shared function test.lib.wrap<test.lib.BRef>(v0: test.lib.BRef): test.lib.Pair<test.lib.BRef>;
"#,
    );
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

    session.assert_mir_function("main.ds", "test.main.build", r#"
@copy
type test.lib.Wrap<'a, T>;

function test.main.build<'a>(v0: ref<int32, borrowed, 'a, readonly, local>): test.lib.Wrap<'a & local, int32> {
    local l0: ref<int32, borrowed, 'a, readonly, local>

entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<int32, borrowed, 'a, readonly, local> = local.get l0
    v2: test.lib.Wrap<'a & local, int32> = call test.lib.Wrap.make<int32>(v1): <'a>(ref<int32, borrowed, 'a, readonly, local>) => test.lib.Wrap<'a & local, int32>
    return v2
}
"#);

    session.assert_mir_function(
        "main.ds",
        "test.lib.Wrap.make<int32>",
        r#"
@copy
type test.lib.Wrap<'a, T>;

shared function test.lib.Wrap.make<int32, 'a>(v0: ref<int32, borrowed, 'a, readonly, local>): test.lib.Wrap<'a & local, int32>;
"#,
    );
}

#[test]
fn test_call_a_witness_member_at_a_closed_newtype_argument() {
    let session = TestSession::single(
        r#"
import { IoError } from "destack:error";

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

function take(reader: &Reader): void {
    let task = reader.read();
    const value = task.park();
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.take",
        r#"
@copy
@languageItem("error.IoError")
type IoError;

@copy
@languageItem("error.Result")
type Result<T, E>;

type test.main.Task<T> {
    value: T;
}

type test.main.Reader { }

function test.main.take<'a>(v0: ref<test.main.Reader, borrowed, 'a, mutable, local>): void {
    local l0: ref<test.main.Reader, borrowed, 'a, mutable, local>
    local l1: test.main.Task<Result<usize, IoError>>
    local l2: Result<usize, IoError>

entry(v0: ref<test.main.Reader, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<test.main.Reader, borrowed, 'a, mutable, local> = local.get l0
    v2: ref<test.main.Reader, managed, mutable, local> = load v1
    v3: test.main.Task<Result<usize, IoError>> = call.dynamic v2, test.main.Reader, 0(): () => test.main.Task<Result<usize, IoError>>
    local.set l1, v3
    v4: ref<test.main.Task<Result<usize, IoError>>, borrowed, 'frame, mutable, local> = local.address l1
    v5: Result<usize, IoError> = call test.main.Task.park<Result<usize, IoError>>(v4): <'a>(ref<test.main.Task<Result<usize, IoError>>, borrowed, 'a, mutable, local>) => Result<usize, IoError>
    local.set l2, v5
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
        "main.ds",
        "test.main.Cell.index",
        r#"
@copy
type test.main.Cell<T> {
    value: T;
}

function test.main.Cell.index<T, access A, 'a>(v0: ref<test.main.Cell<T>, borrowed, 'a, A, local>): variant<uint1> { 0uint1 = void; 1uint1 = ref<T, borrowed, 'a, A, local>; } {
    local l0: ref<test.main.Cell<T>, borrowed, 'a, A, local>

entry(v0: ref<test.main.Cell<T>, borrowed, 'a, A, local>):
    local.set l0, v0
    v1: ref<test.main.Cell<T>, borrowed, 'a, A, local> = local.get l0
    v2: variant<uint1> { 0uint1 = void; 1uint1 = ref<T, borrowed, 'a, A, local>; } = call test.main.Cell.get<T, A>(v1): <'a>(ref<test.main.Cell<T>, borrowed, 'a, A, local>) => variant<uint1> { 0uint1 = void; 1uint1 = ref<T, borrowed, 'a, A, local>; }
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
        "main.ds",
        "test.main.Cell.index",
        r#"
@copy
type test.main.Cell<T> {
    value: T;
}

function test.main.Cell.index<T, access A, 'a>(v0: ref<test.main.Cell<T>, borrowed, 'a, A, local>): variant<uint1> { 0uint1 = void; 1uint1 = ref<T, borrowed, 'a, A, local>; } {
    local l0: ref<test.main.Cell<T>, borrowed, 'a, A, local>

entry(v0: ref<test.main.Cell<T>, borrowed, 'a, A, local>):
    local.set l0, v0
    v1: ref<test.main.Cell<T>, borrowed, 'a, A, local> = local.get l0
    v2: variant<uint1> { 0uint1 = void; 1uint1 = ref<T, borrowed, 'a, A, local>; } = call test.main.Cell.get<T, A>(v1): <'a>(ref<test.main.Cell<T>, borrowed, 'a, A, local>) => variant<uint1> { 0uint1 = void; 1uint1 = ref<T, borrowed, 'a, A, local>; }
    return v2
}
"#,
    );
}

#[test]
fn test_copy_a_parameter_through_its_integer_bound() {
    let session = TestSession::single(
        r#"
import { Integer } from "destack:math";

export function twice<T: Integer>(value: &T): T {
    const first: T = *value;
    const second: T = *value;
    return first;
}
"#,
    );
    session.assert_mir_lowered(
        "main.ds",
        r#"
@languageItem("memory.Concrete")
type Concrete { }

@languageItem("memory.Clone")
type Clone { }

@languageItem("memory.Copy")
type Copy extends Clone { }

@languageItem("math.IntegerDomain")
type IntegerDomain { }

@languageItem("math.Zero")
type Zero { }

@languageItem("math.One")
type One { }

@languageItem("math.Integer")
type Integer extends Concrete, Copy, IntegerDomain, Zero, One { }

function test.main.twice<T: Integer, 'a>(v0: ref<T, borrowed, 'a, mutable, local>): T {
    local l0: ref<T, borrowed, 'a, mutable, local>
    local l1: T
    local l2: T

entry(v0: ref<T, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<T, borrowed, 'a, mutable, local> = local.get l0
    v2: T = load v1
    local.set l1, v2
    v3: ref<T, borrowed, 'a, mutable, local> = local.get l0
    v4: T = load v3
    local.set l2, v4
    v5: T = local.get l1
    return v5
}

/// @layout.struct name=Concrete size=0 align=1
/// @layout.struct name=Clone size=0 align=1
/// @layout.struct name=Copy size=0 align=1
/// @layout.struct name=IntegerDomain size=0 align=1
/// @layout.struct name=Zero size=0 align=1
/// @layout.struct name=One size=0 align=1
/// @layout.struct name=Integer size=0 align=1

/// @dispatch.shape constraint=type@2 function=clone function=cloneFrom function=zero function=one
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
        "main.ds",
        "test.main.Dup.dupFrom",
        r#"
type test.main.Dup { }

function test.main.Dup.dupFrom<this: test.main.Dup, 'a, 'b>(v0: ref<this, borrowed, 'a, mutable, local>, v1: ref<this, borrowed, 'b, readonly, local>): void {
    local l0: ref<this, borrowed, 'b, readonly, local>
    local l1: ref<this, borrowed, 'a, mutable, local>

entry(v0: ref<this, borrowed, 'a, mutable, local>, v1: ref<this, borrowed, 'b, readonly, local>):
    local.set l0, v1
    local.set l1, v0
    v2: ref<this, borrowed, 'a, mutable, local> = local.get l1
    v3: ref<this, borrowed, 'b, readonly, local> = local.get l0
    v4: this = call.witness this, test.main.Dup, test.main.Dup.dup(v3): <'a>(ref<this, borrowed, 'a, readonly, local>) => this
    store v2, v4
    return
}

/// @layout.struct name=test.main.Dup size=0 align=1
"#,
    );
}

#[test]
fn test_replace_through_an_exclusive_reference_at_an_open_type() {
    let session = TestSession::single(
        r#"
import { replace } from "destack:memory";

struct Cell<T> {
    value: T;
}

export extension<T> of Cell<T> {
    index(&this): &T {
        return &this.value;
    }

    set(&this, value: T): void {
        replace(this.index(), value);
    }
}
"#,
    );
    session.assert_mir_function(
        "main.ds",
        "test.main.Cell.set",
        r#"
@copy
type test.main.Cell<T> {
    value: T;
}

function test.main.Cell.set<T, 'a>(v0: ref<test.main.Cell<T>, borrowed, 'a, mutable, local>, v1: T): void {
    local l0: T
    local l1: ref<test.main.Cell<T>, borrowed, 'a, mutable, local>

entry(v0: ref<test.main.Cell<T>, borrowed, 'a, mutable, local>, v1: T):
    local.set l0, v1
    local.set l1, v0
    v2: ref<test.main.Cell<T>, borrowed, 'a, mutable, local> = local.get l1
    v3: ref<T, borrowed, 'a, mutable, local> = call test.main.Cell.index<T>(v2): <'a>(ref<test.main.Cell<T>, borrowed, 'a, mutable, local>) => ref<T, borrowed, 'a, mutable, local>
    v4: ptr<T, mutable> = cast.bit v3 -> ptr<T, mutable>
    v5: T = local.get l0
    v6: T = load v4
    store v4, v5
    return
}
"#,
    );
}

#[test]
fn test_address_a_slice_element_at_the_slice_access() {
    let session = TestSession::single(
        r#"
import { Slice } from "destack:collections";
import { Copy } from "destack:memory";

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
        "main.ds",
        "test.main.Slice.unsafeGet2",
        r#"
function test.main.Slice.unsafeGet2<T: Copy, 'a>(v0: slice<T, borrowed, 'a, readonly, local>, v1: usize): T {
    local l0: usize
    local l1: slice<T, borrowed, 'a, readonly, local>

entry(v0: slice<T, borrowed, 'a, readonly, local>, v1: usize):
    local.set l0, v1
    local.set l1, v0
    v2: slice<T, borrowed, 'a, readonly, local> = local.get l1
    v3: usize = local.get l0
    v4: ref<T, borrowed, 'a, readonly, local> = element.address v2, v3
    v5: T = load v4
    return v5
}
"#,
    );
}
