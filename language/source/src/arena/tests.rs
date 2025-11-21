use super::*;

/// Test simple allocation.
#[test]
fn test_simple_allocation() {
    let mut arena = Arena::with(1024, 1024);
    let id1 = arena.allocate(10);
    let id2 = arena.allocate(20);

    assert_eq!(*arena.get(id1), 10);
    assert_eq!(*arena.get(id2), 20);
    assert_eq!(arena.len(), 2);
}

/// Test chunk boundary.
#[test]
fn test_chunk_boundary() {
    let mut arena = Arena::with(2, 2);
    let id1 = arena.allocate(1);
    let id2 = arena.allocate(2);
    let id3 = arena.allocate(3);

    assert_eq!(*arena.get(id1), 1);
    assert_eq!(*arena.get(id2), 2);
    assert_eq!(*arena.get(id3), 3);

    assert_eq!(arena.chunks.len(), 2);
    assert_eq!(arena.chunks[0].len(), 2);
    assert_eq!(arena.chunks[1].len(), 1);
}

/// Test iterator.
#[test]
fn test_iter() {
    let mut arena = Arena::with(10, 10);
    for i in 0..100 {
        arena.allocate(i);
    }

    let vec: Vec<_> = arena.iter().copied().collect();
    assert_eq!(vec.len(), 100);
    for (i, val) in vec.iter().enumerate() {
        assert_eq!(*val, i as i32);
    }
}

/// Test stable addresses.
#[test]
fn test_stable_addresses() {
    let mut arena = Arena::with(2, 2);
    let id1 = arena.allocate(100);
    let ptr1 = arena.get(id1) as *const _;

    arena.allocate(200);
    arena.allocate(300);

    let ptr1_after = arena.get(id1) as *const _;
    assert_eq!(
        ptr1, ptr1_after,
        "address should not change when allocating new chunks"
    );
}

/// Test get mutable.
#[test]
fn test_get_mut() {
    let mut arena = Arena::with(1024, 1024);
    let id = arena.allocate(5);
    *arena.get_mut(id) = 10;
    assert_eq!(*arena.get(id), 10);
}
