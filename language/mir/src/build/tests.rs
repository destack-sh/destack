use crate::{MirFormatOptions, Mutability, format_mir};

use super::ModuleBuilder;

/// Empty function with void return.
#[test]
fn test_build_empty_function() {
    // setup
    let mut module = ModuleBuilder::new();
    let void_type = module.type_void();

    // build empty function
    let mut builder = module.function("empty", &[], void_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @empty() -> void {
block0:
    return
}";
    assert_eq!(output, expected);
}

/// Function with two parameters that are added together.
#[test]
fn test_build_function_with_parameters() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    // build add function
    let mut builder = module.function("add", &[i32_type, i32_type], i32_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let left_value = builder.function_parameter(0);
    let right_value = builder.function_parameter(1);
    let sum_value = builder.iadd(left_value, right_value);
    builder.return_(Some(sum_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @add(v0: i32, v1: i32) -> i32 {
block0:
    v2 = iadd v0, v1
    return v2
}";
    assert_eq!(output, expected);
}

/// Function with a local variable (stack slot).
#[test]
fn test_build_function_with_locals() {
    // setup
    let mut module = ModuleBuilder::new();
    let i64_type = module.type_i64();

    // build function with local
    let mut builder = module.function("with_local", &[], i64_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);

    // create local, store, then load
    let local = builder.create_local(i64_type, Mutability::Mutable);
    let constant_value = builder.iconst_i64(42);
    builder.local_set(local, constant_value);
    let loaded_value = builder.local_get(local);
    builder.return_(Some(loaded_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @with_local() -> i64 {
    local0: i64 ; owned, mut
block0:
    v0 = iconst 42i64
    local.set local0, v0
    v1 = local.get local0
    return v1
}";
    assert_eq!(output, expected);
}

/// Function with conditional branch and multiple blocks.
#[test]
fn test_build_function_with_branch() {
    // setup
    let mut module = ModuleBuilder::new();
    let bool_type = module.type_bool();
    let i32_type = module.type_i32();

    // build function with branch
    let mut builder = module.function("select", &[bool_type], i32_type);

    // create blocks
    let entry_block = builder.create_block();
    let then_block = builder.create_block();
    let else_block = builder.create_block();
    let merge_block = builder.create_block();

    // entry block: branch on condition
    builder.switch_to_block(entry_block);
    let condition_value = builder.function_parameter(0);
    builder.branch(condition_value, then_block, else_block);
    builder.seal_block(entry_block);

    // then block: return 1
    builder.switch_to_block(then_block);
    let one_value = builder.iconst_i32(1);
    builder.jump(merge_block);
    builder.seal_block(then_block);

    // else block: return 0
    builder.switch_to_block(else_block);
    let _zero_value = builder.iconst_i32(0);
    builder.jump(merge_block);
    builder.seal_block(else_block);

    // merge block: seal all predecessors known, return one value
    // NOTE: this is simplified, see test_ssa_branch_with_phi for proper SSA
    builder.switch_to_block(merge_block);
    builder.return_(Some(one_value));
    builder.seal_block(merge_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @select(v0: bool) -> i32 {
block0:
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    jump block3
block2:
    v2 = iconst 0i32
    jump block3
block3:
    return v1
}";
    assert_eq!(output, expected);
}

/// SSA variable definition and use within a single block.
#[test]
fn test_ssa_define_use_single_block() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("var_test", &[], i32_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);

    // define variable and use it
    let variable = builder.create_variable(i32_type);
    let constant_value = builder.iconst_i32(10);
    builder.define_variable(variable, constant_value);
    let used_value = builder.use_variable(variable);

    // use_variable should return the same value
    assert_eq!(constant_value, used_value);

    builder.return_(Some(used_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @var_test() -> i32 {
block0:
    v0 = iconst 10i32
    return v0
}";
    assert_eq!(output, expected);
}

/// SSA variable redefinition in the same block should return the latest value.
#[test]
fn test_ssa_redefine_variable() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("redefine", &[], i32_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);

    // define variable twice
    let variable = builder.create_variable(i32_type);
    let first_value = builder.iconst_i32(1);
    builder.define_variable(variable, first_value);
    let second_value = builder.iconst_i32(2);
    builder.define_variable(variable, second_value);

    // should get the latest value
    let result_value = builder.use_variable(variable);
    assert_eq!(result_value, second_value);

    builder.return_(Some(result_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @redefine() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    return v1
}";
    assert_eq!(output, expected);
}

/// SSA construction across multiple blocks creates block parameters (φ-functions).
#[test]
fn test_ssa_branch_with_phi() {
    // setup
    let mut module = ModuleBuilder::new();
    let bool_type = module.type_bool();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("phi_test", &[bool_type], i32_type);

    // create blocks
    let entry_block = builder.create_block();
    let then_block = builder.create_block();
    let else_block = builder.create_block();
    let merge_block = builder.create_block();

    // create variable for the result
    let result_variable = builder.create_variable(i32_type);

    // entry block: branch on condition
    builder.switch_to_block(entry_block);
    let condition_value = builder.function_parameter(0);
    builder.branch(condition_value, then_block, else_block);
    builder.seal_block(entry_block);

    // then block: result = 1
    builder.switch_to_block(then_block);
    let one_value = builder.iconst_i32(1);
    builder.define_variable(result_variable, one_value);
    builder.jump(merge_block);
    builder.seal_block(then_block);

    // else block: result = 0
    builder.switch_to_block(else_block);
    let zero_value = builder.iconst_i32(0);
    builder.define_variable(result_variable, zero_value);
    builder.jump(merge_block);
    builder.seal_block(else_block);

    // merge block: use result (should create block parameter)
    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);
    let result_value = builder.use_variable(result_variable);
    builder.return_(Some(result_value));
    builder.finish();

    // verify output - should have block parameter in merge block
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @phi_test(v0: bool) -> i32 {
block0:
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    jump block3(v1)
block2:
    v2 = iconst 0i32
    jump block3(v2)
block3(v3: i32):
    return v3
}";
    assert_eq!(output, expected);
}

/// Trivial φ removal: if all predecessors have the same value, no block parameter is needed.
#[test]
fn test_ssa_trivial_phi_removal() {
    // setup
    let mut module = ModuleBuilder::new();
    let bool_type = module.type_bool();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("trivial_phi", &[bool_type], i32_type);

    // create blocks
    let entry_block = builder.create_block();
    let then_block = builder.create_block();
    let else_block = builder.create_block();
    let merge_block = builder.create_block();

    // create variable
    let result_variable = builder.create_variable(i32_type);

    // entry block: define variable, then branch
    builder.switch_to_block(entry_block);
    let condition_value = builder.function_parameter(0);
    let same_value = builder.iconst_i32(42);
    builder.define_variable(result_variable, same_value);
    builder.branch(condition_value, then_block, else_block);
    builder.seal_block(entry_block);

    // then block: don't redefine, just jump
    builder.switch_to_block(then_block);
    builder.jump(merge_block);
    builder.seal_block(then_block);

    // else block: don't redefine, just jump
    builder.switch_to_block(else_block);
    builder.jump(merge_block);
    builder.seal_block(else_block);

    // merge block: use variable (should NOT create block parameter - trivial phi)
    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);
    let result_value = builder.use_variable(result_variable);
    builder.return_(Some(result_value));
    builder.finish();

    // verify output - no block parameter in merge block
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @trivial_phi(v0: bool) -> i32 {
block0:
    v1 = iconst 42i32
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
    return v1
}";
    assert_eq!(output, expected);
}

/// All arithmetic operations in sequence.
#[test]
fn test_build_arithmetic_operations() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("arithmetic", &[i32_type, i32_type], i32_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);

    // chain of arithmetic operations
    let left_value = builder.function_parameter(0);
    let right_value = builder.function_parameter(1);
    let sum_value = builder.iadd(left_value, right_value);
    let difference_value = builder.isub(sum_value, right_value);
    let product_value = builder.imul(difference_value, left_value);
    let quotient_value = builder.sdiv(product_value, right_value);

    builder.return_(Some(quotient_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @arithmetic(v0: i32, v1: i32) -> i32 {
block0:
    v2 = iadd v0, v1
    v3 = isub v2, v1
    v4 = imul v3, v0
    v5 = sdiv v4, v1
    return v5
}";
    assert_eq!(output, expected);
}

/// Comparison operations.
#[test]
fn test_build_comparison_operations() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let bool_type = module.type_bool();

    // build function
    let mut builder = module.function("compare", &[i32_type, i32_type], bool_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);

    // comparison operations
    let left_value = builder.function_parameter(0);
    let right_value = builder.function_parameter(1);
    let equal_value = builder.icmp_eq(left_value, right_value);
    let less_than_value = builder.icmp_slt(left_value, right_value);
    let result_value = builder.band(equal_value, less_than_value);

    builder.return_(Some(result_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @compare(v0: i32, v1: i32) -> bool {
block0:
    v2 = icmp_eq v0, v1
    v3 = icmp_slt v0, v1
    v4 = band v2, v3
    return v4
}";
    assert_eq!(output, expected);
}

/// Type construction methods create correct types.
#[test]
fn test_type_construction() {
    use crate::Type;

    // setup
    let mut module = ModuleBuilder::new();

    // create various types
    let void_type = module.type_void();
    let bool_type = module.type_bool();
    let i32_type = module.type_i32();
    let i64_type = module.type_i64();
    let f32_type = module.type_f32();
    let f64_type = module.type_f64();
    let pointer_type = module.type_raw_pointer(i32_type);
    let array_type = module.type_array(i32_type, 10);
    let tuple_type = module.type_tuple(vec![i32_type, i64_type]);
    let function_pointer_type = module.type_function_pointer(vec![i32_type], i32_type);

    // verify types
    let (tree, _strings) = module.finish();
    assert!(matches!(tree.get(void_type), Type::Void));
    assert!(matches!(tree.get(bool_type), Type::Boolean));
    assert!(matches!(
        tree.get(i32_type),
        Type::Int {
            width: 32,
            signed: true
        }
    ));
    assert!(matches!(
        tree.get(i64_type),
        Type::Int {
            width: 64,
            signed: true
        }
    ));
    assert!(matches!(tree.get(f32_type), Type::Float { width: 32 }));
    assert!(matches!(tree.get(f64_type), Type::Float { width: 64 }));
    assert!(matches!(tree.get(pointer_type), Type::RawPointer { .. }));
    assert!(matches!(
        tree.get(array_type),
        Type::Array { length: 10, .. }
    ));
    assert!(matches!(tree.get(tuple_type), Type::Tuple { .. }));
    assert!(matches!(
        tree.get(function_pointer_type),
        Type::FunctionPointer { .. }
    ));
}

/// seal_all_blocks seals all blocks at once.
#[test]
fn test_seal_all_blocks() {
    // setup
    let mut module = ModuleBuilder::new();
    let void_type = module.type_void();

    // build multi-block function
    let mut builder = module.function("multi_block", &[], void_type);
    let block0 = builder.create_block();
    let block1 = builder.create_block();
    let block2 = builder.create_block();

    // populate blocks
    builder.switch_to_block(block0);
    builder.jump(block1);
    builder.switch_to_block(block1);
    builder.jump(block2);
    builder.switch_to_block(block2);
    builder.return_(None);

    // seal all at once instead of individually
    builder.seal_all_blocks();
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @multi_block() -> void {
block0:
    jump block1
block1:
    jump block2
block2:
    return
}";
    assert_eq!(output, expected);
}

/// ManagedReference type construction.
#[test]
fn test_managed_reference_types() {
    use crate::Type;

    // setup
    let mut module = ModuleBuilder::new();

    // create managed reference types
    let i32_type = module.type_i32();
    let ref_type = module.type_managed_reference(i32_type);
    let ref_nullable_type = module.type_managed_reference_nullable(i32_type);

    // verify types
    let (tree, _strings) = module.finish();
    assert!(matches!(
        tree.get(ref_type),
        Type::ManagedReference {
            is_nullable: false,
            ..
        }
    ));
    assert!(matches!(
        tree.get(ref_nullable_type),
        Type::ManagedReference {
            is_nullable: true,
            ..
        }
    ));
}

/// Allocation instruction: managed.alloc.
#[test]
fn test_build_managed_alloc() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let ref_type = module.type_managed_reference(i32_type);

    // build function with managed.alloc
    let mut builder = module.function("alloc_test", &[], ref_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let allocated_value = builder.managed_alloc(i32_type);
    builder.return_(Some(allocated_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @alloc_test() -> ref<i32> {
block0:
    v0 = managed.alloc i32
    return v0
}";
    assert_eq!(output, expected);
}

/// Allocation instruction: managed.alloc_array.
#[test]
fn test_build_managed_alloc_array() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let i64_type = module.type_i64();
    let array_ref_type = module.type_managed_reference(i32_type);

    // build function with managed.alloc_array
    let mut builder = module.function("alloc_array_test", &[i64_type], array_ref_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let length_value = builder.function_parameter(0);
    let allocated_value = builder.managed_alloc_array(i32_type, length_value);
    builder.return_(Some(allocated_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @alloc_array_test(v0: i64) -> ref<i32> {
block0:
    v1 = managed.alloc_array i32, v0
    return v1
}";
    assert_eq!(output, expected);
}

/// Allocation instruction: raw.alloc and raw.free.
#[test]
fn test_build_raw_alloc_and_free() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let void_type = module.type_void();
    let rawptr_type = module.type_raw_pointer(i32_type);

    // build function with raw.alloc and raw.free
    let mut builder = module.function("raw_alloc_test", &[], void_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let allocated_value = builder.raw_alloc(i32_type);
    // use the allocation
    let const_val = builder.iconst_i32(42);
    builder.store(allocated_value, const_val);
    // free the allocation
    builder.raw_free(allocated_value);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @raw_alloc_test() -> void {
block0:
    v0 = raw.alloc i32
    v1 = iconst 42i32
    store v0, v1
    raw.free v0
    return
}";
    // rawptr_type was created but not used in output
    let _ = rawptr_type;
    assert_eq!(output, expected);
}

/// Allocation instruction: stack.alloc.
#[test]
fn test_build_stack_alloc() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let rawptr_type = module.type_raw_pointer(i32_type);

    // build function with stack.alloc
    let mut builder = module.function("stack_alloc_test", &[], rawptr_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let allocated_value = builder.stack_alloc(i32_type);
    builder.return_(Some(allocated_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @stack_alloc_test() -> rawptr<i32> {
block0:
    v0 = stack.alloc i32
    return v0
}";
    assert_eq!(output, expected);
}

/// Intrinsic instructions via the builder.
#[test]
fn test_build_intrinsics() {
    use crate::Intrinsic;

    // setup
    let mut module = ModuleBuilder::new();
    let f64_type = module.type_f64();

    // build function with intrinsics
    let mut builder = module.function("intrinsic_test", &[f64_type, f64_type], f64_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);

    // chain of intrinsic operations
    let x = builder.function_parameter(0);
    let y = builder.function_parameter(1);
    let sqrt_x = builder.intrinsic(Intrinsic::Sqrt, vec![x]);
    let min_val = builder.intrinsic(Intrinsic::Min, vec![sqrt_x, y]);
    let result = builder.intrinsic(Intrinsic::Abs, vec![min_val]);
    builder.return_(Some(result));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @intrinsic_test(v0: f64, v1: f64) -> f64 {
block0:
    v2 = intrinsic.sqrt(v0)
    v3 = intrinsic.min(v2, v1)
    v4 = intrinsic.abs(v3)
    return v4
}";
    assert_eq!(output, expected);
}

/// Void intrinsic instruction via the builder.
#[test]
fn test_build_void_intrinsic() {
    use crate::Intrinsic;

    // setup
    let mut module = ModuleBuilder::new();
    let void_type = module.type_void();

    // build function with void intrinsic
    let mut builder = module.function("fence_test", &[], void_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    builder.intrinsic_void(Intrinsic::AtomicFence, vec![]);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @fence_test() -> void {
block0:
    intrinsic.atomic.fence()
    return
}";
    assert_eq!(output, expected);
}
