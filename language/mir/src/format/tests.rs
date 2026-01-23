use crate::{
    AddressSpace, BinaryOperator, Constant, Copyability, ExecutionModel, ExecutionStage,
    GlobalInitializer, MirFormatOptions, ModuleBuilder, Mutability, ReferenceKind,
    TensorConvertMode, TensorDimension, TensorLayout, VectorConvertMode, format_mir,
};

/// Simple add function with two parameters.
#[test]
fn test_format_simple_add() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    // build function
    let mut builder = module.function("add", &[i32_type, i32_type], i32_type);
    let entry_block = builder.create_block();
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
function @add(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: i32 = iadd v0, v1
    return v2
}";
    assert_eq!(output, expected);
}

/// Function with local variables.
#[test]
fn test_format_with_locals() {
    // setup
    let mut module = ModuleBuilder::new();
    let i64_type = module.type_i64();

    // build function with local
    let mut builder = module.function("with_locals", &[], i64_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let local = builder.create_local(i64_type, Mutability::Mutable);
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
function @with_locals() -> i64 {
    local0: i64 ; owned, mut
block0:
    v0: i64 = iconst 42i64
    local.set local0, v0
    v1: i64 = local.get local0
    return v1
}";
    assert_eq!(output, expected);
}

/// Function with a local address.
#[test]
fn test_format_local_addr() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let void_type = module.type_void();

    // build function with local address
    let mut builder = module.function("local_addr", &[], void_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let local = builder.create_local(i32_type, Mutability::Mutable);
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
function @local_addr() -> void {
    local0: i32 ; owned, mut
block0:
    v0: ref<borrowed addrspace(stack) mut i32> = local.addr local0
    return
}";
    assert_eq!(output, expected);
}

/// Format vector compare and convert operations.
#[test]
fn test_format_vector_compare_and_convert() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let bool_type = module.type_bool();
    let f32_type = module.type_f32();
    let vector_i32_type = module.type_vector(i32_type, 4, Copyability::Trivial);
    let vector_bool_type = module.type_vector(bool_type, 4, Copyability::Trivial);
    let vector_f32_type = module.type_vector(f32_type, 4, Copyability::Trivial);

    // build function
    let void_type = module.type_void();
    let mut builder = module.function("vector_ops", &[], void_type);
    let entry_block = builder.create_block();
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
    let _converted =
        builder.vector_convert(vector_f32_type, VectorConvertMode::Exact, left_vector);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @vector_ops() -> void {
block0:
    v0: i32 = iconst 1i32
    v1: vector<i32, 4> = vector.splat v0
    v2: vector<i32, 4> = vector.splat v0
    v3: vector<bool, 4> = vector.compare icmp_eq, v1, v2
    v4: vector<f32, 4> = vector.convert exact, v1
    return
}";
    assert_eq!(output, expected);
}

/// Format tensor compare and convert operations.
#[test]
fn test_format_tensor_compare_and_convert() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let bool_type = module.type_bool();
    let f32_type = module.type_f32();
    let shape = vec![TensorDimension::Static(2), TensorDimension::Static(2)];
    let tensor_i32_type =
        module.type_tensor(i32_type, shape.clone(), TensorLayout::RowMajor, Copyability::Trivial);
    let tensor_bool_type =
        module.type_tensor(bool_type, shape.clone(), TensorLayout::RowMajor, Copyability::Trivial);
    let tensor_f32_type =
        module.type_tensor(f32_type, shape, TensorLayout::RowMajor, Copyability::Trivial);

    // build function
    let void_type = module.type_void();
    let mut builder = module.function("tensor_ops", &[tensor_i32_type, tensor_i32_type], void_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let left_tensor = builder.function_parameter(0);
    let right_tensor = builder.function_parameter(1);
    let _comparison = builder.tensor_compare(
        tensor_bool_type,
        BinaryOperator::Equal,
        left_tensor,
        right_tensor,
    );
    let _converted =
        builder.tensor_convert(tensor_f32_type, TensorConvertMode::Exact, left_tensor);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @tensor_ops(v0: tensor<i32, [2, 2]>, v1: tensor<i32, [2, 2]>) -> void {
block0(v0: tensor<i32, [2, 2]>, v1: tensor<i32, [2, 2]>):
    v2: tensor<bool, [2, 2]> = tensor.compare icmp_eq, v0, v1
    v3: tensor<f32, [2, 2]> = tensor.convert exact, v0
    return
}";
    assert_eq!(output, expected);
}

/// Function with kernel metadata.
#[test]
fn test_format_function_metadata() {
    // setup
    let mut module = ModuleBuilder::new();
    let void_type = module.type_void();

    // build kernel function
    let mut builder = module.function("kernel", &[], void_type);
    builder.set_execution_model(ExecutionModel::Kernel);
    builder.set_workgroup_size([8, 1, 1]);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
#[execution_model(kernel)]
#[workgroup_size(8, 1, 1)]
function @kernel() -> void {
block0:
    return
}";
    assert_eq!(output, expected);
}

/// Function with graphics stage metadata.
#[test]
fn test_format_function_stage_metadata() {
    // setup
    let mut module = ModuleBuilder::new();
    let void_type = module.type_void();

    // build graphics entry point
    let mut builder = module.function("vertex_main", &[], void_type);
    builder.set_execution_model(ExecutionModel::Graphics);
    builder.set_execution_stage(ExecutionStage::Vertex);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
#[execution_model(graphics)]
#[execution_stage(vertex)]
function @vertex_main() -> void {
block0:
    return
}";
    assert_eq!(output, expected);
}

/// Function with conditional branch.
#[test]
fn test_format_branch() {
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

    // merge block: return the then value (simplified test)
    builder.switch_to_block(merge_block);
    builder.return_(Some(one_value));
    builder.seal_block(merge_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @select(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1: i32 = iconst 1i32
    jump block3
block2:
    v2: i32 = iconst 0i32
    jump block3
block3:
    return v1
}";
    assert_eq!(output, expected);
}

/// Function with void return.
#[test]
fn test_format_void_return() {
    // setup
    let mut module = ModuleBuilder::new();
    let void_type = module.type_void();

    // build empty function
    let mut builder = module.function("noop", &[], void_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @noop() -> void {
block0:
    return
}";
    assert_eq!(output, expected);
}

/// SSA construction with variables.
#[test]
fn test_format_ssa_variable() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    // build function using SSA variable
    let mut builder = module.function("var_test", &[], i32_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);

    // define and use variable
    let variable = builder.create_variable(i32_type);
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
function @var_test() -> i32 {
block0:
    v0: i32 = iconst 10i32
    return v0
}";
    assert_eq!(output, expected);
}

/// Module with mutable global variable.
#[test]
fn test_format_global_variable() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let void_type = module.type_void();

    // create a mutable global
    let counter = module.global_variable("counter", i32_type, GlobalInitializer::zero());

    // build function that increments the global via pointer
    let mut builder = module.function("increment", &[], void_type);
    let entry_block = builder.create_block();
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
global @counter: i32 = zeroinit ; mut
function @increment() -> void {
block0:
    v0: ref<raw addrspace(global) mut i32> = global.addr @counter
    v1: i32 = load v0
    v2: i32 = iconst 1i32
    v3: i32 = iadd v1, v2
    store v0, v3
    return
}";
    assert_eq!(output, expected);
}

/// Module with immutable global constant.
#[test]
fn test_format_global_constant() {
    // setup
    let mut module = ModuleBuilder::new();
    let i64_type = module.type_i64();

    // create an immutable global with scalar init
    let init = Constant::Int {
        value: 42,
        width: 64,
        is_signed: true,
    };
    let magic = module.global_constant("MAGIC", i64_type, GlobalInitializer::scalar(init));

    // build function that reads the constant
    let mut builder = module.function("get_magic", &[], i64_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let value = builder.global_const(magic);
    builder.return_(Some(value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish_immutable();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
global @MAGIC: i64 = 42i64 ; const
function @get_magic() -> i64 {
block0:
    v0: i64 = global.const @MAGIC
    return v0
}";
    assert_eq!(output, expected);
}
