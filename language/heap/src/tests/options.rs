use crate::{Heap, HeapError, HeapImage, HeapLimits, HeapOptions};

/// Reject unsupported managed-reference widths at heap construction.
#[test]
fn test_heap_rejects_invalid_managed_reference_width() {
    let options = HeapOptions {
        managed_reference_bytes: 3,
        ..HeapOptions::default()
    };

    let error = Heap::with_limits_and_options(HeapLimits::default(), options)
        .expect_err("invalid heap options should fail loudly");

    assert_eq!(
        error,
        HeapError::UnsupportedManagedReferenceWidth { bytes: 3 }
    );
}

/// Reject invalid remembered-card widths at heap construction.
#[test]
fn test_heap_rejects_invalid_card_width() {
    let options = HeapOptions {
        card_bytes: 0,
        ..HeapOptions::default()
    };

    let error = Heap::with_limits_and_options(HeapLimits::default(), options)
        .expect_err("invalid heap options should fail loudly");

    assert_eq!(error, HeapError::InvalidCardBytes { bytes: 0 });
}

/// Reject invalid table chunk lengths at heap construction.
#[test]
fn test_heap_rejects_invalid_table_chunk_len() {
    let options = HeapOptions {
        table_chunk_len: 0,
        ..HeapOptions::default()
    };

    let error = Heap::with_limits_and_options(HeapLimits::default(), options)
        .expect_err("invalid heap options should fail loudly");

    assert_eq!(error, HeapError::InvalidTableChunkLen { len: 0 });
}

/// Reject contradictory managed young-space admission policy.
#[test]
fn test_heap_rejects_young_threshold_above_capacity() {
    let options = HeapOptions {
        managed_young_bytes: 1024,
        max_managed_young_allocation_bytes: 2048,
        ..HeapOptions::default()
    };

    let error = Heap::with_limits_and_options(HeapLimits::default(), options)
        .expect_err("invalid heap options should fail loudly");

    assert_eq!(
        error,
        HeapError::ManagedYoungThresholdExceedsCapacity {
            threshold: 2048,
            capacity: 1024,
        }
    );
}

/// Reject misaligned size classes under one explicit heap alignment.
#[test]
fn test_heap_rejects_misaligned_size_class_table() {
    let options = HeapOptions {
        size_classes: crate::SizeClassTable::new([16, 24, 32])
            .expect("size classes should validate structurally"),
        small_allocation_alignment_bytes: 16,
        ..HeapOptions::default()
    };

    let error = Heap::with_limits_and_options(HeapLimits::default(), options)
        .expect_err("invalid heap options should fail loudly");

    assert_eq!(
        error,
        HeapError::MisalignedSizeClass {
            alignment_bytes: 16,
            class_bytes: 24,
        }
    );
}

/// Reject unsupported managed-reference widths when restoring a heap image.
#[test]
fn test_heap_image_rejects_invalid_managed_reference_width() {
    let heap = Heap::new().expect("default heap should build");
    let image = heap.image().expect("heap image should capture");
    let managed = image.managed().clone().with_managed_reference_bytes(3);
    let image = HeapImage::new(
        image.arena().clone(),
        image.options().clone(),
        managed,
        image.raw().clone(),
    );

    let error =
        Heap::from_image(&image).expect_err("invalid heap image options should fail loudly");

    assert_eq!(
        error,
        HeapError::UnsupportedManagedReferenceWidth { bytes: 3 }
    );
}
