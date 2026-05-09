use crate::build::ModuleBuilder;
use crate::{
    Access, AddressSpace, Copy, FenceAccess, Lifetime, MemoryFlags, MemoryOrdering, MemoryScope,
    MirFormatOptions, Mutability, ReferenceKind, SyncScope, Type, format_mir,
};

/// Empty function with void return.
#[test]
fn test_build_empty_function() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let void_type = module.type_void();

    // build empty function
    let mut builder = module.function("empty", &[], void_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function empty(): void {
entry0:
    return
}";
    assert_eq!(output, expected);
}

/// Function with two parameters that are added together.
#[test]
fn test_build_function_with_parameters() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();

    // build add function
    let mut builder = module.function("add", &[i32_type, i32_type], i32_type);
    let entry_block = builder.block();
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
function add(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = int.add value0, value1
    return value2
}";
    assert_eq!(output, expected);
}

/// Function with a local variable (stack slot).
#[test]
fn test_build_function_with_locals() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i64_type = module.type_i64();

    // build function with local
    let mut builder = module.function("withLocal", &[], i64_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    // create local, store, then load
    let local = builder.local(i64_type, Mutability::Mutable);
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
function withLocal(): int64 {
    local local0: int64, owned

entry0:
    value0: int64 = 42int64
    local.set local0, value0
    value1: int64 = local.get local0
    return value1
}";
    assert_eq!(output, expected);
}

/// Function with conditional branch and multiple blocks.
#[test]
fn test_build_function_with_branch() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let bool_type = module.type_boolean();
    let i32_type = module.type_i32();

    // build function with branch
    let mut builder = module.function("select", &[bool_type], i32_type);

    // create blocks
    let entry_block = builder.block();
    let then_block = builder.block();
    let else_block = builder.block();
    let merge_block = builder.block();

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
function select(value0: boolean): int32 {
entry0(value0: boolean):
    branch value0, block1, block2

block1:
    value1: int32 = 1int32
    jump block3

block2:
    value2: int32 = 0int32
    jump block3

block3:
    return value1
}";
    assert_eq!(output, expected);
}

/// Exceptional call terminators carry explicit success and exception continuations.
#[test]
fn test_build_function_with_exceptional_call_terminator() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let exception_type = module.type_managed_reference(i32_type);
    let signature = module.type_function_signature(vec![i32_type], i32_type);
    let callee = module.extern_function("callee", &[i32_type], i32_type);

    // build function
    let mut builder = module.function("caller", &[i32_type], i32_type);
    let entry_block = builder.block();
    let normal_block = builder.block();
    let unwind_block = builder.block();

    // entry: branch through the exceptional call
    builder.switch_to_block(entry_block);
    let argument = builder.function_parameter(0);
    let result = builder.add_block_parameter(normal_block, i32_type);
    let exception = builder.add_block_parameter(unwind_block, exception_type);
    builder.call_branch(
        callee,
        signature,
        vec![argument],
        normal_block,
        Vec::new(),
        unwind_block,
        Vec::new(),
    );
    builder.seal_block(entry_block);

    // success continuation
    builder.switch_to_block(normal_block);
    builder.return_(Some(result));
    builder.seal_block(normal_block);

    // exception continuation
    builder.switch_to_block(unwind_block);
    builder.throw(exception);
    builder.seal_block(unwind_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
extern function callee(int32): int32

function caller(value0: int32): int32 {
entry0(value0: int32):
    invoke callee(value0): (int32) -> int32 -> block1, catch block2

block1(value1: int32):
    return value1

block2(value2: ref<int32, managed, readonly>):
    throw value2
}";
    assert_eq!(output, expected);
}

/// Trap terminators format as explicit control exits.
#[test]
fn test_build_function_with_trap_terminator() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let string_type = module.type_managed_reference(i32_type);
    let void_type = module.type_void();

    // build function
    let mut builder = module.function("trapper", &[], void_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let payload = builder.null(string_type);
    builder.trap_panic(payload);
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function trapper(): void {
entry0:
    value0: ref<int32, managed, readonly> = null
    trap.panic value0
}";
    assert_eq!(output, expected);
}

/// SSA variable definition and use within a single block.
#[test]
fn test_ssa_define_use_single_block() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("varTest", &[], i32_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    // define variable and use it
    let variable = builder.variable(i32_type);
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
function varTest(): int32 {
entry0:
    value0: int32 = 10int32
    return value0
}";
    assert_eq!(output, expected);
}

/// SSA variable redefinition in the same block should return the latest value.
#[test]
fn test_ssa_redefine_variable() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("redefine", &[], i32_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    // define variable twice
    let variable = builder.variable(i32_type);
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
function redefine(): int32 {
entry0:
    value0: int32 = 1int32
    value1: int32 = 2int32
    return value1
}";
    assert_eq!(output, expected);
}

/// SSA construction across multiple blocks creates block parameters (φ-functions).
#[test]
fn test_ssa_branch_with_phi() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let bool_type = module.type_boolean();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("phiTest", &[bool_type], i32_type);

    // create blocks
    let entry_block = builder.block();
    let then_block = builder.block();
    let else_block = builder.block();
    let merge_block = builder.block();

    // create variable for the result
    let result_variable = builder.variable(i32_type);

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
function phiTest(value0: boolean): int32 {
entry0(value0: boolean):
    branch value0, block1, block2

block1:
    value1: int32 = 1int32
    jump block3(value1)

block2:
    value2: int32 = 0int32
    jump block3(value2)

block3(value3: int32):
    return value3
}";
    assert_eq!(output, expected);
}

/// Trivial φ removal: if all predecessors have the same value, no block parameter is needed.
#[test]
fn test_ssa_trivial_phi_removal() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let bool_type = module.type_boolean();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("trivialPhi", &[bool_type], i32_type);

    // create blocks
    let entry_block = builder.block();
    let then_block = builder.block();
    let else_block = builder.block();
    let merge_block = builder.block();

    // create variable
    let result_variable = builder.variable(i32_type);

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
function trivialPhi(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 42int32
    branch value0, block1, block2

block1:
    jump block3

block2:
    jump block3

block3:
    return value1
}";
    assert_eq!(output, expected);
}

/// Trivial phi removal is skipped for incomplete phis.
#[test]
fn test_ssa_trivial_phi_unsealed() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let bool_type = module.type_boolean();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("trivialPhiUnsealed", &[bool_type], i32_type);

    // create blocks
    let entry_block = builder.block();
    let then_block = builder.block();
    let else_block = builder.block();
    let merge_block = builder.block();

    // create variable
    let result_variable = builder.variable(i32_type);

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

    // merge block: use variable before sealing to create an incomplete phi
    builder.switch_to_block(merge_block);
    let result_value = builder.use_variable(result_variable);
    builder.return_(Some(result_value));
    builder.finish();

    // verify output: trivial phi removal rewrites the unsealed use
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function trivialPhiUnsealed(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 42int32
    branch value0, block1, block2

block1:
    jump block3

block2:
    jump block3

block3:
    return value1
}";
    assert_eq!(output, expected);
}

/// All arithmetic operations in sequence.
#[test]
fn test_build_arithmetic_operations() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("arithmetic", &[i32_type, i32_type], i32_type);
    let entry_block = builder.block();
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
function arithmetic(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = int.add value0, value1
    value3: int32 = int.sub value2, value1
    value4: int32 = int.mul value3, value0
    value5: int32 = int.div.s value4, value1
    return value5
}";
    assert_eq!(output, expected);
}

/// Comparison operations.
#[test]
fn test_build_comparison_operations() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let bool_type = module.type_boolean();

    // build function
    let mut builder = module.function("compare", &[i32_type, i32_type], bool_type);
    let entry_block = builder.block();
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
function compare(value0: int32, value1: int32): boolean {
entry0(value0: int32, value1: int32):
    value2: boolean = int.eq value0, value1
    value3: boolean = int.lt.s value0, value1
    value4: boolean = int.and value2, value3
    return value4
}";
    assert_eq!(output, expected);
}

/// Type construction methods create correct types.
#[test]
fn test_type_construction() {
    use crate::Type;

    // setup
    let mut module = ModuleBuilder::unchecked();

    // create various types
    let void_type = module.type_void();
    let bool_type = module.type_boolean();
    let i32_type = module.type_i32();
    let i64_type = module.type_i64();
    let f32_type = module.type_f32();
    let f64_type = module.type_f64();
    let pointer_type = module.type_raw_pointer(i32_type);
    let array_type = module.type_array(i32_type, 10, Copy::Yes);
    let tuple_type = module.type_tuple(vec![i32_type, i64_type], Copy::Yes);
    let signature = module.type_function_signature(vec![i32_type], i32_type);
    let function_pointer_type = module.type_function_pointer(signature);
    let callable_type = module.type_callable(signature);

    // verify types
    let (tree, _strings) = module.finish();
    assert!(matches!(tree.get(void_type), Type::Void));
    assert!(matches!(tree.get(bool_type), Type::Boolean));
    assert!(matches!(
        tree.get(i32_type),
        Type::Int {
            width: 32,
            is_signed: true
        }
    ));
    assert!(matches!(
        tree.get(i64_type),
        Type::Int {
            width: 64,
            is_signed: true
        }
    ));
    assert_eq!(tree.get(f32_type), &Type::FLOAT32);
    assert_eq!(tree.get(f64_type), &Type::FLOAT64);
    assert!(matches!(
        tree.get(pointer_type),
        Type::Reference {
            kind: ReferenceKind::Raw,
            ..
        }
    ));
    assert!(matches!(
        tree.get(array_type),
        Type::Array { length: 10, .. }
    ));
    assert!(matches!(tree.get(tuple_type), Type::Tuple { .. }));
    assert!(matches!(
        tree.get(function_pointer_type),
        Type::FunctionPointer { .. }
    ));
    assert!(matches!(tree.get(callable_type), Type::Callable { .. }));
}

/// seal_all_blocks seals all blocks at once.
#[test]
fn test_seal_all_blocks() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let void_type = module.type_void();

    // build multi-block function
    let mut builder = module.function("multiBlock", &[], void_type);
    let b0 = builder.block();
    let b1 = builder.block();
    let b2 = builder.block();

    // populate blocks
    builder.switch_to_block(b0);
    builder.jump(b1);
    builder.switch_to_block(b1);
    builder.jump(b2);
    builder.switch_to_block(b2);
    builder.return_(None);

    // seal all at once instead of individually
    builder.seal_all_blocks();
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function multiBlock(): void {
entry0:
    jump block1

block1:
    jump block2

block2:
    return
}";
    assert_eq!(output, expected);
}

/// Managed reference type construction.
#[test]
fn test_managed_reference_types() {
    use crate::Type;

    // setup
    let mut module = ModuleBuilder::unchecked();

    // create managed reference types
    let i32_type = module.type_i32();
    let managed_readonly_type = module.type_managed_reference(i32_type);
    let managed_mutable_type = module.type_managed_reference_mutable(i32_type);
    let managed_nullable_readonly_type = module.type_managed_reference_nullable(i32_type);
    let managed_nullable_mutable_type = module.type_managed_reference_nullable_mutable(i32_type);
    let owned_readonly_type = module.type_owned_reference_readonly(i32_type);
    let owned_mutable_type = module.type_owned_reference_mutable(i32_type);
    let raw_readonly_type = module.type_raw_pointer(i32_type);
    let raw_mutable_type = module.type_raw_pointer_mutable(i32_type);

    // verify types
    let (tree, _strings) = module.finish();
    assert!(matches!(
        tree.get(managed_readonly_type),
        Type::Reference {
            kind: ReferenceKind::Managed,
            access: Access::Readonly,
            is_nullable: false,
            ..
        }
    ));
    assert!(matches!(
        tree.get(managed_mutable_type),
        Type::Reference {
            kind: ReferenceKind::Managed,
            access: Access::Mutable,
            is_nullable: false,
            ..
        }
    ));
    assert!(matches!(
        tree.get(managed_nullable_readonly_type),
        Type::Reference {
            kind: ReferenceKind::Managed,
            access: Access::Readonly,
            is_nullable: true,
            ..
        }
    ));
    assert!(matches!(
        tree.get(managed_nullable_mutable_type),
        Type::Reference {
            kind: ReferenceKind::Managed,
            access: Access::Mutable,
            is_nullable: true,
            ..
        }
    ));
    assert!(matches!(
        tree.get(owned_readonly_type),
        Type::Reference {
            kind: ReferenceKind::Owned,
            access: Access::Readonly,
            is_nullable: false,
            ..
        }
    ));
    assert!(matches!(
        tree.get(owned_mutable_type),
        Type::Reference {
            kind: ReferenceKind::Owned,
            access: Access::Mutable,
            is_nullable: false,
            ..
        }
    ));
    assert!(matches!(
        tree.get(raw_readonly_type),
        Type::Reference {
            kind: ReferenceKind::Raw,
            access: Access::Readonly,
            is_nullable: false,
            ..
        }
    ));
    assert!(matches!(
        tree.get(raw_mutable_type),
        Type::Reference {
            kind: ReferenceKind::Raw,
            access: Access::Mutable,
            is_nullable: false,
            ..
        }
    ));
}

/// Allocation instruction: new.
#[test]
fn test_build_new() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let ref_type = module.type_managed_reference(i32_type);

    // build function with new
    let mut builder = module.function("allocTest", &[], ref_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let allocated_value = builder.new_(i32_type, ref_type);
    builder.return_(Some(allocated_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function allocTest(): ref<int32, managed, readonly> {
entry0:
    value0: ref<int32, managed, readonly> = new int32
    return value0
}";
    assert_eq!(output, expected);
}

/// Allocation instruction: new.slice.
#[test]
fn test_build_new_slice() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let i64_type = module.type_i64();
    let slice_type = module.type_slice(i32_type);

    // build function with new.slice
    let mut builder = module.function("allocArrayTest", &[i64_type], slice_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let length_value = builder.function_parameter(0);
    let allocated_value = builder.new_slice(i32_type, length_value, slice_type);
    builder.return_(Some(allocated_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function allocArrayTest(value0: int64): slice<int32> {
entry0(value0: int64):
    value1: slice<int32> = new.slice int32, value0
    return value1
}";
    assert_eq!(output, expected);
}

/// Allocation instruction: raw.alloc and raw.free.
#[test]
fn test_build_raw_alloc_and_free() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let void_type = module.type_void();
    let raw_ref_type = module.type_raw_pointer(i32_type);

    // build function with raw.alloc and raw.free
    let mut builder = module.function("rawAllocTest", &[], void_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let allocated_value = builder.raw_alloc(i32_type, raw_ref_type);
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
function rawAllocTest(): void {
entry0:
    value0: ref<int32, raw, readonly> = raw.alloc int32
    value1: int32 = 42int32
    store value0, value1
    raw.free value0
    return
}";
    assert_eq!(output, expected);
}

/// Allocation instruction: stack.alloc.
#[test]
fn test_build_stack_alloc() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let raw_ref_type = module.tree_mut().insert_type(Type::Reference {
        kind: ReferenceKind::Raw,
        lifetime: Lifetime::empty(),
        address_space: AddressSpace::Stack,
        access: Access::Readonly,
        pointee: i32_type.into(),
        is_nullable: false,
    });

    // build function with stack.alloc
    let mut builder = module.function("stackAllocTest", &[], raw_ref_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let allocated_value = builder.stack_alloc(i32_type, raw_ref_type);
    builder.return_(Some(allocated_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function stackAllocTest(): ref<int32, raw, readonly, space(stack)> {
entry0:
    value0: ref<int32, raw, readonly, space(stack)> = stack.alloc int32
    return value0
}";
    assert_eq!(output, expected);
}

/// Intrinsic instructions via the builder.
#[test]
fn test_build_intrinsics() {
    use crate::Intrinsic;

    // setup
    let mut module = ModuleBuilder::unchecked();
    let f64_type = module.type_f64();

    // build function with intrinsics
    let mut builder = module.function("intrinsicTest", &[f64_type, f64_type], f64_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    // chain of intrinsic operations
    let x = builder.function_parameter(0);
    let y = builder.function_parameter(1);
    let sqrt_x = builder.intrinsic(Intrinsic::Sqrt, f64_type, vec![x]);
    let min_val = builder.intrinsic(Intrinsic::Min, f64_type, vec![sqrt_x, y]);
    let result = builder.intrinsic(Intrinsic::Abs, f64_type, vec![min_val]);
    builder.return_(Some(result));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function intrinsicTest(value0: float64, value1: float64): float64 {
entry0(value0: float64, value1: float64):
    value2: float64 = intrinsic.sqrt(value0)
    value3: float64 = intrinsic.min(value2, value1)
    value4: float64 = intrinsic.abs(value3)
    return value4
}";
    assert_eq!(output, expected);
}

/// Void intrinsic instruction via the builder.
#[test]
fn test_build_void_intrinsic() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let void_type = module.type_void();

    // build function with void intrinsic
    let mut builder = module.function("fenceTest", &[], void_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let access = FenceAccess::new(
        MemoryOrdering::SequentiallyConsistent,
        SyncScope::Device,
        MemoryScope::Device,
        MemoryFlags::default(),
    );
    builder.atomic_fence(access);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function fenceTest(): void {
entry0:
    atomic.fence sequentiallyConsistent, scope(device), memory(device)
    return
}";
    assert_eq!(output, expected);
}

/// Struct instruction for building structs.
#[test]
fn test_build_struct() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let f64_type = module.type_f64();

    // create a struct type {i32, f64}
    let value0 = module.field(None, i32_type);
    let value1 = module.field(None, f64_type);
    let struct_type = module.type_struct(vec![value0, value1], Copy::Yes);

    // build function that constructs a struct
    let mut builder = module.function("makePoint", &[i32_type, f64_type], struct_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let x = builder.function_parameter(0);
    let y = builder.function_parameter(1);
    let point = builder.struct_(struct_type, vec![x, y]);
    builder.return_(Some(point));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function makePoint(value0: int32, value1: float64): { int32, float64 } {
entry0(value0: int32, value1: float64):
    value2: { int32, float64 } = struct { int32, float64 } (value0, value1)
    return value2
}";
    assert_eq!(output, expected);
}

/// Tuple instruction for building tuples.
#[test]
fn test_build_tuple() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let bool_type = module.type_boolean();
    let tuple_type = module.type_tuple(vec![i32_type, bool_type], Copy::Yes);

    // build function that constructs a tuple
    let mut builder = module.function("makePair", &[i32_type, bool_type], tuple_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let a = builder.function_parameter(0);
    let b = builder.function_parameter(1);
    let pair = builder.tuple(tuple_type, vec![a, b]);
    builder.return_(Some(pair));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function makePair(value0: int32, value1: boolean): (int32, boolean) {
entry0(value0: int32, value1: boolean):
    value2: (int32, boolean) = tuple (int32, boolean) (value0, value1)
    return value2
}";
    assert_eq!(output, expected);
}

/// Array instruction for building arrays.
#[test]
fn test_build_array() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let array_type = module.type_array(i32_type, 3, Copy::Yes);

    // build function that constructs an array
    let mut builder = module.function("makeArray", &[], array_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let v0 = builder.iconst_i32(1);
    let v1 = builder.iconst_i32(2);
    let v2 = builder.iconst_i32(3);
    let arr = builder.array(array_type, vec![v0, v1, v2]);
    builder.return_(Some(arr));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function makeArray(): int32[3] {
entry0:
    value0: int32 = 1int32
    value1: int32 = 2int32
    value2: int32 = 3int32
    value3: int32[3] = array int32[3] (value0, value1, value2)
    return value3
}";
    assert_eq!(output, expected);
}

/// Extract a field from a struct using field_get.
#[test]
fn test_build_field_get_struct() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let f64_type = module.type_f64();
    let value0 = module.field(None, i32_type);
    let value1 = module.field(None, f64_type);
    let struct_type = module.type_struct(vec![value0, value1], Copy::Yes);

    // build function that extracts the second field
    let mut builder = module.function("getY", &[struct_type], f64_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let point = builder.function_parameter(0);
    let y = builder.field_get(point, 1);
    builder.return_(Some(y));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function getY(value0: { int32, float64 }): float64 {
entry0(value0: { int32, float64 }):
    value1: float64 = field.get value0, 1
    return value1
}";
    assert_eq!(output, expected);
}

/// Extract an element from a tuple using field_get.
#[test]
fn test_build_field_get_tuple() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let bool_type = module.type_boolean();
    let tuple_type = module.type_tuple(vec![i32_type, bool_type], Copy::Yes);

    // build function that extracts the first element
    let mut builder = module.function("getFirst", &[tuple_type], i32_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let pair = builder.function_parameter(0);
    let first = builder.field_get(pair, 0);
    builder.return_(Some(first));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function getFirst(value0: (int32, boolean)): int32 {
entry0(value0: (int32, boolean)):
    value1: int32 = field.get value0, 0
    return value1
}";
    assert_eq!(output, expected);
}

/// Extract an element from an array using element_get.
#[test]
fn test_build_element_get_array() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let i64_type = module.type_i64();
    let array_type = module.type_array(i32_type, 3, Copy::Yes);

    // build function that extracts one fixed element
    let mut builder = module.function("getElement", &[array_type, i64_type], i32_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let arr = builder.function_parameter(0);
    let element = builder.element_get(arr, 1);
    builder.return_(Some(element));
    builder.seal_block(entry_block);
    builder.finish();

    // verify output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function getElement(value0: int32[3], value1: int64): int32 {
entry0(value0: int32[3], value1: int64):
    value2: int32 = element.get value0, 1
    return value2
}";
    assert_eq!(output, expected);
}

/// SSA construction with variable pass-through intermediate block.
///
/// Tests the case where:
/// - b0: defines x, jumps to b1
/// - b1 (loop header): uses x, branches to b2 or b4
/// - b2 (body): updates x, jumps to b3
/// - b3 (intermediate): does NOT use x, jumps back to b1
/// - b4 (exit): returns x
///
/// The updated x from b2 must flow through b3 to b1.
#[test]
fn test_ssa_passthrough_intermediate_block() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let bool_type = module.type_boolean();

    // build function
    let mut builder = module.function("passthrough", &[bool_type], i32_type);

    // create blocks
    let b0 = builder.block(); // init
    let b1 = builder.block(); // loop header
    let b2 = builder.block(); // body (updates x)
    let b3 = builder.block(); // intermediate (pass-through)
    let b4 = builder.block(); // exit

    // create variable
    let x_var = builder.variable(i32_type);

    // b0: define x = 1, jump to header
    builder.switch_to_block(b0);
    let init_value = builder.iconst_i32(1);
    builder.define_variable(x_var, init_value);
    builder.jump(b1);
    builder.seal_block(b0);

    // b1 (header): use x, branch based on condition
    builder.switch_to_block(b1);
    let cond = builder.function_parameter(0);
    let _x_header = builder.use_variable(x_var); // use x in header
    builder.branch(cond, b2, b4);
    // don't seal yet - has back edge from b3

    // b2 (body): update x = x + 10
    builder.switch_to_block(b2);
    let x_body = builder.use_variable(x_var);
    let ten = builder.iconst_i32(10);
    let x_new = builder.iadd(x_body, ten);
    builder.define_variable(x_var, x_new);
    builder.jump(b3);
    builder.seal_block(b2);

    // b3 (intermediate): does NOT touch x, just jumps back to header
    builder.switch_to_block(b3);
    // intentionally no use or define of x_var here
    builder.jump(b1);
    builder.seal_block(b3);

    // now seal b1 (all predecessors known: b0, b3)
    builder.seal_block(b1);

    // b4 (exit): return x
    builder.switch_to_block(b4);
    let x_exit = builder.use_variable(x_var);
    builder.return_(Some(x_exit));
    builder.seal_block(b4);

    builder.finish();

    // verify output
    // the key check: b3 must pass the updated x to b1
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());

    // expected: b3 passes the updated x (v4) from b2 to b1
    // note: b3 has no block parameter since it has only one predecessor
    let expected = "\
function passthrough(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 1int32
    jump block1(value1)

block1(value2: int32):
    branch value0, block2, block4

block2:
    value4: int32 = 10int32
    value5: int32 = int.add value2, value4
    jump block3

block3:
    jump block1(value5)

block4:
    return value2
}";
    assert_eq!(output, expected);
}

/// SSA construction with multiple variables needing phis at the same merge point.
///
/// Tests that block parameter and argument ordering is correct when multiple
/// variables need phis at the same block.
#[test]
fn test_ssa_multiple_phis_at_merge() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let bool_type = module.type_boolean();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("multiPhi", &[bool_type], i32_type);

    // create blocks: diamond CFG
    let entry = builder.block();
    let then_block = builder.block();
    let else_block = builder.block();
    let merge = builder.block();

    // create two variables
    let x_var = builder.variable(i32_type);
    let y_var = builder.variable(i32_type);

    // entry: branch
    builder.switch_to_block(entry);
    let cond = builder.function_parameter(0);
    builder.branch(cond, then_block, else_block);
    builder.seal_block(entry);

    // then: x = 1, y = 10
    builder.switch_to_block(then_block);
    let x_then = builder.iconst_i32(1);
    let y_then = builder.iconst_i32(10);
    builder.define_variable(x_var, x_then);
    builder.define_variable(y_var, y_then);
    builder.jump(merge);
    builder.seal_block(then_block);

    // else: x = 2, y = 20
    builder.switch_to_block(else_block);
    let x_else = builder.iconst_i32(2);
    let y_else = builder.iconst_i32(20);
    builder.define_variable(x_var, x_else);
    builder.define_variable(y_var, y_else);
    builder.jump(merge);
    builder.seal_block(else_block);

    // merge: use x and y, return x + y
    builder.switch_to_block(merge);
    builder.seal_block(merge);
    let x_val = builder.use_variable(x_var);
    let y_val = builder.use_variable(y_var);
    let sum = builder.iadd(x_val, y_val);
    builder.return_(Some(sum));
    builder.finish();

    // verify output: two block parameters, arguments in correct order
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function multiPhi(value0: boolean): int32 {
entry0(value0: boolean):
    branch value0, block1, block2

block1:
    value1: int32 = 1int32
    value2: int32 = 10int32
    jump block3(value1, value2)

block2:
    value3: int32 = 2int32
    value4: int32 = 20int32
    jump block3(value3, value4)

block3(value5: int32, value6: int32):
    value7: int32 = int.add value5, value6
    return value7
}";
    assert_eq!(output, expected);
}
