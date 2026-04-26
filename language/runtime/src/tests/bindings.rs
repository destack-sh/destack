use destack_engine::Value;
use destack_mir::ModuleBuilder;
use destack_vm::Isolate;

use crate::diagnostic::RuntimeError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::tests::runtime::TestRuntime;

/// Build a minimal MIR module that calls the given random binding.
fn build_random_call_module(
    extern_name: &str,
    stream_arg: Option<u64>,
) -> (destack_mir::NodeTree, destack_core::ImmutableStringPool) {
    // core types
    let mut module = ModuleBuilder::checked();
    let u64_type = module.type_u64();

    // extern function signature
    let param_types = stream_arg.map(|_| vec![u64_type]).unwrap_or_default();
    let extern_name = format!("destack.{extern_name}");
    let extern_id = module.extern_function(&extern_name, &param_types, u64_type);
    let signature = module.type_function_signature(param_types.clone(), u64_type);

    // entry function
    let mut builder = module.function("main", &[], u64_type);
    let entry_block = builder.block();
    builder.switch_to_block(entry_block);

    // arguments
    let arguments = if let Some(stream) = stream_arg {
        vec![builder.iconst(stream as i64, 64, false)]
    } else {
        Vec::new()
    };

    // call external binding and return
    let value = builder
        .call(extern_id, signature, arguments)
        .expect("external call returns a value");
    builder.return_(Some(value));
    builder.finish();

    module.finish_immutable()
}

/// Execute the VM random binding via an external call.
fn run_vm_random_call(
    runtime: &mut TestRuntime,
    extern_name: &str,
    stream_arg: Option<u64>,
) -> u64 {
    // build the MIR module
    let (tree, strings) = build_random_call_module(extern_name, stream_arg);

    // runtime and isolate setup
    let mut isolate =
        Isolate::build(destack_vm::IsolateId::new(1), tree, strings).expect("isolate init");
    let mut statics = destack_vm::StaticSpace::empty();
    let mut heap = destack_vm::Heap::with_allocator_limits_and_options(
        std::sync::Arc::new(
            destack_vm::Allocator::try_new(
                destack_vm::HeapOptions::local().page_bytes,
                destack_vm::HeapOptions::local().allocator_arena_bytes,
            )
            .expect("test vm allocator should build"),
        ),
        destack_vm::HeapLimits::default(),
        destack_vm::HeapOptions::local(),
    )
    .expect("test vm heap should build");
    let shared = destack_vm::SharedHeap::with_allocator_limits_and_options(
        std::sync::Arc::new(
            destack_vm::Allocator::try_new(
                destack_vm::HeapOptions::shared().page_bytes,
                destack_vm::HeapOptions::shared().allocator_arena_bytes,
            )
            .expect("test shared allocator should build"),
        ),
        destack_vm::SharedHeapLimits::default(),
        destack_vm::HeapOptions::shared(),
    )
    .expect("test vm shared heap should build");
    runtime.install_vm_defaults(&mut isolate);
    isolate
        .initialize_statics(&mut statics)
        .expect("isolate statics should initialize");

    // execute entry function
    let output = runtime
        .with_native_call_context(|_| {
            isolate.run_function_by_name(&mut statics, &mut heap, &shared, "main", &[])
        })
        .expect("vm execution");
    let Value::UInt { value, width } = output.value else {
        panic!("u64 result")
    };
    assert_eq!(width, 64);

    value
}

#[test]
fn test_random_next_u64_matches_vm_and_native() {
    // native and VM bindings should produce the same deterministic value
    let mut native_runtime = TestRuntime::deterministic_random();
    let native_value = match native_runtime.call_native_next_u64() {
        Ok(value) => value,
        Err(error) if is_not_supported_error(&error) => return,
        Err(error) => panic!("native random nextU64 failed: {}", error.message()),
    };

    let mut vm_runtime = TestRuntime::deterministic_random();
    let vm_value = run_vm_random_call(&mut vm_runtime, "random.stream.nextU64", None);
    assert_eq!(native_value, vm_value);
}

#[test]
fn test_random_next_u64_from_matches_vm_and_native() {
    // stream based random calls should match for the same stream id
    let mut native_runtime = TestRuntime::deterministic_random();
    let native_value = match native_runtime.call_native_next_u64_from(0) {
        Ok(value) => value,
        Err(error) if is_not_supported_error(&error) => return,
        Err(error) => panic!("native random nextU64From failed: {}", error.message()),
    };

    let mut vm_runtime = TestRuntime::deterministic_random();
    let vm_value = run_vm_random_call(&mut vm_runtime, "random.stream.nextU64From", Some(0));
    assert_eq!(native_value, vm_value);
}

/// Return whether a runtime error maps to a not-supported platform error.
fn is_not_supported_error(error: &RuntimeError) -> bool {
    error
        .platform_error()
        .is_some_and(|platform| platform.code == PlatformErrorCode::NotSupported)
}
