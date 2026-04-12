use super::tests::roundtrip;

#[test]
fn test_roundtrip_simple_add() {
    roundtrip(
        r#"
function add(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_function_metadata() {
    roundtrip(
        r#"
@executionModel(kernel)
@workgroupSize(8, 1, 1)
function kernel(): void {
b0:
    return
}"#,
    );
}

#[test]
fn test_roundtrip_dotted_symbol_names() {
    roundtrip(
        r#"
type Status = newtype<int32>;

global Status.Default: Status, readonly = 1int32

function Status.isActive(v0: Status): boolean {
b0(v0: Status):
    v1: int32 = cast.bit v0 -> int32
    v2: int32 = 1int32
    v3: boolean = int.eq v1, v2
    return v3
}

function checkDefault(): boolean {
b0:
    v0: Status = global.const Status.Default
    v1: boolean = call Status.isActive(v0): (Status) -> boolean
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_function_value_type() {
    roundtrip(
        r#"
type Callable = closure(int32) -> int32;

function use(v0: Callable): Callable {
b0(v0: Callable):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_function_workgroup_short_form() {
    roundtrip(
        r#"
@executionModel(kernel)
@workgroupSize(8)
function kernelShort(): void {
b0:
    return
}"#,
    );
}

#[test]
fn test_roundtrip_function_stage_metadata() {
    roundtrip(
        r#"
@executionModel(graphics)
@executionStage(vertex)
function vertexMain(): void {
b0:
    return
}"#,
    );
}

#[test]
fn test_roundtrip_attribute_float_value() {
    roundtrip(
        r#"
@tolerance(0.25)
function precision(): void {
b0:
    return
}"#,
    );
}

/// Roundtrip parsing supports attributes on items and fields.
#[test]
fn test_roundtrip_item_attributes() {
    roundtrip(
        r#"
@packed
type Point {
    @offset(0)
    x: int32;
    @offset(4)
    y: int32;
}

@section(".rodata")
global Count: int32, readonly = 1int32

function usePoint(v0: Point): int32 {
b0(v0: Point):
    v1: int32 = field.get v0, 0
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_void_return() {
    roundtrip(
        r#"
function noop(): void {
b0:
    return
}"#,
    );
}

#[test]
fn test_roundtrip_pointer_sized_types() {
    roundtrip(
        r#"
function pointerSized(v0: isize, v1: usize, v2: typeDescriptor, v3: typeId): isize {
b0(v0: isize, v1: usize, v2: typeDescriptor, v3: typeId):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_branch() {
    roundtrip(
        r#"
function select(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2

b1:
    v1: int32 = 1int32
    jump b3(v1)

b2:
    v2: int32 = 0int32
    jump b3(v2)

b3(v3: int32):
    return v3
}"#,
    );
}

/// Roundtrip parsing supports check terminators and assume instructions.
#[test]
fn test_roundtrip_check_and_assume() {
    roundtrip(
        r#"
function guard(v0: uint32, v1: uint32, v2: int32[4]): int32 {
b0(v0: uint32, v1: uint32, v2: int32[4]):
    v3: boolean = int.lt.u v0, v1
    assume v3
    check bounds.u v0, v1, v2 -> b1(v0), b2

b1(v4: uint32):
    v5: int32 = 0int32
    return v5

b2:
    unreachable
}"#,
    );
}

/// Roundtrip parsing supports type guard checks.
#[test]
fn test_roundtrip_check_type_guards() {
    roundtrip(
        r#"
function guard(v0: uint32, v1: ref<void, managed>): int32 {
b0(v0: uint32, v1: ref<void, managed>):
    v2: boolean = int.eq v0, v0
    check dynamicType v0, int32 -> b1, b4

b1:
    v3: boolean = int.eq v0, v0
    check unionTag v0, 1 -> b4, b5

b2:
    v4: boolean = int.eq v0, v0
    check receiverType v1, int32 -> b2, b4

b3:
    v5: boolean = int.eq v0, v0
    check interfaceConformance v1, int32 -> b3, b5

b4:
    v6: int32 = 0int32
    return v6

b5:
    unreachable
}"#,
    );
}

#[test]
fn test_roundtrip_call() {
    roundtrip(
        r#"
extern function callee(int32, int32): int32

function caller(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_exceptional_call_terminator() {
    roundtrip(
        r#"
extern function callee(int32): int32

function caller(v0: int32): int32 {
b0(v0: int32):
    invoke callee(v0): (int32) -> int32 -> b1, catch b2

b1(v1: int32):
    return v1

b2(v2: ref<int32, managed, readonly>):
    throw v2
}"#,
    );
}

#[test]
fn test_roundtrip_trap_terminator() {
    roundtrip(
        r#"
global message: ref<void, managed, readonly>, readonly = "boom"

function trapper(): void {
b0:
    v0: ref<void, managed, readonly> = global.const message
    trap.panic v0
}"#,
    );
}

#[test]
fn test_roundtrip_function_addr() {
    roundtrip(
        r#"
extern function callee(int32): int32

function caller(): int32 {
b0:
    v0: fn(int32) -> int32 = function.address callee
    v1: int32 = 1int32
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_function_environment() {
    roundtrip(
        r#"
@environment(ref<void, managed>)
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: ref<void, managed> = function.environment
    return v0
}

@environment(ref<void, managed>)
function caller(): int32 {
b0:
    v0: ref<void, managed> = function.environment
    v1: closure(int32) -> int32 = function.bind callee, v0
    v2: int32 = 1int32
    v3: int32 = call.indirect v1(v2): (int32) -> int32
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_null_constant() {
    roundtrip(
        r#"
function caller(): ref?<void, managed> {
b0:
    v0: ref?<void, managed> = null
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_switch() {
    roundtrip(
        r#"
function dispatch(v0: int32): int32 {
b0(v0: int32):
    switch v0, b3, 0 => b1, 1 => b2

b1:
    v1: int32 = 100int32
    return v1

b2:
    v2: int32 = 200int32
    return v2

b3:
    v3: int32 = 0int32
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_vector_tensor_ops() {
    roundtrip(
        r#"
function vectorTensorOps(v0: vector<int32, 4>, v1: int32, v2: tensor<int32, (2, 2)>, v3: tensorRef<int32, borrowed, (2, 2)>): tensor<int32, (2, 2)> {
b0(v0: vector<int32, 4>, v1: int32, v2: tensor<int32, (2, 2)>, v3: tensorRef<int32, borrowed, (2, 2)>):
    v4: vector<int32, 4> = vector.splat v1
    v5: int32 = vector.extract v4, v1
    v6: vector<int32, 4> = vector.insert v4, v1, v1
    v7: vector<int32, 4> = vector.shuffle v4, v6, [0, 1, 2, 3]
    v8: int32 = vector.reduce add, v7
    v9: vector<boolean, 4> = vector.compare int.eq, v4, v6
    v10: vector<int32, 4> = vector.convert exact, v4
    v11: int32 = 0int32
    v12: int32 = 1int32
    v13: int32 = tensor.load v3, [v11, v12]
    tensor.store v3, [v12, v11], v13
    tensor.fill v3, v11
    tensor.copy v3, v3
    v14: tensor<int32, (2, 2)> = tensor.reshape v2, shape(v11, v12)
    v15: tensor<int32, (2, 2)> = tensor.broadcast v2, dimensions(0, 1)
    v16: tensor<int32, (2, 2)> = tensor.transpose v2, permutation(1, 0)
    v17: tensor<int32, (2, 2)> = tensor.cast v2
    v18: tensorRef<int32, borrowed, (2, 2)> = tensor.view v3, offsets(v11, v11), sizes(v12, v12), strides(v12, v12)
    v19: tensor<int32, (2, 2)> = tensor.slice v2, offsets(v11, v11), sizes(v12, v12), strides(v12, v12)
    v20: tensor<int32, (2, 2)> = tensor.pad v2, value(v11), low(v11, v11), high(v11, v11), interior(v11, v11)
    v21: tensor<int32, (2, 2)> = tensor.concat tensors(v2, v2), axis(0)
    v22: tensor<boolean, (2, 2)> = tensor.compare int.eq, v2, v2
    v23: tensor<int32, (2, 2)> = tensor.reduce add, v2, v11, axes(0)
    v24: tensor<int32, (2, 2)> = tensor.dot v2, v2, dims(lhsBatch(), rhsBatch(), lhsContract(1), rhsContract(0))
    v25: tensor<int32, (2, 2)> = tensor.convolution v2, v2, dims(inputBatch(0), inputFeature(1), inputSpatial(2, 3), kernelInputFeature(0), kernelOutputFeature(1), kernelSpatial(2, 3), outputBatch(0), outputFeature(1), outputSpatial(2, 3)), window(strides(1, 1), paddingLow(0, 0), paddingHigh(0, 0), lhsDilation(1, 1), rhsDilation(1, 1), windowReversal(false, false)), groups(feature(1), batch(1))
    v26: tensor<int32, (2, 2)> = tensor.gather v2, v2, dims(offsetDims(0), collapsedSliceDims(1), startIndexMap(0), indexVectorDim(1)), sliceSizes(1, 1)
    v27: tensor<int32, (2, 2)> = tensor.scatter v2, v2, v2, dims(updateWindowDims(0), insertedWindowDims(1), scatterDimsToOperandDims(0), indexVectorDim(1)), mode(replace)
    v28: tensor<float32, (2, 2)> = tensor.convert exact, v2
    return v14
}"#,
    );
}

/// Roundtrip parsing supports yield terminators.
#[test]
fn test_roundtrip_yield() {
    roundtrip(
        r#"
function yieldOnce(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 5int32
    yield v1, b1(v0)

b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}"#,
    );
}

#[test]
fn test_roundtrip_managed_alloc() {
    roundtrip(
        r#"
function allocTest(): ref<int32, managed> {
b0:
    v0: ref<int32, managed> = managed.alloc int32
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_managed_alloc_array() {
    roundtrip(
        r#"
function arrayAlloc(v0: int64): ref<int32, managed> {
b0(v0: int64):
    v1: ref<int32, managed> = managed.allocArray int32, v0
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_raw_alloc_and_free() {
    roundtrip(
        r#"
function rawAlloc(): void {
b0:
    v0: ref<int32, raw> = raw.alloc int32
    raw.free v0
    return
}"#,
    );
}

#[test]
fn test_roundtrip_stack_alloc() {
    roundtrip(
        r#"
function stackAlloc(): ref<int32, raw, addressSpace(stack)> {
b0:
    v0: ref<int32, raw, addressSpace(stack)> = stack.alloc int32
    return v0
}"#,
    );
}

/// Roundtrip parsing supports address space references.
#[test]
fn test_roundtrip_address_space_reference() {
    roundtrip(
        r#"
function addressSpaceTest(v0: ref<int32, raw, addressSpace(shared)>, v1: ref<int32, raw, addressSpace(7)>): void {
b0(v0: ref<int32, raw, addressSpace(shared)>, v1: ref<int32, raw, addressSpace(7)>):
    return
}"#,
    );
}

#[test]
fn test_roundtrip_nullable_ref() {
    roundtrip(
        r#"
function nullableTest(): ref?<int32, managed> {
b0:
    v0: ref<int32, managed> = managed.alloc int32
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_reference_mutability_matrix() {
    roundtrip(
        r#"
function refKinds(v0: ref<int32, managed>, v1: ref<int32, managed, readonly>, v2: ref<int32, owned>, v3: ref<int32, owned, readonly>, v4: ref<int32, raw>, v5: ref<int32, raw, readonly>): ref<int32, managed> {
b0(v0: ref<int32, managed>, v1: ref<int32, managed, readonly>, v2: ref<int32, owned>, v3: ref<int32, owned, readonly>, v4: ref<int32, raw>, v5: ref<int32, raw, readonly>):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_string_constant() {
    roundtrip(
        r#"
global stringLiteralHelloWorld: ref<void, managed>, readonly = "hello world"

function stringTest(): void {
b0:
    v0: ref<void, managed> = global.const stringLiteralHelloWorld
    return
}"#,
    );
}

#[test]
fn test_roundtrip_string_with_escapes() {
    roundtrip(
        r#"
global stringLiteralHelloWorldNl: ref<void, managed>, readonly = "hello\nworld"

function escapeTest(): void {
b0:
    v0: ref<void, managed> = global.const stringLiteralHelloWorldNl
    return
}"#,
    );
}

#[test]
fn test_roundtrip_char_constant() {
    roundtrip(
        r#"
function charTest(): void {
b0:
    v0: uint32 = 'a'
    return
}"#,
    );
}

#[test]
fn test_roundtrip_char_escape() {
    roundtrip(
        r#"
function charEscapeTest(): void {
b0:
    v0: uint32 = '\n'
    return
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_unary() {
    // unary intrinsic with one argument
    roundtrip(
        r#"
function sqrtTest(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.sqrt(v0)
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_binary() {
    // binary intrinsic with two arguments
    roundtrip(
        r#"
function minTest(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = intrinsic.min(v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_ternary() {
    // ternary intrinsic (fused multiply-add)
    roundtrip(
        r#"
function fmaTest(v0: float64, v1: float64, v2: float64): float64 {
b0(v0: float64, v1: float64, v2: float64):
    v3: float64 = intrinsic.fma(v0, v1, v2)
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_void() {
    // void intrinsic (no return value)
    roundtrip(
        r#"
function fenceTest(): void {
b0:
    atomic.fence sequentiallyConsistent, device, device, any
    return
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_bit_manipulation() {
    // bit manipulation intrinsics
    roundtrip(
        r#"
function bitTest(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = intrinsic.leadingZeroCount(v0)
    v2: int32 = intrinsic.trailingZeroCount(v1)
    v3: int32 = intrinsic.populationCount(v2)
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_overflow() {
    // checked arithmetic intrinsics
    roundtrip(
        r#"
function addOverflowTest(v0: int32, v1: int32): (int32, boolean) {
b0(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.add.overflow(v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_atomic() {
    // atomic intrinsic with memory ordering
    roundtrip(
        r#"
function atomicTest(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = atomic.load v0, acquire, device, device, [global, makeVisible]
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_atomic_fence() {
    // atomic fence with ordering
    roundtrip(
        r#"
function fenceTest(): void {
b0:
    atomic.fence sequentiallyConsistent, device, device, any
    return
}"#,
    );
}

#[test]
fn test_roundtrip_field_operations() {
    // field.get and field.set
    roundtrip(
        r#"
function fieldTest(v0: (int32, float64)): int32 {
b0(v0: (int32, float64)):
    v1: int32 = field.get v0, 0
    v2: int32 = 42int32
    v3: (int32, float64) = field.set v0, 0, v2
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_element_operations() {
    // element.get and element.set
    roundtrip(
        r#"
function elementTest(v0: int32[10], v1: int64): int32 {
b0(v0: int32[10], v1: int64):
    v2: int32 = element.get v0, v1
    v3: int32 = 42int32
    v4: int32[10] = element.set v0, v1, v3
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_load_store() {
    // load and store through pointer
    roundtrip(
        r#"
function loadStoreTest(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    v2: int32 = 42int32
    store v0, v2
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_unary_operations() {
    // unary operations
    roundtrip(
        r#"
function unaryTest(v0: int32, v1: float64): int32 {
b0(v0: int32, v1: float64):
    v2: int32 = int.negate v0
    v3: int32 = int.not v0
    v4: float64 = float.negate v1
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_cast_operations() {
    // cast operations
    roundtrip(
        r#"
function castTest(v0: int32): int64 {
b0(v0: int32):
    v1: int64 = cast.extend.s v0 -> int64
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_tuple_type() {
    // tuple type in function signature
    roundtrip(
        r#"
function tupleTest(v0: (int32, float64, boolean)): (int32, float64, boolean) {
b0(v0: (int32, float64, boolean)):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_array_type() {
    // array type in function signature
    roundtrip(
        r#"
function arrayTest(v0: int32[10]): int32[10] {
b0(v0: int32[10]):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_function_pointer_type() {
    // function pointer type
    roundtrip(
        r#"
function fnptrTest(v0: fn(int32, int32) -> int64): fn(int32, int32) -> int64 {
b0(v0: fn(int32, int32) -> int64):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_struct_type() {
    // struct type in function signature
    roundtrip(
        r#"
function structTest(v0: { int32, float64 }): { int32, float64 } {
b0(v0: { int32, float64 }):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_struct_type_named_fields() {
    // struct type with field names in function signature
    roundtrip(
        r#"
function structNamedTest(v0: { x: int32, y: float64 }): { x: int32, y: float64 } {
b0(v0: { x: int32, y: float64 }):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_type_alias() {
    roundtrip(
        r#"
type Point {
    int32;
    int32;
}

function usePoint(v0: ref<Point, managed>): ref<Point, managed> {
b0(v0: ref<Point, managed>):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_recursive_type_alias() {
    roundtrip(
        r#"
type Node {
    value: int64;
    next: ref<Node, managed>;
}

function useNode(v0: ref<Node, managed>): ref<Node, managed> {
b0(v0: ref<Node, managed>):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_call_indirect() {
    // call through function pointer
    roundtrip(
        r#"
function indirectCallTest(v0: fn(int32) -> int32, v1: int32): int32 {
b0(v0: fn(int32) -> int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_struct() {
    // construct a struct from field values
    roundtrip(
        r#"
function makePoint(v0: int32, v1: int32): { int32, int32 } {
b0(v0: int32, v1: int32):
    v2: { int32, int32 } = struct { int32, int32 } (v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_tuple() {
    // construct a tuple from element values
    roundtrip(
        r#"
function makePair(v0: int32, v1: boolean): (int32, boolean) {
b0(v0: int32, v1: boolean):
    v2: (int32, boolean) = tuple (int32, boolean) (v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_array() {
    // construct an array from element values
    roundtrip(
        r#"
function makeArray(): int32[3] {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32[3] = array int32[3] (v0, v1, v2)
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_struct_type_alias() {
    // construct a struct using a type alias
    roundtrip(
        r#"
type Point {
    int32;
    int32;
}

function makePoint(v0: int32, v1: int32): Point {
b0(v0: int32, v1: int32):
    v2: Point = struct Point (v0, v1)
    return v2
}"#,
    );
}
