use destack_mir::ModuleBuilder;

use crate::CraneliftCodegenBackend;

/// Test lowering an empty void function.
#[test]
fn test_lower_empty_function() {
    let mut module = ModuleBuilder::new();
    let void_type = module.type_void();

    let mut builder = module.function("empty", &[], void_type);
    let entry = builder.create_block();
    builder.switch_to_block(entry);
    builder.return_(None);
    builder.seal_block(entry);
    builder.finish();

    let (tree, strings) = module.finish();

    let backend = CraneliftCodegenBackend::native().expect("failed to create backend");
    let clif = backend
        .compile_to_clif(&tree, &strings)
        .expect("failed to compile");

    // verify it contains the function signature
    assert!(clif.contains("function u0:0"));
}

/// Test lowering a function with i32 parameters.
#[test]
fn test_lower_function_with_params() {
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    let mut builder = module.function("add", &[i32_type, i32_type], i32_type);
    let entry = builder.create_block();
    builder.switch_to_block(entry);

    let a = builder.function_parameter(0);
    let b = builder.function_parameter(1);
    let sum = builder.iadd(a, b);
    builder.return_(Some(sum));

    builder.seal_block(entry);
    builder.finish();

    let (tree, strings) = module.finish();

    let backend = CraneliftCodegenBackend::native().expect("failed to create backend");
    let clif = backend
        .compile_to_clif(&tree, &strings)
        .expect("failed to compile");

    // verify it has the iadd instruction
    assert!(clif.contains("iadd"));
}
