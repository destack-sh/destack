use crate::parse::{ParseOptions, Parser};
use crate::{MirFormatOptions, format_mir};
use destack_source::FileId;

/// Test parsing and re-formatting produces the same output.
fn roundtrip(source: &str) {
    let (tree, strings) =
        Parser::parse(FileId::new(0), source, ParseOptions::default()).expect("parse failed");
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    assert_eq!(source.trim(), output.trim(), "roundtrip mismatch");
}

#[test]
fn test_roundtrip_simple_add() {
    roundtrip(
        r#"function @add(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: i32 = iadd v0, v1
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_function_metadata() {
    roundtrip(
        r#"#[execution_model(kernel)]
#[workgroup_size(8, 1, 1)]
function @kernel() -> void {
block0:
    return
}"#,
    );
}

#[test]
fn test_roundtrip_function_workgroup_short_form() {
    roundtrip(
        r#"#[execution_model(kernel)]
#[workgroup_size(8)]
function @kernel_short() -> void {
block0:
    return
}"#,
    );
}

#[test]
fn test_roundtrip_function_stage_metadata() {
    roundtrip(
        r#"#[execution_model(graphics)]
#[execution_stage(vertex)]
function @vertex_main() -> void {
block0:
    return
}"#,
    );
}

#[test]
fn test_roundtrip_attribute_float_value() {
    roundtrip(
        r#"#[tolerance(0.25)]
function @precision() -> void {
block0:
    return
}"#,
    );
}

/// Roundtrip parsing supports attributes on items and fields.
#[test]
fn test_roundtrip_item_attributes() {
    roundtrip(
        r#"#[packed]
type @Point = { #[offset(0)] x: i32, #[offset(4)] y: i32 }
#[section(".rodata")]
global @Count: i32 = 1i32 ; readonly
function @usePoint(v0: @Point) -> i32 {
block0(v0: @Point):
    v1: i32 = field.get v0, 0
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_void_return() {
    roundtrip(
        r#"function @noop() -> void {
block0:
    return
}"#,
    );
}

#[test]
fn test_roundtrip_pointer_sized_types() {
    roundtrip(
        r#"function @pointerSized(v0: isize, v1: usize, v2: type) -> isize {
block0(v0: isize, v1: usize, v2: type):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_branch() {
    roundtrip(
        r#"function @select(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1: i32 = iconst 1i32
    jump block3(v1)
block2:
    v2: i32 = iconst 0i32
    jump block3(v2)
block3(v3: i32):
    return v3
}"#,
    );
}

/// Roundtrip parsing supports check terminators and assume instructions.
#[test]
fn test_roundtrip_check_and_assume() {
    roundtrip(
        r#"function @guard(v0: u32, v1: u32, v2: [i32; 4]) -> i32 {
block0(v0: u32, v1: u32, v2: [i32; 4]):
    v3: bool = icmp_ult v0, v1
    assume v3
    check v3, bounds.unsigned v0, v1, v2, block1(v0), block2
block1(v4: u32):
    v5: i32 = iconst 0i32
    return v5
block2:
    unreachable
}"#,
    );
}

/// Roundtrip parsing supports type guard checks.
#[test]
fn test_roundtrip_check_type_guards() {
    roundtrip(
        r#"function @guard(v0: u32, v1: ref<managed void>) -> i32 {
block0(v0: u32, v1: ref<managed void>):
    v2: bool = icmp_eq v0, v0
    check v2, type v0, i32, block1, block4
block1:
    v3: bool = icmp_eq v0, v0
    check v3, union v0, 1, block4, block5
block2:
    v4: bool = icmp_eq v0, v0
    check v4, vtable v1, i32, block2, block4
block3:
    v5: bool = icmp_eq v0, v0
    check v5, itab v1, 0, block3, block5
block4:
    v6: i32 = iconst 0i32
    return v6
block5:
    unreachable
}"#,
    );
}

#[test]
fn test_roundtrip_call() {
    roundtrip(
        r#"extern function @callee(i32, i32) -> i32
function @caller() -> i32 {
block0:
    v0: i32 = iconst 1i32
    v1: i32 = iconst 2i32
    v2: i32 = call @callee(v0, v1) -> fn(i32, i32) -> i32
    return v2
}"#,
    );
}

// TODO #Cleanup: streamline MIR text syntax
//  (array from [T; N] to T[N], call.indirect should be vx(..) : with colon, see related -> usage)

#[test]
fn test_roundtrip_function_addr() {
    roundtrip(
        r#"extern function @callee(i32) -> i32
function @caller() -> i32 {
block0:
    v0: fn(i32) -> i32 = function.addr @callee
    v1: i32 = iconst 1i32
    v2: i32 = call.indirect v0(v1) -> fn(i32) -> i32
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_function_env() {
    roundtrip(
        r#"extern function @callee(i32) -> i32
#[closure_env(ref<managed void>)]
function @caller() -> i32 {
block0:
    v0: ref<managed void> = function.env
    v1: fn(i32) -> i32 = function.addr @callee
    v2: i32 = iconst 1i32
    v3: i32 = call.indirect v1(v2, env=v0) -> fn(i32) -> i32
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_null_constant() {
    roundtrip(
        r#"function @caller() -> ref?<managed void> {
block0:
    v0: ref?<managed void> = iconst null
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_switch() {
    roundtrip(
        r#"function @dispatch(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block3, 0 => block1, 1 => block2
block1:
    v1: i32 = iconst 100i32
    return v1
block2:
    v2: i32 = iconst 200i32
    return v2
block3:
    v3: i32 = iconst 0i32
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_vector_tensor_ops() {
    roundtrip(
        r#"function @vector_tensor_ops(v0: vector<i32, 4>, v1: i32, v2: tensor<i32, [2, 2]>, v3: tensor_ref<borrowed i32, [2, 2]>) -> tensor<i32, [2, 2]> {
block0(v0: vector<i32, 4>, v1: i32, v2: tensor<i32, [2, 2]>, v3: tensor_ref<borrowed i32, [2, 2]>):
    v4: vector<i32, 4> = vector.splat v1
    v5: i32 = vector.extract v4, v1
    v6: vector<i32, 4> = vector.insert v4, v1, v1
    v7: vector<i32, 4> = vector.shuffle v4, v6, [0, 1, 2, 3]
    v8: i32 = vector.reduce add, v7
    v9: vector<bool, 4> = vector.compare icmp_eq, v4, v6
    v10: vector<i32, 4> = vector.convert exact, v4
    v11: i32 = iconst 0i32
    v12: i32 = iconst 1i32
    v13: i32 = tensor.load v3, [v11, v12]
    tensor.store v3, [v12, v11], v13
    tensor.fill v3, v11
    tensor.copy v3, v3
    v14: tensor<i32, [2, 2]> = tensor.reshape v2, [v11, v12]
    v15: tensor<i32, [2, 2]> = tensor.broadcast v2, [0, 1]
    v16: tensor<i32, [2, 2]> = tensor.transpose v2, [1, 0]
    v17: tensor<i32, [2, 2]> = tensor.cast v2
    v18: tensor_ref<borrowed i32, [2, 2]> = tensor.view v3, offsets=[v11, v11], sizes=[v12, v12], strides=[v12, v12]
    v19: tensor<i32, [2, 2]> = tensor.slice v2, offsets=[v11, v11], sizes=[v12, v12], strides=[v12, v12]
    v20: tensor<i32, [2, 2]> = tensor.pad v2, value=v11, low=[v11, v11], high=[v11, v11], interior=[v11, v11]
    v21: tensor<i32, [2, 2]> = tensor.concat [v2, v2], axis=0
    v22: tensor<bool, [2, 2]> = tensor.compare icmp_eq, v2, v2
    v23: tensor<i32, [2, 2]> = tensor.reduce add, v2, v11, axes=[0]
    v24: tensor<i32, [2, 2]> = tensor.dot v2, v2, dims(lhs_batch=[], rhs_batch=[], lhs_contract=[1], rhs_contract=[0])
    v25: tensor<i32, [2, 2]> = tensor.convolution v2, v2, dims(input_batch=0, input_feature=1, input_spatial=[2, 3], kernel_input_feature=0, kernel_output_feature=1, kernel_spatial=[2, 3], output_batch=0, output_feature=1, output_spatial=[2, 3]), strides=[1, 1], padding_low=[0, 0], padding_high=[0, 0], lhs_dilation=[1, 1], rhs_dilation=[1, 1], window_reversal=[false, false], feature_group=1, batch_group=1
    v26: tensor<i32, [2, 2]> = tensor.gather v2, v2, dims(offset_dims=[0], collapsed_slice_dims=[1], start_index_map=[0], index_vector_dim=1), slice_sizes=[1, 1]
    v27: tensor<i32, [2, 2]> = tensor.scatter v2, v2, v2, dims(update_window_dims=[0], inserted_window_dims=[1], scatter_dims_to_operand_dims=[0], index_vector_dim=1), mode=replace
    v28: tensor<f32, [2, 2]> = tensor.convert exact, v2
    return v14
}"#,
    );
}

/// Roundtrip parsing supports yield terminators.
#[test]
fn test_roundtrip_yield() {
    roundtrip(
        r#"function @yield_once(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 5i32
    yield v1, block1(v0)
block1(v2: i32, v3: i32):
    v4: i32 = iadd v2, v3
    return v4
}"#,
    );
}

#[test]
fn test_roundtrip_managed_alloc() {
    roundtrip(
        r#"function @allocTest() -> ref<managed i32> {
block0:
    v0: ref<managed i32> = managed.alloc i32
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_managed_alloc_array() {
    roundtrip(
        r#"function @arrayAlloc(v0: i64) -> ref<managed i32> {
block0(v0: i64):
    v1: ref<managed i32> = managed.alloc_array i32, v0
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_raw_alloc_and_free() {
    roundtrip(
        r#"function @rawAlloc() -> void {
block0:
    v0: ref<raw i32> = raw.alloc i32
    raw.free v0
    return
}"#,
    );
}

#[test]
fn test_roundtrip_stack_alloc() {
    roundtrip(
        r#"function @stackAlloc() -> ref<raw addrspace(stack) i32> {
block0:
    v0: ref<raw addrspace(stack) i32> = stack.alloc i32
    return v0
}"#,
    );
}

/// Roundtrip parsing supports address space references.
#[test]
fn test_roundtrip_addrspace_reference() {
    roundtrip(
        r#"function @addrspaceTest(v0: ref<raw addrspace(shared) i32>, v1: ref<raw addrspace(7) i32>) -> void {
block0(v0: ref<raw addrspace(shared) i32>, v1: ref<raw addrspace(7) i32>):
    return
}"#,
    );
}

#[test]
fn test_roundtrip_nullable_ref() {
    roundtrip(
        r#"function @nullableTest() -> ref?<managed i32> {
block0:
    v0: ref<managed i32> = managed.alloc i32
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_string_constant() {
    roundtrip(
        r#"global @literal:string:hello_world: ref<managed void> = "hello world" ; readonly
function @stringTest() -> void {
block0:
    v0: ref<managed void> = global.const @literal:string:hello_world
    return
}"#,
    );
}

#[test]
fn test_roundtrip_string_with_escapes() {
    roundtrip(
        r#"global @literal:string:hello_world_nl: ref<managed void> = "hello\nworld" ; readonly
function @escapeTest() -> void {
block0:
    v0: ref<managed void> = global.const @literal:string:hello_world_nl
    return
}"#,
    );
}

#[test]
fn test_roundtrip_char_constant() {
    roundtrip(
        r#"function @charTest() -> void {
block0:
    v0: u32 = iconst 'a'
    return
}"#,
    );
}

#[test]
fn test_roundtrip_char_escape() {
    roundtrip(
        r#"function @charEscapeTest() -> void {
block0:
    v0: u32 = iconst '\n'
    return
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_unary() {
    // unary intrinsic with one argument
    roundtrip(
        r#"function @sqrtTest(v0: f64) -> f64 {
block0(v0: f64):
    v1: f64 = intrinsic.sqrt(v0)
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_binary() {
    // binary intrinsic with two arguments
    roundtrip(
        r#"function @minTest(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2: f64 = intrinsic.min(v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_ternary() {
    // ternary intrinsic (fused multiply-add)
    roundtrip(
        r#"function @fmaTest(v0: f64, v1: f64, v2: f64) -> f64 {
block0(v0: f64, v1: f64, v2: f64):
    v3: f64 = intrinsic.fma(v0, v1, v2)
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_void() {
    // void intrinsic (no return value)
    roundtrip(
        r#"function @fenceTest() -> void {
block0:
    intrinsic.atomic.fence(ordering=seq_cst, scope=device, memory_scope=device, semantics=any)
    return
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_bit_manipulation() {
    // bit manipulation intrinsics
    roundtrip(
        r#"function @bitTest(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = intrinsic.clz(v0)
    v2: i32 = intrinsic.ctz(v1)
    v3: i32 = intrinsic.popcnt(v2)
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_overflow() {
    // checked arithmetic intrinsics
    roundtrip(
        r#"function @addOverflowTest(v0: i32, v1: i32) -> (i32, bool) {
block0(v0: i32, v1: i32):
    v2: (i32, bool) = intrinsic.add.overflow(v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_atomic() {
    // atomic intrinsic with memory ordering
    roundtrip(
        r#"function @atomicTest(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1: i32 = intrinsic.atomic.load(v0, ordering=acquire, scope=device, memory_scope=device, semantics=[global, make_visible])
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_intrinsic_atomic_fence() {
    // atomic fence with ordering
    roundtrip(
        r#"function @fenceTest() -> void {
block0:
    intrinsic.atomic.fence(ordering=seq_cst, scope=device, memory_scope=device, semantics=any)
    return
}"#,
    );
}

#[test]
fn test_roundtrip_field_operations() {
    // field.get and field.set
    roundtrip(
        r#"function @fieldTest(v0: (i32, f64)) -> i32 {
block0(v0: (i32, f64)):
    v1: i32 = field.get v0, 0
    v2: i32 = iconst 42i32
    v3: (i32, f64) = field.set v0, 0, v2
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_element_operations() {
    // element.get and element.set
    roundtrip(
        r#"function @elementTest(v0: [i32; 10], v1: i64) -> i32 {
block0(v0: [i32; 10], v1: i64):
    v2: i32 = element.get v0, v1
    v3: i32 = iconst 42i32
    v4: [i32; 10] = element.set v0, v1, v3
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_load_store() {
    // load and store through pointer
    roundtrip(
        r#"function @loadStoreTest(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1: i32 = load v0
    v2: i32 = iconst 42i32
    store v0, v2
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_unary_operations() {
    // unary operations
    roundtrip(
        r#"function @unaryTest(v0: i32, v1: f64) -> i32 {
block0(v0: i32, v1: f64):
    v2: i32 = ineg v0
    v3: i32 = bnot v0
    v4: f64 = fneg v1
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_cast_operations() {
    // cast operations
    roundtrip(
        r#"function @castTest(v0: i32) -> i64 {
block0(v0: i32):
    v1: i64 = sextend v0 -> i64
    return v1
}"#,
    );
}

#[test]
fn test_roundtrip_tuple_type() {
    // tuple type in function signature
    roundtrip(
        r#"function @tupleTest(v0: (i32, f64, bool)) -> (i32, f64, bool) {
block0(v0: (i32, f64, bool)):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_array_type() {
    // array type in function signature
    roundtrip(
        r#"function @arrayTest(v0: [i32; 10]) -> [i32; 10] {
block0(v0: [i32; 10]):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_function_pointer_type() {
    // function pointer type
    roundtrip(
        r#"function @fnptrTest(v0: fn(i32, i32) -> i64) -> fn(i32, i32) -> i64 {
block0(v0: fn(i32, i32) -> i64):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_struct_type() {
    // struct type in function signature
    roundtrip(
        r#"function @structTest(v0: { i32, f64 }) -> { i32, f64 } {
block0(v0: { i32, f64 }):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_struct_type_named_fields() {
    // struct type with field names in function signature
    roundtrip(
        r#"function @structNamedTest(v0: { x: i32, y: f64 }) -> { x: i32, y: f64 } {
block0(v0: { x: i32, y: f64 }):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_type_alias() {
    roundtrip(
        r#"type @Point = { i32, i32 }
function @usePoint(v0: ref<managed @Point>) -> ref<managed @Point> {
block0(v0: ref<managed @Point>):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_recursive_type_alias() {
    roundtrip(
        r#"type @Node = { value: i64, next: ref<managed @Node> }
function @useNode(v0: ref<managed @Node>) -> ref<managed @Node> {
block0(v0: ref<managed @Node>):
    return v0
}"#,
    );
}

#[test]
fn test_roundtrip_call_indirect() {
    // call through function pointer
    roundtrip(
        r#"function @indirectCallTest(v0: fn(i32) -> i32, v1: i32) -> i32 {
block0(v0: fn(i32) -> i32, v1: i32):
    v2: i32 = call.indirect v0(v1) -> fn(i32) -> i32
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_struct() {
    // construct a struct from field values
    roundtrip(
        r#"function @makePoint(v0: i32, v1: i32) -> { i32, i32 } {
block0(v0: i32, v1: i32):
    v2: { i32, i32 } = struct { i32, i32 } (v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_tuple() {
    // construct a tuple from element values
    roundtrip(
        r#"function @makePair(v0: i32, v1: bool) -> (i32, bool) {
block0(v0: i32, v1: bool):
    v2: (i32, bool) = tuple (i32, bool) (v0, v1)
    return v2
}"#,
    );
}

#[test]
fn test_roundtrip_array() {
    // construct an array from element values
    roundtrip(
        r#"function @makeArray() -> [i32; 3] {
block0:
    v0: i32 = iconst 1i32
    v1: i32 = iconst 2i32
    v2: i32 = iconst 3i32
    v3: [i32; 3] = array [i32; 3] (v0, v1, v2)
    return v3
}"#,
    );
}

#[test]
fn test_roundtrip_struct_type_alias() {
    // construct a struct using a type alias
    roundtrip(
        r#"type @Point = { i32, i32 }
function @makePoint(v0: i32, v1: i32) -> @Point {
block0(v0: i32, v1: i32):
    v2: @Point = struct @Point (v0, v1)
    return v2
}"#,
    );
}
