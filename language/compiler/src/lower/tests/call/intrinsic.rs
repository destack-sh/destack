use crate::tests::TestSession;

#[test]
fn test_lower_named_intrinsic_to_its_machine_operation() {
    let session = TestSession::single(
        r#"
@intrinsic("math.bits.populationCount")
declare function populationCount(value: int32): int32;

function weigh(value: int32): int32 {
    return populationCount(value);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.weigh",
        r#"
function test.main.weigh(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = intrinsic.math.bits.populationCount(v1)
    return v2
}
"#,
    );
}

#[test]
fn test_reject_an_unknown_intrinsic_name_loudly() {
    let session = TestSession::single(
        r#"
@intrinsic("time.warp")
declare function warp(value: int32): int32;

function bend(value: int32): int32 {
    return warp(value);
}
"#,
    );

    session.assert_mir_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=unsupported-lower-construct message="unsupported construct: the 'time.warp' intrinsic"
/// @diagnostic.label line=6 column=12 span="warp(value)" line_source="return warp(value);"
"#,
    );
}

#[test]
fn test_lower_trap_intrinsic_to_an_abort_terminator() {
    let session = TestSession::single(
        r#"
@intrinsic("error.trap")
declare function trap(): never;

function halt(): int32 {
    trap();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.halt",
        r#"
function test.main.halt(): int32 {
entry:
    abort

b1:
    return
}
"#,
    );
}

#[test]
fn test_lower_breakpoint_intrinsic_to_its_instruction() {
    let session = TestSession::single(
        r#"
@intrinsic("error.debug.breakpoint")
declare function breakpoint(): void;

function pause(value: int32): int32 {
    breakpoint();
    return value;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.pause",
        r#"
function test.main.pause(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    breakpoint
    v1: int32 = load l0
    return v1
}
"#,
    );
}

#[test]
fn test_lower_spin_loop_intrinsic_without_a_result() {
    let session = TestSession::single(
        r#"
@intrinsic("hint.spinLoop")
declare function spinLoop(): void;

function wait(): void {
    spinLoop();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.wait",
        r#"
function test.main.wait(): void {
entry:
    intrinsic.hint.spinLoop()
    return
}
"#,
    );
}

#[test]
fn test_lower_atomic_fence_with_const_ordering() {
    let session = TestSession::single(
        r#"
enum MemoryOrdering {
    Relaxed = 0,
    Acquire = 1,
    Release = 2,
    AcquireRelease = 3,
    SequentiallyConsistent = 4,
}

enum AtomicScope {
    Invocation = 0,
    Subgroup = 1,
    Workgroup = 2,
    Device = 3,
    CrossDevice = 4,
    QueueFamily = 5,
    ShaderCallGroup = 6,
    System = 7,
}

enum MemoryScope {
    Local = 0,
}

enum MemoryRegionSet {
    All = 0,
}

@intrinsic("sync.atomic.fence")
declare function atomicFence(
    order: MemoryOrdering,
    scope: AtomicScope,
    memoryScope: MemoryScope,
    regions: MemoryRegionSet,
    isVolatile: boolean,
    isMakeAvailable: boolean,
    isMakeVisible: boolean,
): void;

function publish(): void {
    atomicFence(
        MemoryOrdering.Release,
        AtomicScope.System,
        MemoryScope.Local,
        MemoryRegionSet.All,
        false,
        false,
        false,
    );
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.publish",
        r#"
function test.main.publish(): void {
entry:
    atomic.fence release
    return
}
"#,
    );
}

#[test]
fn test_lower_saturating_cast_intrinsic_to_its_operator() {
    let session = TestSession::single(
        r#"
@intrinsic("math.cast.int.saturate")
declare function saturate(value: int32): int8;

function clamp(value: int32): int8 {
    return saturate(value);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.clamp",
        r#"
function test.main.clamp(v0: int32): int8 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int8 = cast.intToIntSaturating v1 -> int8
    return v2
}
"#,
    );
}

#[test]
fn test_lower_atomic_fence_with_auto_numbered_orderings() {
    let session = TestSession::single(
        r#"
enum MemoryOrdering {
    Relaxed,
    Acquire,
    Release,
    AcquireRelease,
    SequentiallyConsistent,
}

enum AtomicScope {
    Invocation,
    Subgroup,
    Workgroup,
    Device,
    CrossDevice,
    QueueFamily,
    ShaderCallGroup,
    System,
}

enum MemoryScope {
    Local,
}

enum MemoryRegionSet {
    All,
}

@intrinsic("sync.atomic.fence")
declare function atomicFence(
    order: MemoryOrdering,
    scope: AtomicScope,
    memoryScope: MemoryScope,
    regions: MemoryRegionSet,
    isVolatile: boolean,
    isMakeAvailable: boolean,
    isMakeVisible: boolean,
): void;

function acquireAll(): void {
    atomicFence(
        MemoryOrdering.Acquire,
        AtomicScope.System,
        MemoryScope.Local,
        MemoryRegionSet.All,
        false,
        false,
        false,
    );
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.acquireAll",
        r#"
function test.main.acquireAll(): void {
entry:
    atomic.fence acquire
    return
}
"#,
    );
}

#[test]
fn test_lower_pointer_read_and_write_to_loads_and_stores() {
    let session = TestSession::single(
        r#"
@intrinsic("memory.ptr.read")
declare function read<T>(source: *T): ^T;

@intrinsic("memory.ptr.write")
declare function write<T>(destination: *T, value: ^T): void;

function bump(pointer: *int32): void {
    write<int32>(pointer, read<int32>(pointer));
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.bump",
        r#"
function test.main.bump(v0: ptr<int32, mutable>): void {
    local l0: ptr<int32, mutable>

entry(v0: ptr<int32, mutable>):
    store l0, v0
    v1: ptr<int32, mutable> = load l0
    v2: ptr<int32, mutable> = load l0
    v3: int32 = load (*v2)
    store (*v1), v3
    return
}
"#,
    );
}

#[test]
fn test_lower_volatile_pointer_access_to_its_operations() {
    let session = TestSession::single(
        r#"
@intrinsic("memory.ptr.readVolatile")
declare function readVolatile<T: Copy>(source: *T): ^T;

@intrinsic("memory.ptr.writeVolatile")
declare function writeVolatile<T: Copy>(destination: *T, value: ^T): void;

function mirror(pointer: *int32): void {
    writeVolatile<int32>(pointer, readVolatile<int32>(pointer));
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.mirror",
        r#"
function test.main.mirror(v0: ptr<int32, mutable>): void {
    local l0: ptr<int32, mutable>

entry(v0: ptr<int32, mutable>):
    store l0, v0
    v1: ptr<int32, mutable> = load l0
    v2: ptr<int32, mutable> = load l0
    v3: int32 = intrinsic.memory.ptr.readVolatile(v2)
    intrinsic.memory.ptr.writeVolatile(v1, v3)
    return
}
"#,
    );
}

#[test]
fn test_lower_pointer_replace_returning_the_old_value() {
    let session = TestSession::single(
        r#"
@intrinsic("memory.ptr.replace")
declare function replace<T>(destination: *T, value: ^T): ^T;

function exchange(pointer: *int32, value: int32): int32 {
    return replace<int32>(pointer, value);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.exchange",
        r#"
function test.main.exchange(v0: ptr<int32, mutable>, v1: int32): int32 {
    local l0: ptr<int32, mutable>
    local l1: int32

entry(v0: ptr<int32, mutable>, v1: int32):
    store l0, v0
    store l1, v1
    v2: ptr<int32, mutable> = load l0
    v3: int32 = load l1
    v4: int32 = load (*v2)
    store (*v2), v3
    return v4
}
"#,
    );
}

#[test]
fn test_lower_pointer_swap_with_paired_loads_and_stores() {
    let session = TestSession::single(
        r#"
@intrinsic("memory.ptr.swap")
declare function swap<T>(a: *T, b: *T): void;

function flip(first: *int32, second: *int32): void {
    swap<int32>(first, second);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.flip",
        r#"
function test.main.flip(v0: ptr<int32, mutable>, v1: ptr<int32, mutable>): void {
    local l0: ptr<int32, mutable>
    local l1: ptr<int32, mutable>

entry(v0: ptr<int32, mutable>, v1: ptr<int32, mutable>):
    store l0, v0
    store l1, v1
    v2: ptr<int32, mutable> = load l0
    v3: ptr<int32, mutable> = load l1
    v4: int32 = load (*v2)
    v5: int32 = load (*v3)
    store (*v2), v5
    store (*v3), v4
    return
}
"#,
    );
}

#[test]
fn test_lower_pointer_drop_in_place_to_a_drop() {
    let session = TestSession::single(
        r#"
@intrinsic("memory.ptr.dropInPlace")
declare function dropInPlace<T>(destination: *T): void;

function destroy(pointer: *int32): void {
    dropInPlace<int32>(pointer);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.destroy",
        r#"
function test.main.destroy(v0: ptr<int32, mutable>): void {
    local l0: ptr<int32, mutable>

entry(v0: ptr<int32, mutable>):
    store l0, v0
    v1: ptr<int32, mutable> = load l0
    v2: int32 = load (*v1)
    drop v2
    return
}
"#,
    );
}

#[test]
fn test_lower_layout_queries_to_target_constants() {
    let session = TestSession::single(
        r#"
struct Pair {
    low: int32;
    high: int64;
}

@intrinsic("reflect.sizeOf")
declare function sizeOf<T>(): usize;

@intrinsic("reflect.alignOf")
declare function alignOf<T>(): usize;

@intrinsic("reflect.strideOf")
declare function strideOf<T>(): usize;

function measure(): usize {
    return sizeOf<Pair>() + alignOf<Pair>() + strideOf<Pair>();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.measure",
        r#"
type test.main.Pair {
    low: int32;
    high: int64;
}

function test.main.measure(): usize {
entry:
    v0: usize = size.of test.main.Pair
    v1: usize = align.of test.main.Pair
    v2: usize = add v0, v1
    v3: usize = stride.of test.main.Pair
    v4: usize = add v2, v3
    return v4
}

/// @layout.struct name=test.main.Pair size=16 align=8
/// @layout.field owner=test.main.Pair index=0 name=low offset=8 size=4 align=4
/// @layout.field owner=test.main.Pair index=1 name=high offset=0 size=8 align=8
"#,
    );
}

#[test]
fn test_lower_from_reference_to_a_pointer_transmute() {
    let session = TestSession::single(
        r#"
@intrinsic("memory.ptr.fromReference")
declare function fromReference<T>(reference: &readonly T): *T;

function locate(value: &readonly int64): *int64 {
    return fromReference(value);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.locate",
        r#"
function test.main.locate<'a>(v0: ref<int64, borrowed, 'a, readonly>): ptr<int64, mutable> {
    local l0: ref<int64, borrowed, 'a, readonly>

entry(v0: ref<int64, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<int64, borrowed, 'a, readonly> = load l0
    v2: ptr<int64, mutable> = cast.referenceToPointer v1 -> ptr<int64, mutable>
    return v2
}
"#,
    );
}

#[test]
fn test_lower_dangling_to_the_alignment_constant() {
    let session = TestSession::single(
        r#"
@intrinsic("memory.ptr.dangling")
declare function dangling<T>(): *T;

function empty(): *int64 {
    return dangling<int64>();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.empty",
        r#"
function test.main.empty(): ptr<int64, mutable> {
entry:
    v0: ptr<int64, mutable> = align.of int64
    v1: ptr<int64, mutable> = intrinsic.memory.raw.transmute(v0)
    return v1
}
"#,
    );
}

#[test]
fn test_lower_pointer_offset_to_stride_arithmetic() {
    let session = TestSession::single(
        r#"
@intrinsic("memory.ptr.offset")
declare function offset<T>(pointer: *T, count: int): *T;

@intrinsic("memory.ptr.offsetFrom")
declare function offsetFrom<T>(pointer: *T, origin: *T): int;

function distance(pointer: *int32, origin: *int32): int {
    return offsetFrom<int32>(offset<int32>(pointer, 2), origin);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.distance",
        r#"
function test.main.distance(v0: ptr<int32, mutable>, v1: ptr<int32, mutable>): int64 {
    local l0: ptr<int32, mutable>
    local l1: ptr<int32, mutable>

entry(v0: ptr<int32, mutable>, v1: ptr<int32, mutable>):
    store l0, v0
    store l1, v1
    v2: ptr<int32, mutable> = load l0
    v3: int64 = 2
    v4: int64 = stride.of int32
    v5: int64 = intrinsic.memory.raw.transmute(v2)
    v6: int64 = mul v3, v4
    v7: int64 = add v5, v6
    v8: ptr<int32, mutable> = intrinsic.memory.raw.transmute(v7)
    v9: ptr<int32, mutable> = load l1
    v10: int64 = stride.of int32
    v11: int64 = intrinsic.memory.ptr.byteOffsetFrom(v8, v9)
    v12: int64 = div v11, v10
    return v12
}
"#,
    );
}

#[test]
fn test_lower_storage_initialization_to_its_constants() {
    let session = TestSession::single(
        r#"
import { MaybeUninit } from "tspp:memory";

@intrinsic("memory.init.uninit")
declare function initUninit<T>(): MaybeUninit<T>;

@intrinsic("memory.init.zeroed")
declare function initZeroed<T>(): MaybeUninit<T>;

@intrinsic("memory.init.assumeInit")
declare function assumeInit<T>(storage: MaybeUninit<T>): T;

function build(): int64 {
    initUninit<int64>();
    return assumeInit<int64>(initZeroed<int64>());
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.build",
        r#"
function test.main.build(): int64 {
entry:
    v0: uninit<int64> = uninit
    v1: uninit<int64> = zeroed
    v2: int64 = intrinsic.memory.raw.transmute(v1)
    return v2
}
"#,
    );
}

#[test]
fn test_lower_manually_drop_conversions_to_transmutes() {
    let session = TestSession::single(
        r#"
import { ManuallyDrop } from "tspp:memory";

@intrinsic("memory.manuallyDrop.new")
declare function newManuallyDrop<T>(value: T): ManuallyDrop<T>;

@intrinsic("memory.manuallyDrop.intoInner")
declare function intoManuallyDropInner<T>(wrapped: ManuallyDrop<T>): T;

function wrap(value: int64): int64 {
    return intoManuallyDropInner<int64>(newManuallyDrop<int64>(value));
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.wrap",
        r#"
function test.main.wrap(v0: int64): int64 {
    local l0: int64

entry(v0: int64):
    store l0, v0
    v1: int64 = load l0
    v2: manual<int64> = intrinsic.memory.raw.transmute(v1)
    v3: int64 = intrinsic.memory.raw.transmute(v2)
    return v3
}
"#,
    );
}

#[test]
fn test_lower_execution_context_operations_to_their_instructions() {
    let session = TestSession::single(
        r#"
class Context {}

class Variable {}

@intrinsic("context.current")
declare function currentContext(): Context;

@intrinsic("context.replace")
declare function replaceContext(context: Context): Context;

@intrinsic("context.bind")
declare function bindContext(context: Context, variable: Variable, value: int32): Context;

@intrinsic("context.get")
declare function getContextValue(context: Context, variable: Variable, defaultValue: int32): int32;

function scope(variable: Variable, value: int32): int32 {
    const bound = bindContext(currentContext(), variable, value);
    const previous = replaceContext(bound);
    const read = getContextValue(bound, variable, value);
    replaceContext(previous);
    return read;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.scope", r#"
@nocopy
type test.main.Context { }

@nocopy
type test.main.Variable { }

function test.main.scope(v0: ref<test.main.Variable, managed, mutable, local>, v1: int32): int32 {
    local l0: ref<test.main.Variable, managed, mutable, local>
    local l1: int32
    local l2: ref<test.main.Context, managed, mutable, local>
    local l3: ref<test.main.Context, managed, mutable, local>
    local l4: int32

entry(v0: ref<test.main.Variable, managed, mutable, local>, v1: int32):
    store l0, v0
    store l1, v1
    v2: ref<test.main.Context, managed, mutable, local> = context.current
    v3: ref<test.main.Variable, managed, mutable, local> = load l0
    v4: int32 = load l1
    v5: ref<test.main.Context, managed, mutable, local> = context.bind v2, v3, v4, { parent: ref<test.main.Context, managed, mutable, local>, variable: ref<test.main.Variable, managed, mutable, local>, value: int32 }
    store l2, v5
    v6: ref<test.main.Context, managed, mutable, local> = load l2
    v7: ref<test.main.Context, managed, mutable, local> = context.replace v6
    store l3, v7
    v8: ref<test.main.Context, managed, mutable, local> = load l2
    v9: ref<test.main.Variable, managed, mutable, local> = load l0
    v10: int32 = load l1
    v11: int32 = context.get v8, v9, v10, { parent: ref<test.main.Context, managed, mutable, local>, variable: ref<test.main.Variable, managed, mutable, local>, value: int32 }
    store l4, v11
    v12: ref<test.main.Context, managed, mutable, local> = load l3
    v13: ref<test.main.Context, managed, mutable, local> = context.replace v12
    v14: int32 = load l4
    return v14
}

/// @layout.struct name=test.main.Context size=0 align=1
/// @layout.struct name=test.main.Variable size=0 align=1
/// @layout.struct name=type@6 size=24 align=8
/// @layout.field owner=type@6 index=0 name=parent offset=0 size=8 align=8
/// @layout.field owner=type@6 index=1 name=variable offset=8 size=8 align=8
/// @layout.field owner=type@6 index=2 name=value offset=16 size=4 align=4
"#);
}

#[test]
fn test_lower_profile_instruments_to_named_sites() {
    let session = TestSession::single(
        r#"
import { counter, sampler } from "tspp:profile";

const requests = counter("requests");
const latency = sampler("latency");

function observe(elapsed: number): void {
    requests.increment();
    requests.increment();
    latency.sample(elapsed);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.observe",
        r#"
function test.main.observe(v0: float64): void {
    local l0: float64

entry(v0: float64):
    store l0, v0
    profile.increment counter(0)
    profile.increment counter(0)
    v1: float64 = load l0
    profile.sample sampler(0), v1
    return
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.@init",
        r#"
type literal.string.requests { }

type literal.string.latency { }

export function test.main.@init(): void {
entry:
    v0: literal.string.requests = zeroed
    v1: void = zeroed
    store @test.main.requests, v1
    v2: literal.string.latency = zeroed
    v3: void = zeroed
    store @test.main.latency, v3
    return
}

/// @layout.struct name=literal.string.requests size=0 align=1
/// @layout.struct name=literal.string.latency size=0 align=1
"#,
    );
}

#[test]
fn test_lower_an_uninitialized_slice_allocation() {
    let session = TestSession::single(
        r#"
import { Slice } from "tspp:collections";
import { MaybeUninit } from "tspp:memory";

function reserve(count: usize): ^[MaybeUninit<int32>] {
    const storage: ^[MaybeUninit<int32>] = Slice.uninit(count);

    return storage;
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.reserve", r#"
function test.main.reserve(v0: usize): slice<uninit<int32>, unique, mutable> {
    local l0: usize
    local l1: slice<uninit<int32>, unique, mutable>

entry(v0: usize):
    store l0, v0
    v1: usize = load l0
    v2: slice<uninit<int32>, unique, mutable> = call Slice.uninit<int32>(v1): (usize) => slice<uninit<int32>, unique, mutable>
    store l1, v2
    v3: slice<uninit<int32>, unique, mutable> = load l1
    return v3
}
"#);
}

#[test]
fn test_lower_module_statements_into_the_initializer() {
    let session = TestSession::single(
        r#"
function tick(): int32 {
    return 1;
}

const first = tick();
tick();
const second = tick();
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.tick",
        r#"
function test.main.tick(): int32 {
entry:
    v0: int32 = 1
    return v0
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.@init",
        r#"
export function test.main.@init(): void {
entry:
    v0: int32 = call test.main.tick(): () => int32
    store @test.main.first, v0
    v1: int32 = call test.main.tick(): () => int32
    v2: int32 = call test.main.tick(): () => int32
    store @test.main.second, v2
    return
}
"#,
    );
}

/// The memory reinterpretation intrinsics lower to transmutes and zero-sized markers.
#[test]
fn test_lower_memory_reinterpretation_intrinsics() {
    let session = TestSession::single(
        r#"
import { ManuallyDrop, Phantom } from "tspp:memory";

struct Point {
    x: int32;
}

function wrap(point: Point): ManuallyDrop<Point> {
    ManuallyDrop.new(point)
}

function unwrap(wrapped: ManuallyDrop<Point>): Point {
    wrapped.intoInner()
}

function marker(): Phantom<Point> {
    Phantom.new()
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.wrap", r#"
type test.main.Point {
    x: int32;
}

function test.main.wrap(v0: test.main.Point): manual<test.main.Point> {
    local l0: test.main.Point

entry(v0: test.main.Point):
    store l0, v0
    v1: test.main.Point = load l0
    v2: manual<test.main.Point> = call ManuallyDrop.new<test.main.Point>(v1): (test.main.Point) => manual<test.main.Point>
    return v2
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#);
    session.assert_mir_function("main.tspp", "test.main.unwrap", r#"
type test.main.Point {
    x: int32;
}

function test.main.unwrap(v0: manual<test.main.Point>): test.main.Point {
    local l0: manual<test.main.Point>

entry(v0: manual<test.main.Point>):
    store l0, v0
    v1: manual<test.main.Point> = load l0
    v2: test.main.Point = call ManuallyDrop.intoInner<test.main.Point>(v1): (manual<test.main.Point>) => test.main.Point
    return v2
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#);
    session.assert_mir_function(
        "main.tspp",
        "test.main.marker",
        r#"
function test.main.marker(): void {
entry:
    call Phantom.new<test.main.Point>(): () => void
    v0: void = zeroed
    return v0
}
"#,
    );
}

/// The vector intrinsics lower to their lane instructions over the vector representation.
#[test]
fn test_lower_vector_intrinsics_to_lane_instructions() {
    let session = TestSession::single(
        r#"
import { Vector, Mask, splat, extract, insert, select, convert, less, reduceAdd } from "tspp:math";

function lanes(value: int32, index: uint32): int32 {
    const vector = splat<int32, 4>(value);
    const replaced = insert(vector, index, value);
    extract(replaced, index)
}

function pick(mask: Mask<4>, a: Vector<int32, 4>, b: Vector<int32, 4>): Vector<int32, 4> {
    select(mask, a, b)
}

function widen(a: Vector<int32, 4>): Vector<int64, 4> {
    convert<int64, int32, 4>(a)
}

function below(a: Vector<int32, 4>, b: Vector<int32, 4>): Mask<4> {
    less(a, b)
}

function total(a: Vector<int32, 4>): int32 {
    reduceAdd(a)
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.lanes",
        r#"
function test.main.lanes(v0: int32, v1: uint32): int32 {
    local l0: int32
    local l1: uint32
    local l2: vector<int32, 4>
    local l3: vector<int32, 4>

entry(v0: int32, v1: uint32):
    store l0, v0
    store l1, v1
    v2: int32 = load l0
    v3: vector<int32, 4> = vector.splat v2
    store l2, v3
    v4: vector<int32, 4> = load l2
    v5: uint32 = load l1
    v6: int32 = load l0
    v7: vector<int32, 4> = vector.insert v4, v5, v6
    store l3, v7
    v8: vector<int32, 4> = load l3
    v9: uint32 = load l1
    v10: int32 = vector.extract v8, v9
    return v10
}
"#,
    );
    session.assert_mir_function("main.tspp", "test.main.pick", r#"
function test.main.pick(v0: vector<boolean, 4>, v1: vector<int32, 4>, v2: vector<int32, 4>): vector<int32, 4> {
    local l0: vector<boolean, 4>
    local l1: vector<int32, 4>
    local l2: vector<int32, 4>

entry(v0: vector<boolean, 4>, v1: vector<int32, 4>, v2: vector<int32, 4>):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: vector<boolean, 4> = load l0
    v4: vector<int32, 4> = load l1
    v5: vector<int32, 4> = load l2
    v6: vector<int32, 4> = vector.select v3, v4, v5
    return v6
}
"#);
    session.assert_mir_function(
        "main.tspp",
        "test.main.widen",
        r#"
function test.main.widen(v0: vector<int32, 4>): vector<int64, 4> {
    local l0: vector<int32, 4>

entry(v0: vector<int32, 4>):
    store l0, v0
    v1: vector<int32, 4> = load l0
    v2: vector<int64, 4> = vector.convert exact, v1
    return v2
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.below",
        r#"
function test.main.below(v0: vector<int32, 4>, v1: vector<int32, 4>): vector<boolean, 4> {
    local l0: vector<int32, 4>
    local l1: vector<int32, 4>

entry(v0: vector<int32, 4>, v1: vector<int32, 4>):
    store l0, v0
    store l1, v1
    v2: vector<int32, 4> = load l0
    v3: vector<int32, 4> = load l1
    v4: vector<boolean, 4> = vector.compare lt, v2, v3
    return v4
}
"#,
    );
    session.assert_mir_function(
        "main.tspp",
        "test.main.total",
        r#"
function test.main.total(v0: vector<int32, 4>): int32 {
    local l0: vector<int32, 4>

entry(v0: vector<int32, 4>):
    store l0, v0
    v1: vector<int32, 4> = load l0
    v2: int32 = vector.reduce add, v1
    return v2
}
"#,
    );
}

/// A const ordering parameter reaches the atomic instruction as its own slot.
#[test]
fn test_lower_a_const_ordering_parameter_into_an_atomic_load() {
    let session = TestSession::single(
        r#"
import { MemoryOrdering, AtomicScope, MemoryScope, MemoryRegionSet, atomicLoad } from "tspp:sync";

function load<
    const Order:
        | MemoryOrdering.Relaxed
        | MemoryOrdering.Acquire
        | MemoryOrdering.SequentiallyConsistent = MemoryOrdering.SequentiallyConsistent,
>(
    ptr: *int32,
    order?: Order,
): int32 {
    atomicLoad(ptr, Order, AtomicScope.Device, MemoryScope.Device, MemoryRegionSet.Any, false, false, false)
}
"#,
    );

    session.assert_mir_lowered(
        "main.tspp", r#"
@languageItem("sync.MemoryOrdering")
type MemoryOrdering = variant<uint8> { 0uint8 = void; 1uint8 = void; 2uint8 = void; 3uint8 = void; 4uint8 = void; };

@nocopy
@languageItem("memory.Clone")
type Clone { }

function test.main.load<const Order: MemoryOrdering>(v0: ptr<int32, mutable>, v1: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }): int32 {
    local l0: ptr<int32, mutable>
    local l1: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }

entry(v0: ptr<int32, mutable>, v1: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }):
    store l0, v0
    store l1, v1
    v2: ptr<int32, mutable> = load l0
    v3: int32 = atomic.load (*v2), Order, scope(device)
    return v3
}

/// @dispatch.shape constraint=type@8 function=clone function=cloneFrom
"#,
    );
}
