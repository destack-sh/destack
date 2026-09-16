use destack_core::StringPool;

use crate::build::ModuleBuilder;
use crate::parse::{ParseOptions, Parser, test_file};
use crate::{
    type_lifetime, type_contains_borrowed_refs, type_borrowed_paths, 
    Access, BinaryOperator, Callee, Copy, ExecutionScope, Extent, FenceAccess, FloatType,
    FormatOptions, Formatter, GenericArgument, GenericParameter, GenericParameterDomain, Importer,
    LayoutBuilder, LayoutTable, Lifetime, MemoryOrdering, Multiplicity, Mutability, Place,
    Projection, Reference, Space, Storage, StorageSet, Substitution, Symbol, TargetLayout,
    TraceMap, Tree, Type, TypeDeclaration,
};

/// Format one test MIR tree.
fn format_test_mir(tree: &Tree, strings: &StringPool) -> String {
    Formatter::new(
        tree,
        TargetLayout::default(),
        strings,
        FormatOptions::default(),
    )
    .format()
    .expect("format MIR")
}

/// Empty function with void return.
#[test]
fn test_build_empty_function() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let void_type = module.type_void();

    // build empty function
    let header = module.function_header("empty").result(void_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function empty(): void {
entry:
    return
}";
    assert_eq!(output, expected);
}

/// Function with two parameters that are added together.
#[test]
fn test_build_function_with_parameters() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);

    // build add function
    let header = module
        .function_header("add")
        .parameters([i32_type, i32_type])
        .result(i32_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let left_value = builder.function_parameter(0);
    let right_value = builder.function_parameter(1);
    let sum_value = builder.binary(BinaryOperator::Add, left_value, right_value);
    builder.return_(Some(sum_value));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function add(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    return v2
}";
    assert_eq!(output, expected);
}

/// Function with a local variable (stack slot).
#[test]
fn test_build_function_with_locals() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i64_type = module.type_int(64, true);

    // build function with local
    let header = module.function_header("withLocal").result(i64_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    // create local, store, then load
    let local = builder.local(i64_type, Mutability::Mutable);
    let constant_value = builder.iconst_i64(42);
    builder.local_set(local, constant_value);
    let loaded_value = builder.local_get(local);
    builder.return_(Some(loaded_value));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function withLocal(): int64 {
    local l0: int64

entry:
    v0: int64 = 42
    store l0, v0
    v1: int64 = load l0
    return v1
}";
    assert_eq!(output, expected);
}

/// Function with conditional branch and multiple blocks.
#[test]
fn test_build_function_with_branch() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let bool_type = module.type_boolean();
    let i32_type = module.type_int(32, true);

    // build function with branch
    let header = module
        .function_header("select")
        .parameters([bool_type])
        .result(i32_type);
    let mut builder = module.function(header);

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
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function select(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = 1
    jump b3

b2:
    v2: int32 = 0
    jump b3

b3:
    return v1
}";
    assert_eq!(output, expected);
}

/// Invokes carry explicit normal and unwind continuations.
#[test]
fn test_build_function_with_invoke() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let signature = module.type_function_signature(vec![i32_type], i32_type);
    let callee_header = module
        .function_header("callee")
        .parameters([i32_type])
        .result(i32_type);
    let callee = module.external_function(callee_header);

    // build function
    let header = module
        .function_header("caller")
        .parameters([i32_type])
        .result(i32_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    let target_block = builder.block();
    let unwind_block = builder.block();

    // entry: branch through the call
    builder.switch_to_block(entry_block);
    let argument = builder.function_parameter(0);
    let result = builder.add_block_parameter(target_block, i32_type);
    builder.invoke(
        Callee::Direct {
            function: callee,
            arguments: Vec::new(),
        },
        signature,
        vec![argument],
        target_block,
        Vec::new(),
        unwind_block,
        Vec::new(),
    );
    builder.seal_block(entry_block);

    // continuation
    builder.switch_to_block(target_block);
    builder.return_(Some(result));
    builder.seal_block(target_block);

    // unwind continuation
    builder.switch_to_block(unwind_block);
    builder.resume_unwind();
    builder.seal_block(unwind_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
external function callee(int32): int32

function caller(v0: int32): int32 {
entry(v0: int32):
    invoke callee(v0): (int32) => int32 => b1 | b2

b1(v1: int32):
    return v1

b2:
    unwind.resume
}";
    assert_eq!(output, expected);
}

/// Calls derive SSA destinations from their signature result.
#[test]
fn test_build_calls_from_callee_and_signature() {
    // setup callable declarations
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let void_type = module.type_void();
    let i32_type = module.type_int(32, true);
    let identity_signature = module.type_function_signature(vec![i32_type], i32_type);
    let sink_signature = module.type_function_signature(vec![i32_type], void_type);
    let identity_header = module
        .function_header("identity")
        .parameters([i32_type])
        .result(i32_type);
    let sink_header = module
        .function_header("sink")
        .parameters([i32_type])
        .result(void_type);
    let identity = module.external_function(identity_header);
    let sink = module.external_function(sink_header);

    // build value and void calls through the same operation
    let header = module
        .function_header("caller")
        .parameters([i32_type])
        .result(i32_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let argument = builder.function_parameter(0);
    let result = builder
        .call(
            Callee::Direct {
                function: identity,
                arguments: Vec::new(),
            },
            identity_signature,
            vec![argument],
            i32_type,
        )
        .expect("identity returns a value");
    let void_result = builder.call(
        Callee::Direct {
            function: sink,
            arguments: Vec::new(),
        },
        sink_signature,
        vec![result],
        void_type,
    );
    assert_eq!(void_result, None);
    builder.return_(Some(result));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify the complete MIR
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
external function identity(int32): int32

external function sink(int32): void

function caller(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call identity(v0): (int32) => int32
    call sink(v1): (int32) => void
    return v1
}";
    assert_eq!(output, expected);
}

/// Panic terminators format as explicit control exits.
#[test]
fn test_build_function_with_panic_terminator() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let string_type = module.type_reference(
        Reference::Managed,
        Lifetime::empty(),
        i32_type,
        Access::Readonly,
        Storage::Heap(Space::Local),
    );
    let void_type = module.type_void();

    // build function
    let header = module.function_header("panicker").result(void_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let payload = builder.null(string_type);
    builder.panic(Some(payload));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function panicker(): void {
entry:
    v0: ref<int32, managed, readonly, local> = null
    panic v0
}";
    assert_eq!(output, expected);
}

/// SSA variable definition and use within a single block.
#[test]
fn test_ssa_define_use_single_block() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);

    // build function
    let header = module.function_header("varTest").result(i32_type);
    let mut builder = module.function(header);
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
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function varTest(): int32 {
entry:
    v0: int32 = 10
    return v0
}";
    assert_eq!(output, expected);
}

/// SSA variable redefinition in the same block should return the latest value.
#[test]
fn test_ssa_redefine_variable() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);

    // build function
    let header = module.function_header("redefine").result(i32_type);
    let mut builder = module.function(header);
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
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function redefine(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    return v1
}";
    assert_eq!(output, expected);
}

/// SSA construction across multiple blocks creates block parameters (φ-functions).
#[test]
fn test_ssa_branch_with_phi() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let bool_type = module.type_boolean();
    let i32_type = module.type_int(32, true);

    // build function
    let header = module
        .function_header("phiTest")
        .parameters([bool_type])
        .result(i32_type);
    let mut builder = module.function(header);

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
    builder.finish().unwrap();

    // verify the merge block takes a block parameter
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function phiTest(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = 1
    jump b3(v1)

b2:
    v2: int32 = 0
    jump b3(v2)

b3(v3: int32):
    return v3
}";
    assert_eq!(output, expected);
}

/// Trivial φ removal: if all predecessors have the same value, no block parameter is needed.
#[test]
fn test_ssa_trivial_phi_removal() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let bool_type = module.type_boolean();
    let i32_type = module.type_int(32, true);

    // build function
    let header = module
        .function_header("trivialPhi")
        .parameters([bool_type])
        .result(i32_type);
    let mut builder = module.function(header);

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

    // then block: jump without redefining
    builder.switch_to_block(then_block);
    builder.jump(merge_block);
    builder.seal_block(then_block);

    // else block: jump without redefining
    builder.switch_to_block(else_block);
    builder.jump(merge_block);
    builder.seal_block(else_block);

    // merge block: use the variable, a trivial phi keeps the block parameterless
    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);
    let result_value = builder.use_variable(result_variable);
    builder.return_(Some(result_value));
    builder.finish().unwrap();

    // verify the merge block stays parameterless
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function trivialPhi(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 42
    branch v0 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    return v1
}";
    assert_eq!(output, expected);
}

/// Trivial phi removal is skipped for incomplete phis.
#[test]
fn test_ssa_trivial_phi_unsealed() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let bool_type = module.type_boolean();
    let i32_type = module.type_int(32, true);

    // build function
    let header = module
        .function_header("trivialPhiUnsealed")
        .parameters([bool_type])
        .result(i32_type);
    let mut builder = module.function(header);

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

    // then block: jump without redefining
    builder.switch_to_block(then_block);
    builder.jump(merge_block);
    builder.seal_block(then_block);

    // else block: jump without redefining
    builder.switch_to_block(else_block);
    builder.jump(merge_block);
    builder.seal_block(else_block);

    // merge block: use variable before sealing to create an incomplete phi
    builder.switch_to_block(merge_block);
    let result_value = builder.use_variable(result_variable);
    builder.return_(Some(result_value));
    builder.finish().unwrap();

    // verify output: trivial phi removal rewrites the unsealed use
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function trivialPhiUnsealed(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 42
    branch v0 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    return v1
}";
    assert_eq!(output, expected);
}

/// All arithmetic operations in sequence.
#[test]
fn test_build_arithmetic_operations() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);

    // build function
    let header = module
        .function_header("arithmetic")
        .parameters([i32_type, i32_type])
        .result(i32_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    // chain of arithmetic operations
    let left_value = builder.function_parameter(0);
    let right_value = builder.function_parameter(1);
    let sum_value = builder.binary(BinaryOperator::Add, left_value, right_value);
    let difference_value = builder.binary(BinaryOperator::Subtract, sum_value, right_value);
    let product_value = builder.binary(BinaryOperator::Multiply, difference_value, left_value);
    let quotient_value = builder.binary(BinaryOperator::Divide, product_value, right_value);

    builder.return_(Some(quotient_value));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function arithmetic(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v3: int32 = sub v2, v1
    v4: int32 = mul v3, v0
    v5: int32 = div v4, v1
    return v5
}";
    assert_eq!(output, expected);
}

/// Comparison operations.
#[test]
fn test_build_comparison_operations() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let bool_type = module.type_boolean();

    // build function
    let header = module
        .function_header("compare")
        .parameters([i32_type, i32_type])
        .result(bool_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    // comparison operations
    let left_value = builder.function_parameter(0);
    let right_value = builder.function_parameter(1);
    let equal_value = builder.binary(BinaryOperator::Equal, left_value, right_value);
    let less_than_value = builder.binary(BinaryOperator::LessThan, left_value, right_value);
    let result_value = builder.binary(BinaryOperator::And, equal_value, less_than_value);

    builder.return_(Some(result_value));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function compare(v0: int32, v1: int32): boolean {
entry(v0: int32, v1: int32):
    v2: boolean = eq v0, v1
    v3: boolean = lt v0, v1
    v4: boolean = and v2, v3
    return v4
}";
    assert_eq!(output, expected);
}

/// Type construction methods create correct types.
#[test]
fn test_type_construction() {
    use crate::Type;

    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);

    // create various types
    let void_type = module.type_void();
    let bool_type = module.type_boolean();
    let character_type = module.type_character();
    let i32_type = module.type_int(32, true);
    let i64_type = module.type_int(64, true);
    let f32_type = module.type_float(FloatType::Float32);
    let f64_type = module.type_float(FloatType::Float64);
    let pointer_type = module.type_pointer(i32_type, Access::Readonly);
    let array_type = module.type_fixed_array(i32_type, 10);
    let tuple_type = module.type_tuple(vec![i32_type, i64_type], Copy::Yes);
    let signature = module.type_function_signature(vec![i32_type], i32_type);
    let function_pointer_type = module.type_function_pointer(signature);
    let callable_type = module.type_function(
        signature,
        Multiplicity::Repeatable,
        Reference::Managed,
        Lifetime::empty(),
        Storage::Heap(Space::Local),
        Access::Mutable,
    );

    // verify types
    let (tree, _strings) = module.finish_tree();
    assert!(matches!(tree.get(void_type), Type::Void));
    assert!(matches!(tree.get(bool_type), Type::Boolean));
    assert!(matches!(tree.get(character_type), Type::Character));
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
    assert!(matches!(tree.get(pointer_type), Type::Pointer { .. }));
    assert!(matches!(
        tree.get(array_type),
        Type::FixedArray { length, .. } if tree.static_value(*length).length() == Some(10)
    ));
    assert!(matches!(tree.get(tuple_type), Type::Tuple { .. }));
    assert!(matches!(
        tree.get(function_pointer_type),
        Type::FunctionPointer { .. }
    ));
    assert!(matches!(tree.get(callable_type), Type::Function { .. }));
}

/// seal_all_blocks seals all blocks at once.
#[test]
fn test_seal_all_blocks() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let void_type = module.type_void();

    // build multi-block function
    let header = module.function_header("multiBlock").result(void_type);
    let mut builder = module.function(header);
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

    // seal every block at once
    builder.seal_all_blocks();
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function multiBlock(): void {
entry:
    jump b1

b1:
    jump b2

b2:
    return
}";
    assert_eq!(output, expected);
}

/// Constructs reference and pointer access forms.
#[test]
fn test_construct_reference_and_pointer_types() {
    use crate::Type;

    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);

    // create managed reference types
    let i32_type = module.type_int(32, true);
    let managed_readonly_type = module.type_reference(
        Reference::Managed,
        Lifetime::empty(),
        i32_type,
        Access::Readonly,
        Storage::Heap(Space::Local),
    );
    let managed_mutable_type = module.type_reference(
        Reference::Managed,
        Lifetime::empty(),
        i32_type,
        Access::Mutable,
        Storage::Heap(Space::Local),
    );
    let pointer_readonly_type = module.type_pointer(i32_type, Access::Readonly);
    let pointer_mutable_type = module.type_pointer(i32_type, Access::Mutable);

    // verify types
    let (tree, _strings) = module.finish_tree();
    assert!(matches!(
        tree.get(managed_readonly_type),
        Type::Reference {
            kind: Reference::Managed,
            access: Access::Readonly,
            ..
        }
    ));
    assert!(matches!(
        tree.get(managed_mutable_type),
        Type::Reference {
            kind: Reference::Managed,
            access: Access::Mutable,
            ..
        }
    ));
    assert!(matches!(
        tree.get(pointer_readonly_type),
        Type::Pointer {
            access: Access::Readonly,
            ..
        }
    ));
    assert!(matches!(
        tree.get(pointer_mutable_type),
        Type::Pointer {
            access: Access::Mutable,
            ..
        }
    ));
}

/// Allocation instruction: new.zeroed.
#[test]
fn test_build_new_zeroed() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let ref_type = module.type_reference(
        Reference::Managed,
        Lifetime::empty(),
        i32_type,
        Access::Readonly,
        Storage::Heap(Space::Local),
    );

    // build function with new.zeroed
    let header = module.function_header("allocTest").result(ref_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let allocated_value = builder.new_zeroed(i32_type, ref_type);
    builder.return_(Some(allocated_value));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function allocTest(): ref<int32, managed, readonly, local> {
entry:
    v0: ref<int32, managed, readonly, local> = new.zeroed int32
    return v0
}";
    assert_eq!(output, expected);
}

/// Allocation instruction: new.slice.zeroed.
#[test]
fn test_build_new_slice_zeroed() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let i64_type = module.type_int(64, true);
    let slice_type = module.type_slice(
        Reference::Managed,
        Lifetime::empty(),
        i32_type,
        Access::Mutable,
        Storage::Heap(Space::Local),
    );

    // build function with new.slice.zeroed
    let header = module
        .function_header("allocArrayTest")
        .parameters([i64_type])
        .result(slice_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let length_value = builder.function_parameter(0);
    let allocated_value = builder.new_slice_zeroed(i32_type, length_value, slice_type);
    builder.return_(Some(allocated_value));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function allocArrayTest(v0: int64): slice<int32, managed, mutable, local> {
entry(v0: int64):
    v1: slice<int32, managed, mutable, local> = new.slice.zeroed int32, v0
    return v1
}";
    assert_eq!(output, expected);
}

/// Slice descriptor instruction: slice.
#[test]
fn test_build_slice_view() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let i64_type = module.type_int(64, true);
    let source_type = module.type_slice(
        Reference::Managed,
        Lifetime::empty(),
        i32_type,
        Access::Mutable,
        Storage::Heap(Space::Local),
    );
    let slice_type = module.type_slice(
        Reference::Borrowed,
        Lifetime::bound(0),
        i32_type,
        Access::Mutable,
        Storage::Heap(Space::Local),
    );

    // build function with slice view
    let header = module
        .function_header("sliceTest")
        .lifetime("'a")
        .parameters([source_type, i64_type, i64_type])
        .result(slice_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let source_value = builder.function_parameter(0);
    let start_value = builder.function_parameter(1);
    let length_value = builder.function_parameter(2);
    let slice_value = builder.address(
        Place::value(source_value)
            .with_projection(Projection::Deref)
            .with_projection(Projection::Slice {
                start: start_value,
                length: length_value,
            }),
        slice_type,
    );
    builder.return_(Some(slice_value));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function sliceTest<'a>(v0: slice<int32, managed, mutable, local>, v1: int64, v2: int64): slice<int32, borrowed, 'a & local, mutable> {
entry(v0: slice<int32, managed, mutable, local>, v1: int64, v2: int64):
    v3: slice<int32, borrowed, 'a & local, mutable> = address (*v0)[v1; v2]
    return v3
}";
    assert_eq!(output, expected);
}

/// Intrinsic instructions via the builder.
#[test]
fn test_build_intrinsics() {
    use crate::Intrinsic;

    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let f64_type = module.type_float(FloatType::Float64);

    // build function with intrinsics
    let header = module
        .function_header("intrinsicTest")
        .parameters([f64_type, f64_type])
        .result(f64_type);
    let mut builder = module.function(header);
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
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function intrinsicTest(v0: float64, v1: float64): float64 {
entry(v0: float64, v1: float64):
    v2: float64 = intrinsic.math.float.sqrt(v0)
    v3: float64 = intrinsic.math.float.min(v2, v1)
    v4: float64 = intrinsic.math.float.abs(v3)
    return v4
}";
    assert_eq!(output, expected);
}

/// Void intrinsic instruction via the builder.
#[test]
fn test_build_void_intrinsic() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let void_type = module.type_void();

    // build function with void intrinsic
    let header = module.function_header("fenceTest").result(void_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let access = FenceAccess::new(
        MemoryOrdering::SequentiallyConsistent,
        ExecutionScope::Device,
        StorageSet::SHARED,
    );
    builder.atomic_fence(access);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function fenceTest(): void {
entry:
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    return
}";
    assert_eq!(output, expected);
}

/// Build one struct aggregate.
#[test]
fn test_build_struct_aggregate() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let f64_type = module.type_float(FloatType::Float64);

    // create a struct type {i32, f64}
    let value0 = module.field(None, i32_type);
    let value1 = module.field(None, f64_type);
    let struct_type = module.type_struct(vec![value0, value1], Copy::Yes);

    // build function that constructs a struct
    let header = module
        .function_header("makePoint")
        .parameters([i32_type, f64_type])
        .result(struct_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let x = builder.function_parameter(0);
    let y = builder.function_parameter(1);
    let point = builder.aggregate(struct_type, vec![x, y]);
    builder.return_(Some(point));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function makePoint(v0: int32, v1: float64): { int32, float64 } {
entry(v0: int32, v1: float64):
    v2: { int32, float64 } = aggregate (v0, v1)
    return v2
}";
    assert_eq!(output, expected);
}

/// Build one tuple aggregate.
#[test]
fn test_build_tuple_aggregate() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let bool_type = module.type_boolean();
    let tuple_type = module.type_tuple(vec![i32_type, bool_type], Copy::Yes);

    // build function that constructs a tuple
    let header = module
        .function_header("makePair")
        .parameters([i32_type, bool_type])
        .result(tuple_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let a = builder.function_parameter(0);
    let b = builder.function_parameter(1);
    let pair = builder.aggregate(tuple_type, vec![a, b]);
    builder.return_(Some(pair));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function makePair(v0: int32, v1: boolean): (int32, boolean) {
entry(v0: int32, v1: boolean):
    v2: (int32, boolean) = aggregate (v0, v1)
    return v2
}";
    assert_eq!(output, expected);
}

/// Build one fixed-array aggregate.
#[test]
fn test_build_array_aggregate() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let array_type = module.type_fixed_array(i32_type, 3);

    // build function that constructs an array
    let header = module.function_header("makeArray").result(array_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let v0 = builder.iconst_i32(1);
    let v1 = builder.iconst_i32(2);
    let v2 = builder.iconst_i32(3);
    let arr = builder.aggregate(array_type, vec![v0, v1, v2]);
    builder.return_(Some(arr));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function makeArray(): [int32; 3] {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: [int32; 3] = aggregate (v0, v1, v2)
    return v3
}";
    assert_eq!(output, expected);
}

/// Extract a field from a struct using field_get.
#[test]
fn test_build_field_get_struct() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let f64_type = module.type_float(FloatType::Float64);
    let value0 = module.field(None, i32_type);
    let value1 = module.field(None, f64_type);
    let struct_type = module.type_struct(vec![value0, value1], Copy::Yes);

    // build function that extracts the second field
    let header = module
        .function_header("getY")
        .parameters([struct_type])
        .result(f64_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let point = builder.function_parameter(0);
    let y = builder.field_get(point, 1);
    builder.return_(Some(y));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function getY(v0: { int32, float64 }): float64 {
entry(v0: { int32, float64 }):
    v1: float64 = field.get v0, 1
    return v1
}";
    assert_eq!(output, expected);
}

/// Extract an element from a tuple using field_get.
#[test]
fn test_build_field_get_tuple() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let bool_type = module.type_boolean();
    let tuple_type = module.type_tuple(vec![i32_type, bool_type], Copy::Yes);

    // build function that extracts the first element
    let header = module
        .function_header("getFirst")
        .parameters([tuple_type])
        .result(i32_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let pair = builder.function_parameter(0);
    let first = builder.field_get(pair, 0);
    builder.return_(Some(first));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function getFirst(v0: (int32, boolean)): int32 {
entry(v0: (int32, boolean)):
    v1: int32 = field.get v0, 0
    return v1
}";
    assert_eq!(output, expected);
}

/// Field projection substitutes the region arguments of an identified aggregate.
#[test]
fn test_build_field_get_from_region_applied_type() {
    // define the referenced user type
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let int32 = module.type_int(32, true);
    let user_field_name = module.strings().intern("id");
    let user_field = module.field(Some(user_field_name), int32);
    let user_name = module.strings().intern("User");
    let user_declaration = module
        .tree_mut()
        .reserve_type(Symbol::named(crate::TEST_MODULE, user_name));
    let definition = module.tree_mut().intern_type(
        Type::Struct {
            fields: vec![user_field],
        },
        Copy::Yes,
    );
    let declaration = module.tree_mut().get_mut(user_declaration);
    declaration.definition = Some(definition);
    declaration.name = Some(user_name);
    let user = module.tree_mut().intern_type(Type::Declaration {
        declaration: user_declaration,
    }, Copy::No);

    // define a region-polymorphic aggregate borrowing the user
    let borrowed_user = module.type_reference(
        Reference::Borrowed,
        Lifetime::new([Extent::Parameter(0)]),
        user,
        Access::Readonly,
        Storage::Parameter(0),
    );
    let view_field_name = module.strings().intern("user");
    let view_field = module.field(Some(view_field_name), borrowed_user);
    let view_name = module.strings().intern("View");
    let region_name = module.strings().intern("'a");
    let region = GenericParameter {
        name: region_name,
        domain: GenericParameterDomain::Region {
            outlives: Vec::new(),
        },
    };
    let view_declaration = module
        .tree_mut()
        .reserve_type(Symbol::named(crate::TEST_MODULE, view_name));
    let definition = module.tree_mut().intern_type(
        Type::Struct {
            fields: vec![view_field],
        },
        Copy::Yes,
    );
    let declaration = module.tree_mut().get_mut(view_declaration);
    declaration.definition = Some(definition);
    declaration.name = Some(view_name);
    declaration.generics = vec![region];
    let view = module.tree_mut().intern_type(Type::Declaration {
        declaration: view_declaration,
    }, Copy::No);

    // project the field from one concrete region application
    let frame_view = module.tree_mut().intern_type(Type::Application {
        base: view,
        arguments: vec![GenericArgument::Region {
            lifetime: Lifetime::frame(),
            storage: Storage::Frame,
        }],
    }, Copy::No);
    let frame_user = module.type_reference(
        Reference::Borrowed,
        Lifetime::frame(),
        user,
        Access::Readonly,
        Storage::Frame,
    );
    let header = module
        .function_header("getFrame")
        .parameter(frame_view)
        .result(frame_user);
    let mut builder = module.function(header);
    let entry = builder.block();
    builder.switch_to_block(entry);
    let value = builder.function_parameter(0);
    let user = builder.field_get(value, 0);
    builder.return_(Some(user));
    builder.seal_block(entry);
    builder.finish().unwrap();

    // require the complete declared and projected MIR
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
@copy
type User {
    id: int32;
}

@copy
type View<'a> {
    user: ref<User, borrowed, 'a, readonly>;
}

function getFrame(v0: View<'frame & frame>): ref<User, borrowed, 'frame & frame, readonly> {
entry(v0: View<'frame & frame>):
    v1: ref<User, borrowed, 'frame & frame, readonly> = field.get v0, 0
    return v1
}";
    assert_eq!(output, expected);
}

/// Extract a fixed-array element with element_get.
#[test]
fn test_build_element_get_array() {
    // setup
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let i64_type = module.type_int(64, true);
    let array_type = module.type_fixed_array(i32_type, 3);

    // build function that extracts one fixed element
    let header = module
        .function_header("getElement")
        .parameters([array_type, i64_type])
        .result(i32_type);
    let mut builder = module.function(header);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    let arr = builder.function_parameter(0);
    let element = builder.element_get(arr, 1);
    builder.return_(Some(element));
    builder.seal_block(entry_block);
    builder.finish().unwrap();

    // verify output
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function getElement(v0: [int32; 3], v1: int64): int32 {
entry(v0: [int32; 3], v1: int64):
    v2: int32 = element.get v0, 1
    return v2
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
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let i32_type = module.type_int(32, true);
    let bool_type = module.type_boolean();

    // build function
    let header = module
        .function_header("passthrough")
        .parameters([bool_type])
        .result(i32_type);
    let mut builder = module.function(header);

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
    // leave b1 unsealed, b3 still branches back into it

    // b2 (body): update x = x + 10
    builder.switch_to_block(b2);
    let x_body = builder.use_variable(x_var);
    let ten = builder.iconst_i32(10);
    let x_new = builder.binary(BinaryOperator::Add, x_body, ten);
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

    builder.finish().unwrap();

    // verify output
    // the key check: b3 must pass the updated x to b1
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);

    // expected: b3 passes the updated x (v4) from b2 to b1
    // note: b3 has no block parameter since it has only one predecessor
    let expected = "\
function passthrough(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 1
    jump b1(v1)

b1(v2: int32):
    branch v0 => b2 | b4

b2:
    v4: int32 = 10
    v5: int32 = add v2, v4
    jump b3

b3:
    jump b1(v5)

b4:
    return v2
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
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let bool_type = module.type_boolean();
    let i32_type = module.type_int(32, true);

    // build function
    let header = module
        .function_header("multiPhi")
        .parameters([bool_type])
        .result(i32_type);
    let mut builder = module.function(header);

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
    let sum = builder.binary(BinaryOperator::Add, x_val, y_val);
    builder.return_(Some(sum));
    builder.finish().unwrap();

    // verify output: two block parameters, arguments in correct order
    let (tree, strings) = module.finish_tree();
    let output = format_test_mir(&tree, &strings);
    let expected = "\
function multiPhi(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = 1
    v2: int32 = 10
    jump b3(v1, v2)

b2:
    v3: int32 = 2
    v4: int32 = 20
    jump b3(v3, v4)

b3(v5: int32, v6: int32):
    v7: int32 = add v5, v6
    return v7
}";
    assert_eq!(output, expected);
}

/// Preserve joined places when importing a specialized function into a different type table.
#[test]
fn test_import_specialized_function_places() {
    // build a function whose parameter and arguments share one joined place
    let mut module = ModuleBuilder::new(crate::TEST_MODULE);
    let integer = module.type_int(32, true);
    let void = module.type_void();
    let space = module
        .tree_mut()
        .intern_space_join([Space::Local, Space::Shared]);
    let other_space = module
        .tree_mut()
        .intern_space_join([Space::Local, Space::Constant]);
    let storage = module.tree_mut().intern_storage_join([
        Storage::Frame,
        Storage::Heap(space),
        Storage::Heap(other_space),
        Storage::Static(Space::Constant),
    ]);
    let lifetime = Lifetime::new([Extent::Static]);
    let borrowed = module.tree_mut().intern_type(Type::Reference {
        kind: Reference::Borrowed,
        lifetime: lifetime.clone(),
        storage,
        access: Access::Readonly,
        pointee: integer,
    }, Copy::Yes);
    let header = module
        .function_header("selected")
        .arguments([
            GenericArgument::Space(space),
            GenericArgument::Region { lifetime, storage },
        ])
        .parameter(borrowed)
        .result(void);
    let function = module.declare_function(header);
    let (source, strings) = module.finish_tree();

    // occupy the source join's index with a different join before importing
    let mut destination = Tree::new();
    destination.intern_space_join([Space::Local, Space::Constant]);
    destination.intern_storage_join([Storage::Frame, Storage::Static(Space::Local)]);
    let mut declared = |_| None;
    let imported =
        Importer::new(&mut destination, &mut declared).import_function_header(&source, function);
    let function = destination.insert(imported);

    // require matching places in both the signature and the specialization arguments
    assert_eq!(
        format_test_mir(&destination, &strings),
        "external function selected<local|shared, 'static & frame|heap(local|shared)|heap(local|constant)|constant>(ref<int32, borrowed, 'static & frame|heap(local|shared)|heap(local|constant)|constant, readonly>): void",
    );

    // trace every movable or reclaimable location after importing the joined borrow
    let parameter = destination.get(function).parameters[0].ty;
    assert_eq!(
        source.type_fingerprint(borrowed),
        destination.type_fingerprint(parameter)
    );
    let mut layouts = LayoutTable::new();
    let layout = LayoutBuilder::new(&destination, &mut layouts, TargetLayout::default())
        .layout_type(parameter)
        .unwrap();
    assert_eq!(
        layouts.layout(layout).trace_map,
        TraceMap::Fixed {
            local_offsets: Box::new([0]),
            shared_offsets: Box::new([0]),
            frame_offsets: Box::new([0]),
        }
    );

    // raw references preserve addressing without retaining storage or excluding null
    let mut raw = destination.type_definition(parameter).clone();
    let Type::Reference { kind, .. } = &mut raw else {
        panic!("expected the imported reference");
    };
    *kind = Reference::Raw;
    let raw = destination.intern_type(raw, Copy::Yes);
    let raw_layout = LayoutBuilder::new(&destination, &mut layouts, TargetLayout::default())
        .layout_type(raw)
        .unwrap();
    assert_eq!(layouts.layout(raw_layout).trace_map, TraceMap::Empty);
    assert_eq!(layouts.layout(raw_layout).niche, None);
    assert_eq!(layouts.layout(raw_layout).size, layouts.layout(layout).size);
}

/// Preserve identified representations and recursive applications across trees.
#[test]
fn test_import_recursive_declarations() {
    let source = "\
type Opaque<T>;

type Handle = newtype<Opaque<int32>>;

type Node<T> {
    next: ref<Node<T>, managed, mutable, local>;
    value: T;
}

type Root = newtype<Node<int32>>;

type Other = newtype<Root>;

type Grow<T> {
    next: ref<Grow<[T; 2]>, managed, mutable, local>;
    value: T;
}

type Grown = newtype<Grow<int32>>;

type Borrowed = newtype<Grow<ref<int32, borrowed, 'static & local, readonly>>>;

type Empty<T> = newtype<int32>;

type Retained = newtype<Empty<ref<int32, borrowed, 'static & local, readonly>>>;

type Wrap<T> {
    value: T;
}

type Nested = newtype<Wrap<Wrap<int32>>>;

type Identity<T> = newtype<T>;

type Direct = newtype<Identity<int32>>;

type Indirect = newtype<Identity<Wrap<int32>>>;";
    let file = test_file(source);
    let (source_tree, strings) = Parser::parse(&file, ParseOptions::default())
        .unwrap()
        .finish()
        .unwrap();

    // shift the destination's node ids and import every named declaration
    let mut destination = Tree::new();
    destination.intern_type(Type::Boolean, Copy::Yes);
    let mut declared = |_| None;
    let mut importer = Importer::new(&mut destination, &mut declared);
    for (_, declaration) in source_tree.iter_nodes::<TypeDeclaration>() {
        let ty = source_tree.identified_type(declaration.symbol).unwrap();
        importer.import_type(&source_tree, ty);
    }

    // preserve the complete declaration text without expanding applications
    assert_eq!(format_test_mir(&destination, &strings), source);

    // inspect lifetime arguments without expanding recursively growing reference targets
    for (name, lifetime, contains) in [
        ("Grown", None, false),
        ("Borrowed", Some(Lifetime::new([Extent::Static])), true),
        ("Retained", Some(Lifetime::new([Extent::Static])), true),
    ] {
        let declaration = destination
            .iter_nodes::<TypeDeclaration>()
            .map(|(_, declaration)| declaration)
            .find(|declaration| declaration.name.is_some_and(|id| strings.get(id) == name))
            .unwrap();
        let ty = destination.identified_type(declaration.symbol).unwrap();
        assert_eq!(type_lifetime(&destination, ty), lifetime);
        assert_eq!(type_contains_borrowed_refs(&destination, ty), contains);
        if name == "Retained" {
            assert_eq!(type_borrowed_paths(&destination, ty), Vec::new());
        }
    }

    // preserve the arguments of opaque applications during representation queries
    let handle = destination
        .iter_nodes::<TypeDeclaration>()
        .map(|(_, declaration)| declaration)
        .find(|declaration| {
            declaration
                .name
                .is_some_and(|id| strings.get(id) == "Handle")
        })
        .unwrap();
    let Type::Newtype { inner, .. } = *destination.get(handle.definition.unwrap()) else {
        panic!("expected the declared newtype");
    };
    assert_eq!(Substitution::resolve(inner, &destination), inner);

    // lay out the finite values while keeping recursive reference targets symbolic
    let mut layouts = LayoutTable::new();
    for (name, size) in [("Grown", 16), ("Nested", 4), ("Direct", 4), ("Indirect", 4)] {
        let declaration = destination
            .iter_nodes::<TypeDeclaration>()
            .map(|(_, declaration)| declaration)
            .find(|declaration| declaration.name.is_some_and(|id| strings.get(id) == name))
            .unwrap();
        let ty = destination.identified_type(declaration.symbol).unwrap();
        let layout = LayoutBuilder::new(&destination, &mut layouts, TargetLayout::default())
            .layout_type(ty)
            .unwrap();
        assert_eq!(layouts.layout(layout).size, size);
    }
}

/// Lay out projected storage while leaving unused and opaque pointees unlaid out.
#[test]
fn test_layout_projected_storage() {
    let file = test_file(
        r#"
type Opaque;
type Unused { value: int64; }
type Record { first: int32; second: int64; }
type Element { first: int64; second: int64; }
type Choice = variant<uint1> { 0uint1 = int32; 1uint1 = boolean; };

function project(
    v0: ptr<Opaque, readonly>,
    v1: slice<Unused, borrowed, 'static, readonly, local>,
    v2: ptr<Record, readonly>,
    v3: slice<Element, borrowed, 'static, readonly, local>,
    v4: ptr<Choice, readonly>,
    v5: uint64
): void {
entry:
    v6: ptr<int64, readonly> = address (*v2).1
    v7: slice<Element, borrowed, 'static, readonly, local> = address (*v3)[v5; v5]
    v8: ref<Element, borrowed, 'static, readonly, local> = address (*v3)[v5]
    v9: uint1 = variant.tag.load (*v4)
    return
}
"#,
    );
    let (tree, strings) = Parser::parse(&file, ParseOptions::default())
        .unwrap()
        .finish()
        .unwrap();
    let mut layouts = LayoutTable::new();
    LayoutBuilder::new(&tree, &mut layouts, TargetLayout::default())
        .layout_reachable_types()
        .unwrap();

    // compare every declared type's layout requirement and size
    let sizes: Vec<_> = tree
        .iter_nodes::<TypeDeclaration>()
        .filter_map(|(_, declaration)| {
            let name = declaration.name?;
            let ty = tree.identified_type(declaration.symbol).unwrap();

            Some((
                strings.get(name),
                layouts.type_layout(ty).map(|layout| layout.size),
            ))
        })
        .collect();
    assert_eq!(
        sizes,
        [
            ("Opaque", None),
            ("Unused", Some(8)),
            ("Record", Some(16)),
            ("Element", Some(16)),
            ("Choice", Some(8)),
        ]
    );
}
