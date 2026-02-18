use super::with_harness_context;

/// Keep the io harness scaffold available for shared test setup.
#[cfg(any(unix, windows))]
#[test]
fn test_io_basic_scaffold() {
    with_harness_context(|context| {
        let _ = context.call_context;
        let _ = context.vm_context;

        Ok(())
    });
}
