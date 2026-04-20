use crate::{HeapError, HeapScan, ManagedReference, trace_managed_references};

/// Trace compact managed references at fixed byte offsets.
#[test]
fn test_reference_offsets_trace_reference32_payloads() {
    let first = ManagedReference::new(0x0102_0304);
    let second = ManagedReference::new(0x1122_3344);
    let bytes = [
        0xAA, 0xBB, 0xCC, 0xDD, 0x04, 0x03, 0x02, 0x01, 0x44, 0x33, 0x22, 0x11,
    ];
    let trace = HeapScan::Reference {
        local_offsets: vec![4, 8].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let mut traced = Vec::new();

    // follow compact references without assuming u64 payload words
    trace_managed_references(&trace, &bytes, 4, |reference| traced.push(reference))
        .expect("reference tracing should succeed");

    assert_eq!(traced, vec![first, second]);
}

/// Trace compact managed references across repeated elements.
#[test]
fn test_repeated_reference_offsets_trace_reference32_payloads() {
    let first = ManagedReference::new(0x0102_0304);
    let second = ManagedReference::new(0x1122_3344);
    let bytes = [
        0x10, 0x20, 0x30, 0x40, 0x04, 0x03, 0x02, 0x01, 0x50, 0x60, 0x70, 0x80, 0x44, 0x33, 0x22,
        0x11,
    ];
    let trace = HeapScan::RepeatedReference {
        count: 2,
        stride: 8,
        local_offsets: vec![4].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let mut traced = Vec::new();

    // follow compact references using the declared repeated stride
    trace_managed_references(&trace, &bytes, 4, |reference| traced.push(reference))
        .expect("reference tracing should succeed");

    assert_eq!(traced, vec![first, second]);
}

/// Reject unsupported traced managed-reference widths loudly.
#[test]
fn test_reference_offsets_reject_unsupported_tracing_width() {
    let bytes = [0xAA, 0xBB, 0xCC, 0xDD];
    let trace = HeapScan::Reference {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };

    let error = trace_managed_references(&trace, &bytes, 3, |_| {})
        .expect_err("unsupported tracing widths should fail");

    assert_eq!(
        error,
        HeapError::UnsupportedManagedReferenceWidth { bytes: 3 }
    );
}
