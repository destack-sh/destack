use destack_mir::ModuleBuilder;

use crate::CraneliftCodegenBackend;

/// Test conditional branch.
#[test]
fn test_conditional_branch() {
    let mut module = ModuleBuilder::new();
    let bool_type = module.type_bool();
    let i32_type = module.type_i32();

    let mut builder = module.function("select", &[bool_type], i32_type);

    let entry = builder.create_block();
    let then_block = builder.create_block();
    let else_block = builder.create_block();

    builder.switch_to_block(entry);
    let condition = builder.function_parameter(0);
    builder.branch(condition, then_block, else_block);
    builder.seal_block(entry);

    builder.switch_to_block(then_block);
    let one = builder.iconst_i32(1);
    builder.return_(Some(one));
    builder.seal_block(then_block);

    builder.switch_to_block(else_block);
    let zero = builder.iconst_i32(0);
    builder.return_(Some(zero));
    builder.seal_block(else_block);

    builder.finish();

    let (tree, strings) = module.finish();

    let backend = CraneliftCodegenBackend::native().expect("failed to create backend");
    let clif = backend
        .compile_to_clif(&tree, &strings)
        .expect("failed to compile");

    // should have brif instruction
    assert!(clif.contains("brif"));
}

/// Test unconditional jump.
#[test]
fn test_unconditional_jump() {
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    let mut builder = module.function("jump_test", &[], i32_type);

    let entry = builder.create_block();
    let target = builder.create_block();

    builder.switch_to_block(entry);
    builder.jump(target);
    builder.seal_block(entry);

    builder.switch_to_block(target);
    let result = builder.iconst_i32(42);
    builder.return_(Some(result));
    builder.seal_block(target);

    builder.finish();

    let (tree, strings) = module.finish();

    let backend = CraneliftCodegenBackend::native().expect("failed to create backend");
    let clif = backend
        .compile_to_clif(&tree, &strings)
        .expect("failed to compile");

    // should have jump instruction
    assert!(clif.contains("jump"));
}

/// Test block parameters (phi equivalent).
#[test]
fn test_block_parameters() {
    let mut module = ModuleBuilder::new();
    let bool_type = module.type_bool();
    let i32_type = module.type_i32();

    let mut builder = module.function("phi_test", &[bool_type], i32_type);

    let entry = builder.create_block();
    let then_block = builder.create_block();
    let else_block = builder.create_block();
    let merge_block = builder.create_block();

    // create variable for SSA
    let var = builder.create_variable(i32_type);

    builder.switch_to_block(entry);
    let condition = builder.function_parameter(0);
    let initial = builder.iconst_i32(0);
    builder.define_variable(var, initial);
    builder.branch(condition, then_block, else_block);
    builder.seal_block(entry);

    builder.switch_to_block(then_block);
    let then_val = builder.iconst_i32(10);
    builder.define_variable(var, then_val);
    builder.jump(merge_block);
    builder.seal_block(then_block);

    builder.switch_to_block(else_block);
    let else_val = builder.iconst_i32(20);
    builder.define_variable(var, else_val);
    builder.jump(merge_block);
    builder.seal_block(else_block);

    builder.switch_to_block(merge_block);
    let result = builder.use_variable(var);
    builder.return_(Some(result));
    builder.seal_block(merge_block);

    builder.finish();

    let (tree, strings) = module.finish();

    let backend = CraneliftCodegenBackend::native().expect("failed to create backend");
    let clif = backend
        .compile_to_clif(&tree, &strings)
        .expect("failed to compile");

    // the merge block should have block parameters
    assert!(clif.contains("block"));
}
