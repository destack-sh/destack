use crate::{Constant, GlobalInitializer, MirFormatOptions, ModuleBuilder, Mutability, format_mir};

/// Simple add function with two parameters.
#[test]
fn test_format_simple_add() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let void_type = module.type_void();

    // build function
    let mut builder = module.function("add", &[i32_type, i32_type], void_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let left_value = builder.function_parameter(0);
    let right_value = builder.function_parameter(1);
    let sum_value = builder.iadd(left_value, right_value);
    builder.return_(Some(sum_value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @add(v0: i32, v1: i32) -> void {
block0:
    v2 = iadd v0, v1
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
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
function @with_locals() -> i64 {
    local0: i64 ; owned, var
block0:
    v0 = iconst 42i64
    local_set local0, v0
    v1 = local_get local0
    return v1
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
    let (tree, strings) = module.finish();
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

/// Module with global variable.
#[test]
fn test_format_global_variable() {
    // setup
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let void_type = module.type_void();

    // create a mutable global
    let counter = module.global_variable("counter", i32_type, GlobalInitializer::zero());

    // build function that increments the global
    let mut builder = module.function("increment", &[], void_type);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    let value = builder.global_get(counter);
    let one = builder.iconst_i32(1);
    let new_value = builder.iadd(value, one);
    builder.global_set(counter, new_value);
    builder.return_(None);
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
global @counter: i32 = zeroinit ; var
function @increment() -> void {
block0:
    v0 = global_get @counter
    v1 = iconst 1i32
    v2 = iadd v0, v1
    global_set @counter, v2
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
    let value = builder.global_get(magic);
    builder.return_(Some(value));
    builder.seal_block(entry_block);
    builder.finish();

    // verify formatted output
    let (tree, strings) = module.finish();
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let expected = "\
global @MAGIC: i64 = 42i64 ; const
function @get_magic() -> i64 {
block0:
    v0 = global_get @MAGIC
    return v0
}";
    assert_eq!(output, expected);
}
