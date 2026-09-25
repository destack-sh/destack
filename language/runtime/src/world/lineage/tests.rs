use tspp_heap::AllocationShape;
use tspp_mir::TraceMap;
use tspp_repository::RuntimeOptions;

use crate::tests::{TestProgram, TestWorld};

/// Restore exact shared memory bytes from one retained Image.
#[test]
fn test_restore_image_memory() {
    let mut world = TestWorld::build(&RuntimeOptions::default(), TestProgram::mir(""));
    let shape = AllocationShape::new(4, 1, None, TraceMap::empty());
    let reference = world.allocate_shared(shape);
    world.write_shared(reference, &[1, 2, 3, 4]);
    let image = world.image("before-write");
    let snapshot = world.snapshot(image);

    // isolate mutations after the image
    world.write_shared(reference, &[9, 8, 7, 6]);
    assert_eq!(world.read_shared(reference, 4), [9, 8, 7, 6]);

    // restore the exact retained memory range and allocation metadata
    world.restore(&snapshot);
    assert_eq!(world.read_shared(reference, 4), [1, 2, 3, 4]);
}

/// Preserve exact shared memory bytes through world snapshot serialization.
#[test]
fn test_roundtrip_image_memory() {
    let mut world = TestWorld::build(&RuntimeOptions::default(), TestProgram::mir(""));
    let shape = AllocationShape::new(4, 1, None, TraceMap::empty());
    let reference = world.allocate_shared(shape);
    world.write_shared(reference, &[1, 2, 3, 4]);
    let image = world.image("serialized");
    let snapshot = world.snapshot(image);
    let bytes = snapshot.encode().expect("world snapshot should encode");
    let runtime_id = world.runtime_id();

    // rebuild a fresh world from serialized memory and metadata
    let world = TestWorld::from_snapshot_bytes(&bytes, runtime_id);
    assert_eq!(world.read_shared(reference, 4), [1, 2, 3, 4]);
}
