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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.weigh(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = intrinsic.math.bits.populationCount(v0)
    return v1
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
        "main.ds",
        r#"
/// @diagnostic.error id=unsupported-lower-construct message="MIR lowering does not support the 'time.warp' intrinsic"
/// @diagnostic.label file="main.ds"
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

    session.assert_mir_lowered(
        "main.ds",
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.pause(v0: int32): int32 {
entry(v0: int32):
    breakpoint
    return v0
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

    session.assert_mir_lowered(
        "main.ds",
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type MemoryOrdering = variant<int64> { 0int64 = void; 1int64 = void; 2int64 = void; 3int64 = void; 4int64 = void; };

@copy
type AtomicScope = variant<int64> { 0int64 = void; 1int64 = void; 2int64 = void; 3int64 = void; 4int64 = void; 5int64 = void; 6int64 = void; 7int64 = void; };

@copy
type MemoryScope = variant<int64> { 0int64 = void; };

@copy
type MemoryRegionSet = variant<int64> { 0int64 = void; };

function test.main.publish(): void {
entry:
    atomic.fence release
    return
}
/// @layout.variant name=MemoryOrdering size=8 align=8
/// @layout.discriminant owner=MemoryOrdering kind=direct offset=0 byte_len=8 bit_offset=0 bit_len=64
/// @layout.case owner=MemoryOrdering index=0 discriminant=0 payload_offset=8
/// @layout.case owner=MemoryOrdering index=1 discriminant=1 payload_offset=8
/// @layout.case owner=MemoryOrdering index=2 discriminant=2 payload_offset=8
/// @layout.case owner=MemoryOrdering index=3 discriminant=3 payload_offset=8
/// @layout.case owner=MemoryOrdering index=4 discriminant=4 payload_offset=8
/// @layout.variant name=AtomicScope size=8 align=8
/// @layout.discriminant owner=AtomicScope kind=direct offset=0 byte_len=8 bit_offset=0 bit_len=64
/// @layout.case owner=AtomicScope index=0 discriminant=0 payload_offset=8
/// @layout.case owner=AtomicScope index=1 discriminant=1 payload_offset=8
/// @layout.case owner=AtomicScope index=2 discriminant=2 payload_offset=8
/// @layout.case owner=AtomicScope index=3 discriminant=3 payload_offset=8
/// @layout.case owner=AtomicScope index=4 discriminant=4 payload_offset=8
/// @layout.case owner=AtomicScope index=5 discriminant=5 payload_offset=8
/// @layout.case owner=AtomicScope index=6 discriminant=6 payload_offset=8
/// @layout.case owner=AtomicScope index=7 discriminant=7 payload_offset=8
/// @layout.variant name=MemoryScope size=8 align=8
/// @layout.discriminant owner=MemoryScope kind=direct offset=0 byte_len=8 bit_offset=0 bit_len=64
/// @layout.case owner=MemoryScope index=0 discriminant=0 payload_offset=8
/// @layout.variant name=MemoryRegionSet size=8 align=8
/// @layout.discriminant owner=MemoryRegionSet kind=direct offset=0 byte_len=8 bit_offset=0 bit_len=64
/// @layout.case owner=MemoryRegionSet index=0 discriminant=0 payload_offset=8
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.clamp(v0: int32): int8 {
entry(v0: int32):
    v1: int8 = cast.saturate v0 -> int8
    return v1
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type MemoryOrdering = variant<int64> { 0int64 = void; 1int64 = void; 2int64 = void; 3int64 = void; 4int64 = void; };

@copy
type AtomicScope = variant<int64> { 0int64 = void; 1int64 = void; 2int64 = void; 3int64 = void; 4int64 = void; 5int64 = void; 6int64 = void; 7int64 = void; };

@copy
type MemoryScope = variant<int64> { 0int64 = void; };

@copy
type MemoryRegionSet = variant<int64> { 0int64 = void; };

function test.main.acquireAll(): void {
entry:
    atomic.fence acquire
    return
}
/// @layout.variant name=MemoryOrdering size=8 align=8
/// @layout.discriminant owner=MemoryOrdering kind=direct offset=0 byte_len=8 bit_offset=0 bit_len=64
/// @layout.case owner=MemoryOrdering index=0 discriminant=0 payload_offset=8
/// @layout.case owner=MemoryOrdering index=1 discriminant=1 payload_offset=8
/// @layout.case owner=MemoryOrdering index=2 discriminant=2 payload_offset=8
/// @layout.case owner=MemoryOrdering index=3 discriminant=3 payload_offset=8
/// @layout.case owner=MemoryOrdering index=4 discriminant=4 payload_offset=8
/// @layout.variant name=AtomicScope size=8 align=8
/// @layout.discriminant owner=AtomicScope kind=direct offset=0 byte_len=8 bit_offset=0 bit_len=64
/// @layout.case owner=AtomicScope index=0 discriminant=0 payload_offset=8
/// @layout.case owner=AtomicScope index=1 discriminant=1 payload_offset=8
/// @layout.case owner=AtomicScope index=2 discriminant=2 payload_offset=8
/// @layout.case owner=AtomicScope index=3 discriminant=3 payload_offset=8
/// @layout.case owner=AtomicScope index=4 discriminant=4 payload_offset=8
/// @layout.case owner=AtomicScope index=5 discriminant=5 payload_offset=8
/// @layout.case owner=AtomicScope index=6 discriminant=6 payload_offset=8
/// @layout.case owner=AtomicScope index=7 discriminant=7 payload_offset=8
/// @layout.variant name=MemoryScope size=8 align=8
/// @layout.discriminant owner=MemoryScope kind=direct offset=0 byte_len=8 bit_offset=0 bit_len=64
/// @layout.case owner=MemoryScope index=0 discriminant=0 payload_offset=8
/// @layout.variant name=MemoryRegionSet size=8 align=8
/// @layout.discriminant owner=MemoryRegionSet kind=direct offset=0 byte_len=8 bit_offset=0 bit_len=64
/// @layout.case owner=MemoryRegionSet index=0 discriminant=0 payload_offset=8
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.bump(v0: ptr<int32, mutable>): void {
entry(v0: ptr<int32, mutable>):
    v1: int32 = load v0
    store v0, v1
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.mirror(v0: ptr<int32, mutable>): void {
entry(v0: ptr<int32, mutable>):
    v1: int32 = intrinsic.memory.ptr.readVolatile(v0)
    intrinsic.memory.ptr.writeVolatile(v0, v1)
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.exchange(v0: ptr<int32, mutable>, v1: int32): int32 {
entry(v0: ptr<int32, mutable>, v1: int32):
    v2: int32 = load v0
    store v0, v1
    return v2
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.flip(v0: ptr<int32, mutable>, v1: ptr<int32, mutable>): void {
entry(v0: ptr<int32, mutable>, v1: ptr<int32, mutable>):
    v2: int32 = load v0
    v3: int32 = load v1
    store v0, v3
    store v1, v2
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.destroy(v0: ptr<int32, mutable>): void {
entry(v0: ptr<int32, mutable>):
    v1: int32 = load v0
    drop v1
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Pair {
    low: int32;
    high: int64;
}

function test.main.measure(): usize {
entry:
    v0: uint64 = 16
    v1: uint64 = 8
    v2: uint64 = int.add v0, v1
    v3: uint64 = 16
    v4: uint64 = int.add v2, v3
    return v4
}
/// @layout.struct name=Pair size=16 align=8
/// @layout.field owner=Pair index=0 name=low offset=8 size=4 align=4
/// @layout.field owner=Pair index=1 name=high offset=0 size=8 align=8
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.empty(): ptr<int64, mutable> {
entry:
    v0: uint64 = 8
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.distance(v0: ptr<int32, mutable>, v1: ptr<int32, mutable>): int64 {
entry(v0: ptr<int32, mutable>, v1: ptr<int32, mutable>):
    v2: int64 = 2
    v3: int64 = 4
    v4: int64 = intrinsic.memory.raw.transmute(v0)
    v5: int64 = int.mul v2, v3
    v6: int64 = int.add v4, v5
    v7: ptr<int32, mutable> = intrinsic.memory.raw.transmute(v6)
    v8: int64 = 4
    v9: int64 = intrinsic.memory.ptr.byteOffsetFrom(v7, v1)
    v10: int64 = int.div.s v9, v8
    return v10
}
"#,
    );
}

#[test]
fn test_lower_storage_initialization_to_carrier_constants() {
    let session = TestSession::single(
        r#"
@languageItem("memory.MaybeUninit")
newtype MaybeUninit<out T> = intrinsic;

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

    session.assert_mir_lowered(
        "main.ds",
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
fn test_lower_manually_drop_wrapping_to_transmutes() {
    let session = TestSession::single(
        r#"
@languageItem("memory.ManuallyDrop")
newtype ManuallyDrop<out T> = intrinsic;

@intrinsic("memory.manuallyDrop.new")
declare function newManuallyDrop<T>(value: T): ManuallyDrop<T>;

@intrinsic("memory.manuallyDrop.intoInner")
declare function intoManuallyDropInner<T>(wrapped: ManuallyDrop<T>): T;

function wrap(value: int64): int64 {
    return intoManuallyDropInner<int64>(newManuallyDrop<int64>(value));
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.wrap(v0: int64): int64 {
entry(v0: int64):
    v1: manual<int64> = intrinsic.memory.raw.transmute(v0)
    v2: int64 = intrinsic.memory.raw.transmute(v1)
    return v2
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Context { }

type Variable { }

function test.main.scope(v0: ref<Variable, managed, mutable>, v1: int32): int32 {
entry(v0: ref<Variable, managed, mutable>, v1: int32):
    v2: ref<Context, managed, mutable> = context.current
    v3: ref<Context, managed, mutable> = context.bind v2, v0, v1, { parent: ref<Context, managed, mutable>, variable: ref<Variable, managed, mutable>, value: int32 }
    v4: ref<Context, managed, mutable> = context.replace v3
    v5: int32 = context.get v3, v0, v1, { parent: ref<Context, managed, mutable>, variable: ref<Variable, managed, mutable>, value: int32 }
    v6: ref<Context, managed, mutable> = context.replace v4
    return v5
}
/// @layout.struct name=Context size=0 align=1
/// @layout.struct name=Variable size=0 align=1
/// @layout.struct name=type@14 size=24 align=8
/// @layout.field owner=type@14 index=0 name=parent offset=0 size=8 align=8
/// @layout.field owner=type@14 index=1 name=variable offset=8 size=8 align=8
/// @layout.field owner=type@14 index=2 name=value offset=16 size=4 align=4
"#,
    );
}
