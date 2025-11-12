use crate::{SmallVec, smallvec};

use core::hash::Hasher;
use core::iter::FromIterator;
use std::rc::Rc;

/// Validate that pushing into a zero-capacity small vector spills to the heap.
#[test]
fn test_zero() {
    let mut v = SmallVec::<_, 0>::new();
    assert!(!v.spilled());
    v.push(0usize);
    assert!(v.spilled());
    assert_eq!(&*v, &[0]);
}

/// Confirm inline storage holds short sequences.
#[test]
fn test_inline() {
    let mut v = SmallVec::<_, 16>::new();
    v.push("hello".to_owned());
    v.push("there".to_owned());
    assert_eq!(&*v, &["hello".to_owned(), "there".to_owned()][..]);
}

/// Ensure exceeding inline capacity spills existing values to the heap.
#[test]
fn test_spill() {
    let mut v = SmallVec::<_, 2>::new();
    v.push("hello".to_owned());
    assert_eq!(v[0], "hello");
    v.push("there".to_owned());
    v.push("burma".to_owned());
    assert_eq!(v[0], "hello");
    v.push("shave".to_owned());
    assert_eq!(
        &*v,
        &[
            "hello".to_owned(),
            "there".to_owned(),
            "burma".to_owned(),
            "shave".to_owned(),
        ][..]
    );
}

/// Ensure repeated spills preserve all appended values.
#[test]
fn test_double_spill() {
    let mut v = SmallVec::<_, 2>::new();
    v.push("hello".to_owned());
    v.push("there".to_owned());
    v.push("burma".to_owned());
    v.push("shave".to_owned());
    v.push("hello".to_owned());
    v.push("there".to_owned());
    v.push("burma".to_owned());
    v.push("shave".to_owned());
    assert_eq!(
        &*v,
        &[
            "hello".to_owned(),
            "there".to_owned(),
            "burma".to_owned(),
            "shave".to_owned(),
            "hello".to_owned(),
            "there".to_owned(),
            "burma".to_owned(),
            "shave".to_owned(),
        ][..]
    );
}

/// Check with_capacity respects inline limits and requested capacity.
#[test]
fn test_with_capacity() {
    let v: SmallVec<u8, 3> = SmallVec::with_capacity(1);
    assert!(v.is_empty());
    assert!(!v.spilled());
    assert_eq!(v.capacity(), 3);

    let v: SmallVec<u8, 3> = SmallVec::with_capacity(10);
    assert!(v.is_empty());
    assert!(v.spilled());
    assert_eq!(v.capacity(), 10);
}

/// Verify draining yields elements and retains capacity across spill states.
#[test]
fn test_drain() {
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    v.push(3);
    assert_eq!(v.drain(..).collect::<Vec<_>>(), &[3]);

    // force spill by pushing past inline storage
    v.push(3);
    v.push(4);
    v.push(5);
    let old_capacity = v.capacity();
    assert_eq!(v.drain(1..).collect::<Vec<_>>(), &[4, 5]);
    // drain should not change capacity
    assert_eq!(v.capacity(), old_capacity);

    // trigger tail-shifting while inline
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    v.push(1);
    v.push(2);
    assert_eq!(v.drain(..1).collect::<Vec<_>>(), &[1]);
}

/// Verify draining in reverse yields elements from back to front.
#[test]
fn test_drain_rev() {
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    v.push(3);
    assert_eq!(v.drain(..).rev().collect::<Vec<_>>(), &[3]);

    // spill by overflowing inline capacity
    v.push(3);
    v.push(4);
    v.push(5);
    assert_eq!(v.drain(..).rev().collect::<Vec<_>>(), &[5, 4, 3]);
}

/// Ensure forgetting a drain iterator leaves the remaining prefix intact.
#[test]
fn test_drain_forget() {
    let mut v: SmallVec<u8, 1> = smallvec![0, 1, 2, 3, 4, 5, 6, 7];
    std::mem::forget(v.drain(2..5));
    assert_eq!(v.len(), 2);
}

/// Exercise splice across empty, prefix, and suffix ranges.
#[test]
fn test_splice() {
    // suffix splice
    let mut v: SmallVec<u8, 1> = smallvec![0, 1, 2, 3, 4, 5, 6];
    let new = [7, 8, 9, 10];
    let u: SmallVec<u8, 1> = v.splice(6.., new).collect();
    assert_eq!(v, [0, 1, 2, 3, 4, 5, 7, 8, 9, 10]);
    assert_eq!(u, [6]);

    // empty range splice
    let mut v: SmallVec<u8, 1> = smallvec![0, 1, 2, 3, 4, 5, 6];
    let new = [7, 8, 9, 10];
    let u: SmallVec<u8, 1> = v.splice(1..1, new).collect();
    assert_eq!(v, [0, 7, 8, 9, 10, 1, 2, 3, 4, 5, 6]);
    assert_eq!(u, []);

    // splice at head
    let mut v: SmallVec<u8, 1> = smallvec![0, 1, 2, 3, 4, 5, 6];
    let new = [7, 8, 9, 10];
    let u: SmallVec<u8, 1> = v.splice(..3, new).collect();
    assert_eq!(v, [7, 8, 9, 10, 3, 4, 5, 6]);
    assert_eq!(u, [0, 1, 2]);
}

/// Ensure into_iter consumes the vector and yields elements in order.
#[test]
fn test_into_iter() {
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    v.push(3);
    assert_eq!(v.into_iter().collect::<Vec<_>>(), &[3]);

    // check order after spilling
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    v.push(3);
    v.push(4);
    v.push(5);
    assert_eq!(v.into_iter().collect::<Vec<_>>(), &[3, 4, 5]);
}

/// Ensure reversing into_iter yields elements in reverse order.
#[test]
fn test_into_iter_rev() {
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    v.push(3);
    assert_eq!(v.into_iter().rev().collect::<Vec<_>>(), &[3]);

    // reverse with spilling
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    v.push(3);
    v.push(4);
    v.push(5);
    assert_eq!(v.into_iter().rev().collect::<Vec<_>>(), &[5, 4, 3]);
}

/// Ensure dropped iterators release every element exactly once.
#[test]
fn test_into_iter_drop() {
    use std::cell::Cell;

    struct DropCounter<'a>(&'a Cell<i32>);

    impl<'a> Drop for DropCounter<'a> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    {
        let cell = Cell::new(0);
        let mut v: SmallVec<DropCounter<'_>, 2> = SmallVec::new();
        v.push(DropCounter(&cell));
        v.into_iter();
        assert_eq!(cell.get(), 1);
    }

    {
        let cell = Cell::new(0);
        let mut v: SmallVec<DropCounter<'_>, 2> = SmallVec::new();
        v.push(DropCounter(&cell));
        v.push(DropCounter(&cell));
        assert!(v.into_iter().next().is_some());
        assert_eq!(cell.get(), 2);
    }

    {
        let cell = Cell::new(0);
        let mut v: SmallVec<DropCounter<'_>, 2> = SmallVec::new();
        v.push(DropCounter(&cell));
        v.push(DropCounter(&cell));
        v.push(DropCounter(&cell));
        assert!(v.into_iter().next().is_some());
        assert_eq!(cell.get(), 3);
    }
    {
        let cell = Cell::new(0);
        let mut v: SmallVec<DropCounter<'_>, 2> = SmallVec::new();
        v.push(DropCounter(&cell));
        v.push(DropCounter(&cell));
        v.push(DropCounter(&cell));
        {
            let mut it = v.into_iter();
            assert!(it.next().is_some());
            assert!(it.next_back().is_some());
        }
        assert_eq!(cell.get(), 3);
    }
}

/// Check reserve, reserve_exact, and shrink_to_fit adjust capacity as expected.
#[test]
fn test_capacity() {
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    v.reserve(1);
    assert_eq!(v.capacity(), 2);
    assert!(!v.spilled());

    v.reserve_exact(0x100);
    assert!(v.capacity() >= 0x100);

    v.push(0);
    v.push(1);
    v.push(2);
    v.push(3);

    v.shrink_to_fit();
    assert!(v.capacity() < 0x100);
}

/// Ensure truncate discards tail elements and keeps earlier ones.
#[test]
fn test_truncate() {
    let mut v: SmallVec<Box<u8>, 8> = SmallVec::new();

    for x in 0..8 {
        v.push(Box::new(x));
    }
    v.truncate(4);

    assert_eq!(v.len(), 4);
    assert!(!v.spilled());

    assert_eq!(*v.swap_remove(1), 1);
    assert_eq!(*v.remove(1), 3);
    v.insert(1, Box::new(3));

    assert_eq!(&v.iter().map(|v| **v).collect::<Vec<_>>(), &[0, 3, 2]);
}

/// Ensure truncate works with borrowed mutable references.
#[test]
fn test_truncate_references() {
    let mut v = [0, 1, 2, 3, 4, 5, 6, 7];
    let mut i = 8;
    let mut v: SmallVec<&mut u8, 8> = v.iter_mut().collect();

    v.truncate(4);

    assert_eq!(v.len(), 4);
    assert!(!v.spilled());

    assert_eq!(*v.swap_remove(1), 1);
    assert_eq!(*v.remove(1), 3);
    v.insert(1, &mut i);

    assert_eq!(
        &v.iter_mut().map(|v| &mut **v).collect::<Vec<_>>(),
        &[&mut 0, &mut 8, &mut 2]
    );
}

/// Ensure split_off separates head and tail without reallocating.
#[test]
fn test_split_off() {
    let mut vec: SmallVec<u32, 4> = smallvec![1, 2, 3, 4, 5, 6];
    let orig_ptr = vec.as_ptr();
    let orig_capacity = vec.capacity();

    let split_off = vec.split_off(4);
    assert_eq!(&vec[..], &[1, 2, 3, 4]);
    assert_eq!(&split_off[..], &[5, 6]);
    assert_eq!(vec.capacity(), orig_capacity);
    assert_eq!(vec.as_ptr(), orig_ptr);
}

/// Ensure split_off(0) transfers allocation to the new vector.
#[test]
fn test_split_off_take_all() {
    // make capacity large enough to distinguish original from split allocation
    let mut vec = SmallVec::<u32, 4>::with_capacity(1000);
    vec.extend([1, 2, 3, 4, 5, 6]);
    let orig_ptr = vec.as_ptr();
    let orig_capacity: usize = vec.capacity();

    let split_off = vec.split_off(0);
    assert_eq!(&vec[..], &[]);
    assert_eq!(&split_off[..], &[1, 2, 3, 4, 5, 6]);
    assert_eq!(vec.capacity(), orig_capacity);
    assert_eq!(vec.as_ptr(), orig_ptr);

    // split-off vector should not reuse allocation
    assert!(split_off.capacity() < orig_capacity);
    assert_ne!(split_off.as_ptr(), orig_ptr);
}

/// Ensure append moves every element from the donor vector.
#[test]
fn test_append() {
    let mut v: SmallVec<u8, 8> = SmallVec::new();
    for x in 0..4 {
        v.push(x);
    }
    assert_eq!(v.len(), 4);

    let mut n: SmallVec<u8, 2> = SmallVec::from_buf([5, 6]);
    v.append(&mut n);
    assert_eq!(v.len(), 6);
    assert_eq!(n.len(), 0);

    assert_eq!(&v.iter().copied().collect::<Vec<_>>(), &[0, 1, 2, 3, 5, 6]);
}

/// Expect grow to panic when the requested capacity is smaller than the length.
#[test]
#[should_panic]
fn test_invalid_grow() {
    let mut v: SmallVec<u8, 8> = SmallVec::new();
    v.extend(0..8);
    v.grow(5);
}

/// Expect draining with an overflowing range to panic.
#[test]
#[should_panic]
fn test_drain_overflow() {
    let mut v: SmallVec<u8, 8> = smallvec![0];
    v.drain(..=usize::MAX);
}

/// Ensure extend_from_slice appends the provided items.
#[test]
fn test_extend_from_slice() {
    let mut v: SmallVec<u8, 8> = SmallVec::new();
    for x in 0..4 {
        v.push(x);
    }
    assert_eq!(v.len(), 4);
    v.extend_from_slice(&[5, 6]);
    assert_eq!(&v.iter().copied().collect::<Vec<_>>(), &[0, 1, 2, 3, 5, 6]);
}

/// Ensure extend_from_within duplicates the requested range.
#[test]
fn test_extend_from_within() {
    let mut v: SmallVec<u8, 8> = smallvec![0, 1, 2, 3];
    v.extend_from_within(1..3);
    assert_eq!(&v.iter().copied().collect::<Vec<_>>(), &[0, 1, 2, 3, 1, 2]);
}

/// Verify a single panic during drop does not double panic.
#[test]
#[should_panic]
fn test_drop_panic_smallvec() {
    // should only panic once, not double drop
    struct DropPanic;

    impl Drop for DropPanic {
        fn drop(&mut self) {
            panic!("drop panic");
        }
    }

    let mut v = SmallVec::<_, 1>::new();
    v.push(DropPanic);
}

/// Ensure equality comparisons mirror slice semantics.
#[test]
fn test_eq() {
    let mut a: SmallVec<u32, 2> = SmallVec::new();
    let mut b: SmallVec<u32, 2> = SmallVec::new();
    let mut c: SmallVec<u32, 2> = SmallVec::new();
    // a = [1, 2]
    a.push(1);
    a.push(2);
    // b = [1, 2]
    b.push(1);
    b.push(2);
    // c = [3, 4]
    c.push(3);
    c.push(4);

    assert!(a == b);
    assert!(a != c);
}

/// Ensure ordering comparisons mirror slice semantics.
#[test]
fn test_ord() {
    let mut a: SmallVec<u32, 2> = SmallVec::new();
    let mut b: SmallVec<u32, 2> = SmallVec::new();
    let mut c: SmallVec<u32, 2> = SmallVec::new();
    // a = [1]
    a.push(1);
    // b = [1, 1]
    b.push(1);
    b.push(1);
    // c = [1, 2]
    c.push(1);
    c.push(2);

    assert!(a < b);
    assert!(b > a);
    assert!(b < c);
    assert!(c > b);
}

/// Confirm hashing matches the equivalent slice.
#[test]
fn test_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;

    fn hash(value: impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    {
        let mut a: SmallVec<u32, 2> = SmallVec::new();
        let b = [1, 2];
        a.extend(b.iter().cloned());
        assert_eq!(hash(a), hash(b));
    }

    {
        let mut a: SmallVec<u32, 2> = SmallVec::new();
        let b = [1, 2, 11, 12];
        a.extend(b.iter().cloned());
        assert_eq!(hash(a), hash(b));
    }
}

/// Ensure as_ref exposes the underlying slice.
#[test]
fn test_as_ref() {
    let mut a: SmallVec<u32, 2> = SmallVec::new();
    a.push(1);
    assert_eq!(a.as_ref(), [1]);
    a.push(2);
    assert_eq!(a.as_ref(), [1, 2]);
    a.push(3);
    assert_eq!(a.as_ref(), [1, 2, 3]);
}

/// Ensure as_mut exposes the mutable slice and edits propagate.
#[test]
fn test_as_mut() {
    let mut a: SmallVec<u32, 2> = SmallVec::new();
    a.push(1);
    assert_eq!(a.as_mut(), [1]);
    a.push(2);
    assert_eq!(a.as_mut(), [1, 2]);
    a.push(3);
    assert_eq!(a.as_mut(), [1, 2, 3]);
    a.as_mut()[1] = 4;
    assert_eq!(a.as_mut(), [1, 4, 3]);
}

/// Ensure Borrow returns the expected slice view.
#[test]
fn test_borrow() {
    use std::borrow::Borrow;

    let mut a: SmallVec<u32, 2> = SmallVec::new();
    a.push(1);
    assert_eq!(a.borrow(), [1]);
    a.push(2);
    assert_eq!(a.borrow(), [1, 2]);
    a.push(3);
    assert_eq!(a.borrow(), [1, 2, 3]);
}

/// Ensure BorrowMut returns the expected mutable slice.
#[test]
fn test_borrow_mut() {
    use std::borrow::BorrowMut;

    let mut a: SmallVec<u32, 2> = SmallVec::new();
    a.push(1);
    assert_eq!(a.borrow_mut(), [1]);
    a.push(2);
    assert_eq!(a.borrow_mut(), [1, 2]);
    a.push(3);
    assert_eq!(a.borrow_mut(), [1, 2, 3]);
    BorrowMut::<[u32]>::borrow_mut(&mut a)[1] = 4;
    assert_eq!(a.borrow_mut(), [1, 4, 3]);
}

/// Ensure from conversions accept slices, arrays, and Vec inputs.
#[test]
fn test_from() {
    assert_eq!(&SmallVec::<u32, 2>::from(&[1][..])[..], [1]);
    assert_eq!(&SmallVec::<u32, 2>::from(&[1, 2, 3][..])[..], [1, 2, 3]);

    let vec = vec![];
    let small_vec: SmallVec<u8, 3> = SmallVec::from(vec);
    assert_eq!(&*small_vec, &[]);
    drop(small_vec);

    let vec = vec![1, 2, 3, 4, 5];
    let small_vec: SmallVec<u8, 3> = SmallVec::from(vec);
    assert_eq!(&*small_vec, &[1, 2, 3, 4, 5]);
    drop(small_vec);

    let vec = vec![1, 2, 3, 4, 5];
    let small_vec: SmallVec<u8, 1> = SmallVec::from(vec);
    assert_eq!(&*small_vec, &[1, 2, 3, 4, 5]);
    drop(small_vec);

    let array = [1];
    let small_vec: SmallVec<u8, 1> = SmallVec::from(array);
    assert_eq!(&*small_vec, &[1]);
    drop(small_vec);

    let array = [99; 128];
    let small_vec: SmallVec<u8, 128> = SmallVec::from(array);
    assert_eq!(&*small_vec, vec![99u8; 128].as_slice());
    drop(small_vec);

    #[derive(PartialEq, Eq, Debug)]
    struct NoClone(u8);
    let array = [NoClone(42)];
    let small_vec: SmallVec<NoClone, 1> = SmallVec::from(array);
    assert_eq!(&*small_vec, &[NoClone(42)]);
    drop(small_vec);

    let vec = vec![NoClone(42)];
    let small_vec: SmallVec<NoClone, 1> = SmallVec::from(vec);
    assert_eq!(&*small_vec, &[NoClone(42)]);
    drop(small_vec);

    let array = [1; 128];
    let small_vec: SmallVec<u8, 1> = SmallVec::from(array);
    assert_eq!(&*small_vec, vec![1; 128].as_slice());
    drop(small_vec);

    let array = [99];
    let small_vec: SmallVec<u8, 128> = SmallVec::from(array);
    assert_eq!(&*small_vec, &[99u8]);
    drop(small_vec);
}

/// Ensure converting from a slice preserves all elements.
#[test]
fn test_from_slice() {
    assert_eq!(&SmallVec::<u32, 2>::from(&[1][..])[..], [1]);
    assert_eq!(&SmallVec::<u32, 2>::from(&[1, 2, 3][..])[..], [1, 2, 3]);
}

/// Ensure iterator length hints stay correct for drain and into_iter.
#[test]
fn test_exact_size_iterator() {
    let mut vec = SmallVec::<u32, 2>::from(&[1, 2, 3][..]);
    assert_eq!(vec.clone().into_iter().len(), 3);
    assert_eq!(vec.drain(..2).len(), 2);
    assert_eq!(vec.into_iter().len(), 1);
}

/// Ensure into_iter exposes accurate slice views before and after consumption.
#[test]
fn test_into_iter_as_slice() {
    let vec = SmallVec::<u32, 2>::from(&[1, 2, 3][..]);
    let mut iter = vec.clone().into_iter();
    assert_eq!(iter.as_slice(), &[1, 2, 3]);
    assert_eq!(iter.as_mut_slice(), &[1, 2, 3]);
    iter.next();
    assert_eq!(iter.as_slice(), &[2, 3]);
    assert_eq!(iter.as_mut_slice(), &[2, 3]);
    iter.next_back();
    assert_eq!(iter.as_slice(), &[2]);
    assert_eq!(iter.as_mut_slice(), &[2]);
}

/// Ensure cloning into_iter yields the same sequence without aliasing.
#[test]
fn test_into_iter_clone() {
    // test that the cloned iterator yields identical elements and that it owns its own copy
    // i.e. no use-after-move errors
    let iter = SmallVec::<u8, 2>::from_iter(0..3).into_iter();
    let mut clone_iter = iter.clone();
    for x in iter {
        assert_eq!(x, clone_iter.next().unwrap());
    }
    assert_eq!(clone_iter.next(), None);
}

/// Ensure cloning an into_iter after partial consumption yields the remaining elements.
#[test]
fn test_into_iter_clone_partially_consumed_iterator() {
    // test that the cloned iterator only contains the remaining elements of the original iterator
    let iter = SmallVec::<u8, 2>::from_iter(0..3).into_iter().skip(1);
    let mut clone_iter = iter.clone();
    for x in iter {
        assert_eq!(x, clone_iter.next().unwrap());
    }
    assert_eq!(clone_iter.next(), None);
}

/// Ensure cloning an empty into_iter yields another empty iterator.
#[test]
fn test_into_iter_clone_empty_smallvec() {
    let mut iter = SmallVec::<u8, 2>::new().into_iter();
    let mut clone_iter = iter.clone();
    assert_eq!(iter.next(), None);
    assert_eq!(clone_iter.next(), None);
}

/// Ensure shrink_to_fit moves a spilled vector back to inline storage when possible.
#[test]
fn test_shrink_to_fit_unspill() {
    let mut vec = SmallVec::<u8, 2>::from_iter(0..3);
    vec.pop();
    assert!(vec.spilled());
    vec.shrink_to_fit();
    assert!(!vec.spilled(), "shrink_to_fit will un-spill if possible");
}

/// Ensure shrink_to_fit on a vector from_vec(vec![]) keeps inline storage.
#[test]
fn test_shrink_after_from_empty_vec() {
    let mut v = SmallVec::<u8, 2>::from_vec(vec![]);
    v.shrink_to_fit();
    assert!(!v.spilled())
}

/// Ensure into_vec returns an owned Vec with identical contents.
#[test]
fn test_into_vec() {
    let vec = SmallVec::<u8, 2>::from_iter(0..2);
    assert_eq!(vec.into_vec(), vec![0, 1]);

    let vec = SmallVec::<u8, 2>::from_iter(0..3);
    assert_eq!(vec.into_vec(), vec![0, 1, 2]);
}

/// Ensure into_inner succeeds only when the vector exactly fills inline storage.
#[test]
fn test_into_inner() {
    let vec = SmallVec::<u8, 2>::from_iter(0..2);
    assert_eq!(vec.into_inner(), Ok([0, 1]));

    let vec = SmallVec::<u8, 2>::from_iter(0..1);
    assert_eq!(vec.clone().into_inner(), Err(vec));

    let vec = SmallVec::<u8, 2>::from_iter(0..3);
    assert_eq!(vec.clone().into_inner(), Err(vec));
}

/// Ensure from_vec handles empty, inline, and spilled allocations.
#[test]
fn test_from_vec() {
    let vec = vec![];
    let small_vec: SmallVec<u8, 3> = SmallVec::from_vec(vec);
    assert_eq!(&*small_vec, &[]);
    drop(small_vec);

    let vec = vec![];
    let small_vec: SmallVec<u8, 1> = SmallVec::from_vec(vec);
    assert_eq!(&*small_vec, &[]);
    drop(small_vec);

    let vec = vec![1];
    let small_vec: SmallVec<u8, 3> = SmallVec::from_vec(vec);
    assert_eq!(&*small_vec, &[1]);
    drop(small_vec);

    let vec = vec![1, 2, 3];
    let small_vec: SmallVec<u8, 3> = SmallVec::from_vec(vec);
    assert_eq!(&*small_vec, &[1, 2, 3]);
    drop(small_vec);

    let vec = vec![1, 2, 3, 4, 5];
    let small_vec: SmallVec<u8, 3> = SmallVec::from_vec(vec);
    assert_eq!(&*small_vec, &[1, 2, 3, 4, 5]);
    drop(small_vec);

    let vec = vec![1, 2, 3, 4, 5];
    let small_vec: SmallVec<u8, 1> = SmallVec::from_vec(vec);
    assert_eq!(&*small_vec, &[1, 2, 3, 4, 5]);
    drop(small_vec);
}

/// Ensure retain filters correctly and runs drop hooks for removed items.
#[test]
fn test_retain() {
    // test inline data storage
    let mut sv: SmallVec<i32, 5> = SmallVec::from(&[1, 2, 3, 3, 4]);
    sv.retain(|&i| i != 3);
    assert_eq!(sv.pop(), Some(4));
    assert_eq!(sv.pop(), Some(2));
    assert_eq!(sv.pop(), Some(1));
    assert_eq!(sv.pop(), None);

    // test spilled data storage
    let mut sv: SmallVec<i32, 3> = SmallVec::from(&[1, 2, 3, 3, 4]);
    sv.retain(|&i| i != 3);
    assert_eq!(sv.pop(), Some(4));
    assert_eq!(sv.pop(), Some(2));
    assert_eq!(sv.pop(), Some(1));
    assert_eq!(sv.pop(), None);

    // test that drop implementations are called for inline storage
    let one = Rc::new(1);
    let mut sv: SmallVec<Rc<i32>, 3> = SmallVec::new();
    sv.push(Rc::clone(&one));
    assert_eq!(Rc::strong_count(&one), 2);
    sv.retain(|_| false);
    assert_eq!(Rc::strong_count(&one), 1);

    // test that drop implementations are called for spilled storage
    let mut sv: SmallVec<Rc<i32>, 1> = SmallVec::new();
    sv.push(Rc::clone(&one));
    sv.push(Rc::new(2));
    assert_eq!(Rc::strong_count(&one), 2);
    sv.retain(|_| false);
    assert_eq!(Rc::strong_count(&one), 1);
}

/// Ensure dedup removes consecutive duplicates across storage modes.
#[test]
fn test_dedup() {
    let mut dupes: SmallVec<i32, 5> = SmallVec::from(&[1, 1, 2, 3, 3]);
    dupes.dedup();
    assert_eq!(&*dupes, &[1, 2, 3]);

    let mut empty: SmallVec<i32, 5> = SmallVec::new();
    empty.dedup();
    assert!(empty.is_empty());

    let mut all_ones: SmallVec<i32, 5> = SmallVec::from(&[1, 1, 1, 1, 1]);
    all_ones.dedup();
    assert_eq!(all_ones.len(), 1);

    let mut no_dupes: SmallVec<i32, 5> = SmallVec::from(&[1, 2, 3, 4, 5]);
    no_dupes.dedup();
    assert_eq!(no_dupes.len(), 5);
}

/// Ensure resize grows with clones and shrinks by dropping elements.
#[test]
fn test_resize() {
    let mut v: SmallVec<i32, 8> = SmallVec::new();
    v.push(1);
    v.resize(5, 0);
    assert_eq!(v[..], [1, 0, 0, 0, 0][..]);

    v.resize(2, -1);
    assert_eq!(v[..], [1, 0][..]);
}

/// Ensure grow can shrink a spilled vector back to inline capacity.
#[test]
fn test_grow_to_shrink() {
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    assert!(v.spilled());
    v.clear();
    // shrink to inline
    v.grow(2);
    assert!(!v.spilled());
    assert_eq!(v.capacity(), 2);
    assert_eq!(v.len(), 0);
    v.push(4);
    assert_eq!(v[..], [4]);
}

/// Ensure extend resumes after iterator pauses and retains surviving items.
#[test]
fn test_resumable_extend() {
    let s = "a b c";
    // this iterator yields (Some('a'), None, Some('b'), None, Some('c')), None
    let it = s
        .chars()
        .scan(0, |_, ch| if ch.is_whitespace() { None } else { Some(ch) });
    let mut v: SmallVec<char, 4> = SmallVec::new();
    v.extend(it);
    assert_eq!(v[..], ['a']);
}

/// Ensure constructing a vector over an uninhabited type continues to compile.
#[test]
fn test_uninhabited() {
    enum Void {}
    let _sv = SmallVec::<Void, 8>::new();
}

/// Ensure grow with an unchanged capacity leaves spilled storage intact.
#[test]
fn test_grow_spilled_same_size() {
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    v.push(0);
    v.push(1);
    v.push(2);
    assert!(v.spilled());
    assert_eq!(v.capacity(), 4);
    // grow with the same capacity
    v.grow(4);
    assert_eq!(v.capacity(), 4);
    assert_eq!(v[..], [0, 1, 2]);
}

/// Ensure a large inline size works under const generics.
#[test]
fn test_const_generics() {
    let _v = SmallVec::<i32, 987>::default();
}

/// Ensure const constructors produce vectors with expected inline contents.
#[test]
fn test_const_new() {
    let v = const_new_inner();
    assert_eq!(v.capacity(), 4);
    assert_eq!(v.len(), 0);
    let v = const_new_inline_sized();
    assert_eq!(v.capacity(), 4);
    assert_eq!(v.len(), 4);
    assert_eq!(v[0], 1);
    let v = const_new_inline_args();
    assert_eq!(v.capacity(), 2);
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], 1);
    assert_eq!(v[1], 4);
}
const fn const_new_inner() -> SmallVec<i32, 4> {
    SmallVec::<i32, 4>::new()
}
const fn const_new_inline_sized() -> SmallVec<i32, 4> {
    crate::smallvec_inline![1; 4]
}
const fn const_new_inline_args() -> SmallVec<i32, 2> {
    crate::smallvec_inline![1, 4]
}

/// Ensure the smallvec! macro handles empty invocations.
#[test]
fn test_empty_macro() {
    let _v: SmallVec<u8, 1> = smallvec![];
}

/// Ensure vectors accept zero-sized elements.
#[test]
fn test_zero_size_items() {
    SmallVec::<(), 0>::new().push(());
}

/// Ensure clone_from reuses allocation and matches donor contents.
#[test]
fn test_clone_from() {
    let mut a: SmallVec<u8, 2> = SmallVec::new();
    a.push(1);
    a.push(2);
    a.push(3);

    let mut b: SmallVec<u8, 2> = SmallVec::new();
    b.push(10);

    let mut c: SmallVec<u8, 2> = SmallVec::new();
    c.push(20);
    c.push(21);
    c.push(22);

    a.clone_from(&b);
    assert_eq!(&*a, &[10]);

    b.clone_from(&c);
    assert_eq!(&*b, &[20, 21, 22]);
}

/// Ensure methods tolerate usize::MAX where they should not panic.
#[test]
fn test_max_dont_panic() {
    let mut sv: SmallVec<i32, 2> = smallvec![0];
    let _ = sv.get(usize::MAX);
    sv.truncate(usize::MAX);
}

/// Expect remove with usize::MAX to panic.
#[test]
#[should_panic]
fn test_max_remove() {
    let mut sv: SmallVec<i32, 2> = smallvec![0];
    sv.remove(usize::MAX);
}

/// Expect swap_remove with usize::MAX to panic.
#[test]
#[should_panic]
fn test_max_swap_remove() {
    let mut sv: SmallVec<i32, 2> = smallvec![0];
    sv.swap_remove(usize::MAX);
}

/// Expect insert with usize::MAX to panic.
#[test]
#[should_panic]
fn test_max_insert() {
    let mut sv: SmallVec<i32, 2> = smallvec![0];
    sv.insert(usize::MAX, 0);
}

/// Ensure collecting from an iterator without size hints grows correctly.
#[test]
fn test_collect_from_iter() {
    // regression test for https://github.com/servo/rust-smallvec/issues/353
    struct IterNoHint<I: Iterator>(I);

    impl<I: Iterator> Iterator for IterNoHint<I> {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()
        }

        // no implementation of size_hint means it returns (0, None) - which forces from_iter to
        // grow the allocated space iteratively.
    }

    // a length of 3 is fine to trigger this bug under valgrind, but making the vector 1 million
    // elements makes it crash - which is much easier to detect.
    let iter = IterNoHint(std::iter::repeat_n(1u8, 1_000_000));

    let _y: SmallVec<u8, 1> = SmallVec::from_iter(iter);
}

/// Ensure collecting chars spills once inline storage fills.
#[test]
fn test_collect_with_spill() {
    let input = "0123456";
    let collected: SmallVec<char, 4> = input.chars().collect();
    assert_eq!(collected, &['0', '1', '2', '3', '4', '5', '6']);
}

/// Ensure spare_capacity_mut reports accurate lengths and pointers.
#[test]
fn test_spare_capacity_mut() {
    let mut v: SmallVec<u8, 2> = SmallVec::new();
    assert!(!v.spilled());
    let spare = v.spare_capacity_mut();
    assert_eq!(spare.len(), 2);
    assert_eq!(spare.as_ptr().cast::<u8>(), v.as_ptr());

    v.push(1);
    assert!(!v.spilled());
    let spare = v.spare_capacity_mut();
    assert_eq!(spare.len(), 1);
    assert_eq!(spare.as_ptr().cast::<u8>(), unsafe { v.as_ptr().add(1) });

    v.push(2);
    assert!(!v.spilled());
    let spare = v.spare_capacity_mut();
    assert_eq!(spare.len(), 0);
    assert_eq!(spare.as_ptr().cast::<u8>(), unsafe { v.as_ptr().add(2) });

    v.push(3);
    assert!(v.spilled());
    let spare = v.spare_capacity_mut();
    assert!(!spare.is_empty());
    assert_eq!(spare.as_ptr().cast::<u8>(), unsafe { v.as_ptr().add(3) });
}
