use destack_mir::ModuleBuilder;

use crate::CraneliftCodegenBackend;

/// Test integer arithmetic operations.
#[test]
fn test_integer_arithmetic() {
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    let mut builder = module.function("arithmetic", &[i32_type, i32_type], i32_type);
    let entry = builder.create_block();
    builder.switch_to_block(entry);

    let a = builder.function_parameter(0);
    let b = builder.function_parameter(1);

    // chain of operations: ((a + b) - a) * b
    let add_result = builder.iadd(a, b);
    let sub_result = builder.isub(add_result, a);
    let mul_result = builder.imul(sub_result, b);

    builder.return_(Some(mul_result));
    builder.seal_block(entry);
    builder.finish();

    let (tree, strings) = module.finish();

    let backend = CraneliftCodegenBackend::native().expect("failed to create backend");
    let clif = backend
        .compile_to_clif(&tree, &strings)
        .expect("failed to compile");

    assert!(clif.contains("iadd"));
    assert!(clif.contains("isub"));
    assert!(clif.contains("imul"));
}

/// Test integer comparison operations.
#[test]
fn test_integer_comparison() {
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();
    let bool_type = module.type_bool();

    let mut builder = module.function("compare", &[i32_type, i32_type], bool_type);
    let entry = builder.create_block();
    builder.switch_to_block(entry);

    let a = builder.function_parameter(0);
    let b = builder.function_parameter(1);

    let cmp_result = builder.icmp_slt(a, b);
    builder.return_(Some(cmp_result));

    builder.seal_block(entry);
    builder.finish();

    let (tree, strings) = module.finish();

    let backend = CraneliftCodegenBackend::native().expect("failed to create backend");
    let clif = backend
        .compile_to_clif(&tree, &strings)
        .expect("failed to compile");

    assert!(clif.contains("icmp"));
}

/// Test bitwise operations.
#[test]
fn test_bitwise_operations() {
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    let mut builder = module.function("bitwise", &[i32_type, i32_type], i32_type);
    let entry = builder.create_block();
    builder.switch_to_block(entry);

    let a = builder.function_parameter(0);
    let b = builder.function_parameter(1);

    let and_result = builder.band(a, b);
    let or_result = builder.bor(and_result, a);
    let xor_result = builder.bxor(or_result, b);

    builder.return_(Some(xor_result));
    builder.seal_block(entry);
    builder.finish();

    let (tree, strings) = module.finish();

    let backend = CraneliftCodegenBackend::native().expect("failed to create backend");
    let clif = backend
        .compile_to_clif(&tree, &strings)
        .expect("failed to compile");

    assert!(clif.contains("band"));
    assert!(clif.contains("bor"));
    assert!(clif.contains("bxor"));
}

/// Test unary operations.
#[test]
fn test_unary_operations() {
    let mut module = ModuleBuilder::new();
    let i32_type = module.type_i32();

    let mut builder = module.function("negate", &[i32_type], i32_type);
    let entry = builder.create_block();
    builder.switch_to_block(entry);

    let a = builder.function_parameter(0);
    let neg_result = builder.ineg(a);

    builder.return_(Some(neg_result));
    builder.seal_block(entry);
    builder.finish();

    let (tree, strings) = module.finish();

    let backend = CraneliftCodegenBackend::native().expect("failed to create backend");
    let clif = backend
        .compile_to_clif(&tree, &strings)
        .expect("failed to compile");

    assert!(clif.contains("ineg"));
}
