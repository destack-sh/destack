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
function main.weigh(v0: int32): int32 {
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
/// @diagnostic.error id=unsupported-native-construct message="native compilation does not support the 'time.warp' intrinsic"
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
function main.halt(): int32 {
entry:
    trap.abort

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
function main.pause(v0: int32): int32 {
entry(v0: int32):
    breakpoint
    return v0
}
"#,
    );
}

#[test]
fn test_lower_atomic_fence_with_comptime_ordering() {
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
type MemoryOrdering = variant<int64, void> { 0int64 = void; 1int64 = void; 2int64 = void; 3int64 = void; 4int64 = void; };

@copy
type AtomicScope = variant<int64, void> { 0int64 = void; 1int64 = void; 2int64 = void; 3int64 = void; 4int64 = void; 5int64 = void; 6int64 = void; 7int64 = void; };

@copy
type MemoryScope = variant<int64, void> { 0int64 = void; };

@copy
type MemoryRegionSet = variant<int64, void> { 0int64 = void; };

function main.publish(): void {
entry:
    atomic.fence release
    return
}
/// @layout.variant name=MemoryOrdering size=8 align=8 encoding=direct(tag@0+8) cases=(0@8, 1@8, 2@8, 3@8, 4@8)
/// @layout.variant name=AtomicScope size=8 align=8 encoding=direct(tag@0+8) cases=(0@8, 1@8, 2@8, 3@8, 4@8, 5@8, 6@8, 7@8)
/// @layout.variant name=MemoryScope size=8 align=8 encoding=direct(tag@0+8) cases=(0@8)
/// @layout.variant name=MemoryRegionSet size=8 align=8 encoding=direct(tag@0+8) cases=(0@8)
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
function main.clamp(v0: int32): int8 {
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
type MemoryOrdering = variant<int64, void> { 0int64 = void; 1int64 = void; 2int64 = void; 3int64 = void; 4int64 = void; };

@copy
type AtomicScope = variant<int64, void> { 0int64 = void; 1int64 = void; 2int64 = void; 3int64 = void; 4int64 = void; 5int64 = void; 6int64 = void; 7int64 = void; };

@copy
type MemoryScope = variant<int64, void> { 0int64 = void; };

@copy
type MemoryRegionSet = variant<int64, void> { 0int64 = void; };

function main.acquireAll(): void {
entry:
    atomic.fence acquire
    return
}
/// @layout.variant name=MemoryOrdering size=8 align=8 encoding=direct(tag@0+8) cases=(0@8, 1@8, 2@8, 3@8, 4@8)
/// @layout.variant name=AtomicScope size=8 align=8 encoding=direct(tag@0+8) cases=(0@8, 1@8, 2@8, 3@8, 4@8, 5@8, 6@8, 7@8)
/// @layout.variant name=MemoryScope size=8 align=8 encoding=direct(tag@0+8) cases=(0@8)
/// @layout.variant name=MemoryRegionSet size=8 align=8 encoding=direct(tag@0+8) cases=(0@8)
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
function main.bump(v0: ref<int32, raw, mutable>): void {
entry(v0: ref<int32, raw, mutable>):
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
declare function readVolatile<T>(source: *T): ^T;

@intrinsic("memory.ptr.writeVolatile")
declare function writeVolatile<T>(destination: *T, value: ^T): void;

function mirror(pointer: *int32): void {
    writeVolatile<int32>(pointer, readVolatile<int32>(pointer));
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.mirror(v0: ref<int32, raw, mutable>): void {
entry(v0: ref<int32, raw, mutable>):
    v1: int32 = intrinsic.memory.ptr.readVolatile(v0)
    v2: void = intrinsic.memory.ptr.writeVolatile(v0, v1)
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
function main.exchange(v0: ref<int32, raw, mutable>, v1: int32): int32 {
entry(v0: ref<int32, raw, mutable>, v1: int32):
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
function main.flip(v0: ref<int32, raw, mutable>, v1: ref<int32, raw, mutable>): void {
entry(v0: ref<int32, raw, mutable>, v1: ref<int32, raw, mutable>):
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
function main.destroy(v0: ref<int32, raw, mutable>): void {
entry(v0: ref<int32, raw, mutable>):
    v1: int32 = load v0
    drop v1
    return
}
"#,
    );
}
