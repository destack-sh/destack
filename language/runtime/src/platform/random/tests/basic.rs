use super::with_harness_context;

#[cfg(any(unix, windows))]
#[test]
fn test_random_basic_scaffold() {
    with_harness_context(|context| {
        let _ = context.call_context;
        let _ = context.vm_context;
        Ok(())
    });
}
