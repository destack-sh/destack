use crate::{HeapLimits, HeapOptions, ReferenceMap};

use super::tests::TestHeap;

/// Reject one managed allocation when the active-byte limit would be exceeded.
#[test]
fn test_reject_managed_allocation_when_limit_exceeded() {
    let mut test_heap = TestHeap::with_options(HeapOptions {
        managed_young_bytes: 0,
        max_managed_young_allocation_bytes: 0,
        ..HeapOptions::default()
    });
    let heap = &mut test_heap.heap;
    let baseline = heap
        .usage()
        .expect("heap usage should resolve")
        .managed
        .active_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        managed: crate::ManagedLimits {
            max_bytes: Some(baseline),
        },
        raw: crate::RawLimits { max_bytes: None },
    })
    .expect("baseline managed heap should fit its current active-byte limit");

    let error = heap
        .allocate_managed_bytes(&[1], ReferenceMap::empty(), None)
        .expect_err("managed allocation should be rejected");

    assert!(matches!(
        error,
        crate::HeapError::LimitExceeded {
            domain: crate::HeapDomain::Managed,
            ..
        }
    ));
    assert_eq!(heap.managed_allocation_count(), 0);
    assert_eq!(
        heap.usage()
            .expect("heap usage should resolve")
            .managed
            .active_bytes,
        baseline
    );
}

/// Reject one raw allocation when the active-byte limit would be exceeded.
#[test]
fn test_reject_raw_allocation_when_limit_exceeded() {
    let mut test_heap = TestHeap::new();
    let heap = &mut test_heap.heap;
    let baseline = heap
        .usage()
        .expect("heap usage should resolve")
        .raw
        .active_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        managed: crate::ManagedLimits { max_bytes: None },
        raw: crate::RawLimits {
            max_bytes: Some(baseline),
        },
    })
    .expect("baseline raw heap should fit its current active-byte limit");

    let error = heap
        .allocate_raw_bytes(&[1])
        .expect_err("raw allocation should be rejected");

    assert!(matches!(
        error,
        crate::HeapError::LimitExceeded {
            domain: crate::HeapDomain::Raw,
            ..
        }
    ));
    assert_eq!(heap.raw().allocation_count(), 0);
    assert_eq!(
        heap.usage()
            .expect("heap usage should resolve")
            .raw
            .active_bytes,
        baseline
    );
}

/// Preserve custom heap limits across in-place heap image restore.
#[test]
fn test_restore_heap_image_preserves_limits() {
    let mut test_heap = TestHeap::new();
    let heap = &mut test_heap.heap;
    let limits = HeapLimits {
        max_bytes: Some(
            heap.usage()
                .expect("heap usage should resolve")
                .active_bytes()
                .expect("heap usage totals should stay exact")
                + 4096,
        ),
        managed: crate::ManagedLimits {
            max_bytes: Some(
                heap.usage()
                    .expect("heap usage should resolve")
                    .managed
                    .active_bytes
                    + 2048,
            ),
        },
        raw: crate::RawLimits {
            max_bytes: Some(
                heap.usage()
                    .expect("heap usage should resolve")
                    .raw
                    .active_bytes
                    + 2048,
            ),
        },
    };
    heap.set_limits(limits)
        .expect("custom heap limits should fit current usage");
    let image = heap.image().expect("heap image should capture");

    // restoring one captured image should keep the existing hard limits
    heap.restore_image(&image)
        .expect("heap image restore should succeed");

    assert_eq!(heap.budget().limits(), limits);
}

/// Reject one raw replace when byte growth would exceed the active-byte limit.
#[test]
fn test_reject_raw_replace_when_limit_exceeded() {
    let mut test_heap = TestHeap::new();
    let heap = &mut test_heap.heap;
    let pointer = heap
        .allocate_raw_bytes(&vec![0xAA; 4097])
        .expect("raw allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut heap = crate::Heap::from_image(&image).expect("heap image layout should restore");
    let baseline = heap
        .usage()
        .expect("heap usage should resolve")
        .raw
        .active_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        managed: crate::ManagedLimits { max_bytes: None },
        raw: crate::RawLimits {
            max_bytes: Some(baseline),
        },
    })
    .expect("baseline raw heap should fit its current active-byte limit");

    let error = heap
        .replace_raw_bytes(pointer, &vec![0xBB; 8193])
        .expect_err("raw replace should be rejected");

    assert!(matches!(
        error,
        crate::HeapError::LimitExceeded {
            domain: crate::HeapDomain::Raw,
            ..
        }
    ));
    assert_eq!(heap.read_raw_bytes(pointer), Ok(vec![0xAA; 4097]));
    assert_eq!(
        heap.usage()
            .expect("heap usage should resolve")
            .raw
            .active_bytes,
        baseline
    );
}
