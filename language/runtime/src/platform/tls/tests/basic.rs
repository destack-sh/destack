use super::with_harness_context;

/// Open and close one TLS context through both harness backends.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_open_close_roundtrip() {
    with_harness_context(|mut context| {
        let options = context.default_client_context_options()?;
        let handle = context.destack_tls_context_open(options)?;
        context.destack_tls_context_close(handle)
    });
}
