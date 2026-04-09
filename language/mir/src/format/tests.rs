use crate::{
    AddressSpace, BinaryOperator, Constant, Copyability, ExecutionModel, ExecutionStage,
    GlobalInitializer, MirFormatOptions, ModuleBuilder, Mutability, ReferenceKind,
    TensorConvertMode, TensorDimension, TensorLayout, VectorConvertMode, format_mir,
};

/// Simple add function with two parameters.
#[test]
fn test_format_simple_add() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("add", &[i32_type, i32_type], i32_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let left_value = builder.function_parameter(0);
    let right_value = builder.function_parameter(1);
    let sum_value = builder.iadd(left_value, right_value);
    builder.return_(Some(sum_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function add(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}";
    assert_eq!(output, expected);
}

/// Function with local variables.
#[test]
fn test_format_with_locals() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i64_type = module.type_i64();

    // build function with local
    let mut builder = module.function("withLocals", &[], i64_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let local = builder.local(i64_type, Mutability::Mutable);
    let constant_value = builder.iconst_i64(42);
    builder.local_set(local, constant_value);
    let loaded_value = builder.local_get(local);
    builder.return_(Some(loaded_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function withLocals(): int64 {
    local local0: int64, owned

b0:
    v0: int64 = 42int64
    local.set local0, v0
    v1: int64 = local.get local0
    return v1
}";
    assert_eq!(output, expected);
}

/// Function with a local address.
#[test]
fn test_format_local_addr() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let void_type = module.type_void();

    // build function with local address
    let mut builder = module.function("localAddr", &[], void_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let local = builder.local(i32_type, Mutability::Mutable);
    let ref_type = builder.type_reference(
        ReferenceKind::Borrowed,
        i32_type,
        Mutability::Mutable,
        AddressSpace::Stack,
        false,
    );
    let _addr = builder.local_addr(local, ref_type);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function localAddr(): void {
    local local0: int32, owned

b0:
    v0: ref<int32, borrowed, addressSpace(stack)> = local.address local0
    return
}";
    assert_eq!(output, expected);
}

/// Format vector compare and convert operations.
#[test]
fn test_format_vector_compare_and_convert() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let bool_type = module.type_boolean();
    let f32_type = module.type_f32();
    let vector_i32_type = module.type_vector(i32_type, 4, Copyability::Trivial);
    let vector_bool_type = module.type_vector(bool_type, 4, Copyability::Trivial);
    let vector_f32_type = module.type_vector(f32_type, 4, Copyability::Trivial);

    // build function
    let void_type = module.type_void();
    let mut builder = module.function("vectorOps", &[], void_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let scalar_value = builder.iconst_i32(1);
    let left_vector = builder.vector_splat(vector_i32_type, scalar_value);
    let right_vector = builder.vector_splat(vector_i32_type, scalar_value);
    let _comparison = builder.vector_compare(
        vector_bool_type,
        BinaryOperator::Equal,
        left_vector,
        right_vector,
    );
    let _converted = builder.vector_convert(vector_f32_type, VectorConvertMode::Exact, left_vector);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function vectorOps(): void {
b0:
    v0: int32 = 1int32
    v1: vector<int32, 4> = vector.splat v0
    v2: vector<int32, 4> = vector.splat v0
    v3: vector<boolean, 4> = vector.compare int.eq, v1, v2
    v4: vector<float32, 4> = vector.convert exact, v1
    return
}";
    assert_eq!(output, expected);
}

/// Format tensor compare and convert operations.
#[test]
fn test_format_tensor_compare_and_convert() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let bool_type = module.type_boolean();
    let f32_type = module.type_f32();
    let shape = vec![TensorDimension::Static(2), TensorDimension::Static(2)];
    let tensor_i32_type = module.type_tensor(
        i32_type,
        shape.clone(),
        TensorLayout::RowMajor,
        Copyability::Trivial,
    );
    let tensor_bool_type = module.type_tensor(
        bool_type,
        shape.clone(),
        TensorLayout::RowMajor,
        Copyability::Trivial,
    );
    let tensor_f32_type = module.type_tensor(
        f32_type,
        shape,
        TensorLayout::RowMajor,
        Copyability::Trivial,
    );

    // build function
    let void_type = module.type_void();
    let mut builder = module.function("tensorOps", &[tensor_i32_type, tensor_i32_type], void_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let left_tensor = builder.function_parameter(0);
    let right_tensor = builder.function_parameter(1);
    let _comparison = builder.tensor_compare(
        tensor_bool_type,
        BinaryOperator::Equal,
        left_tensor,
        right_tensor,
    );
    let _converted = builder.tensor_convert(tensor_f32_type, TensorConvertMode::Exact, left_tensor);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function tensorOps(v0: tensor<int32, (2, 2)>, v1: tensor<int32, (2, 2)>): void {
b0(v0: tensor<int32, (2, 2)>, v1: tensor<int32, (2, 2)>):
    v2: tensor<boolean, (2, 2)> = tensor.compare int.eq, v0, v1
    v3: tensor<float32, (2, 2)> = tensor.convert exact, v0
    return
}";
    assert_eq!(output, expected);
}

/// Function with kernel metadata.
#[test]
fn test_format_function_metadata() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let void_type = module.type_void();

    // build kernel function
    let mut builder = module.function("kernel", &[], void_type);
    builder.set_execution_model(ExecutionModel::Kernel);
    builder.set_workgroup_size([8, 1, 1]);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
@executionModel(kernel)
@workgroupSize(8, 1, 1)
function kernel(): void {
b0:
    return
}";
    assert_eq!(output, expected);
}

/// Function with graphics stage metadata.
#[test]
fn test_format_function_stage_metadata() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let void_type = module.type_void();

    // build graphics entry point
    let mut builder = module.function("vertexMain", &[], void_type);
    builder.set_execution_model(ExecutionModel::Graphics);
    builder.set_execution_stage(ExecutionStage::Vertex);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
@executionModel(graphics)
@executionStage(vertex)
function vertexMain(): void {
b0:
    return
}";
    assert_eq!(output, expected);
}

/// Function with conditional branch.
#[test]
fn test_format_branch() {
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

    // merge block: return the then value (simplified test)
    builder.switch_to_block(merge_block);
    builder.return_(Some(one_value));
    builder.seal_block(merge_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function select(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2

b1:
    v1: int32 = 1int32
    jump b3

b2:
    v2: int32 = 0int32
    jump b3

b3:
    return v1
}";
    assert_eq!(output, expected);
}

/// Function with void return.
#[test]
fn test_format_void_return() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let void_type = module.type_void();

    // build empty function
    let mut builder = module.function("noop", &[], void_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function noop(): void {
b0:
    return
}";
    assert_eq!(output, expected);
}

/// SSA construction with variables.
#[test]
fn test_format_ssa_variable() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();

    // build function using SSA variable
    let mut builder = module.function("varTest", &[], i32_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    // define and use variable
    let variable = builder.variable(i32_type);
    let constant_value = builder.iconst_i32(10);
    builder.define_variable(variable, constant_value);
    let used_value = builder.use_variable(variable);
    builder.return_(Some(used_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function varTest(): int32 {
b0:
    v0: int32 = 10int32
    return v0
}";
    assert_eq!(output, expected);
}

/// Module with mutable global variable.
#[test]
fn test_format_global_variable() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let void_type = module.type_void();

    // create a mutable global
    let counter = module.global_variable("counter", i32_type, GlobalInitializer::zero());

    // build function that increments the global via pointer
    let mut builder = module.function("increment", &[], void_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let ptr_type = builder.type_reference(
        ReferenceKind::Raw,
        i32_type,
        Mutability::Mutable,
        AddressSpace::Global,
        false,
    );
    let ptr = builder.global_addr(counter, ptr_type);
    let value = builder.load(ptr, i32_type);
    let one = builder.iconst_i32(1);
    let new_value = builder.iadd(value, one);
    builder.store(ptr, new_value);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
global counter: int32 = zeroInit

function increment(): void {
b0:
    v0: ref<int32, raw, addressSpace(global)> = global.address counter
    v1: int32 = load v0
    v2: int32 = 1int32
    v3: int32 = int.add v1, v2
    store v0, v3
    return
}";
    assert_eq!(output, expected);
}

/// Module with immutable global constant.
#[test]
fn test_format_global_constant() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i64_type = module.type_i64();

    // create an immutable global with scalar init
    let init = Constant::Int {
        value: 42,
        width: 64,
        is_signed: true,
    };
    let magic = module.global_constant("MAGIC", i64_type, GlobalInitializer::scalar(init));

    // build function that reads the constant
    let mut builder = module.function("getMagic", &[], i64_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let value = builder.global_const(magic);
    builder.return_(Some(value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
global MAGIC: int64, readonly = 42int64

function getMagic(): int64 {
b0:
    v0: int64 = global.const MAGIC
    return v0
}";
    assert_eq!(output, expected);
}

/// Managed and owned references preserve mutable vs readonly spelling.
#[test]
fn test_format_reference_mutability_preserved() {
    // setup
    let mut module = ModuleBuilder::unchecked();
    let i32_type = module.type_i32();
    let managed_mutable_type = module.type_managed_reference_mutable(i32_type);
    let owned_readonly_type = module.type_owned_reference_readonly(i32_type);

    // build function that returns the mutable managed parameter
    let mut builder = module.function(
        "refMutability",
        &[managed_mutable_type, owned_readonly_type],
        managed_mutable_type,
    );
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);
    let value = builder.function_parameter(0);
    builder.return_(Some(value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function refMutability(v0: ref<int32, managed>, v1: ref<int32, owned, readonly>): ref<int32, managed> {
b0(v0: ref<int32, managed>, v1: ref<int32, owned, readonly>):
    return v0
}";
    assert_eq!(output, expected);
}
