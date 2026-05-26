use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use crate::{HeapError, HeapReference, HeapResult};

/// Active pin state for scoped heap borrows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PinSet {
    /// The active scoped pins keyed by heap reference.
    counts: BTreeMap<HeapReference, NonZeroUsize>,
}

impl PinSet {
    /// Pin one heap reference.
    pub(crate) fn pin(&mut self, reference: HeapReference) -> HeapResult<()> {
        if let Some(count) = self.counts.get_mut(&reference) {
            *count = Self::checked_add_nonzero(*count, NonZeroUsize::MIN)?;

            return Ok(());
        }

        self.counts.insert(reference, NonZeroUsize::MIN);

        Ok(())
    }

    /// Unpin one heap reference.
    pub(crate) fn unpin(&mut self, reference: HeapReference) -> HeapResult<()> {
        let Some(count) = self.counts.get(&reference).copied() else {
            return Err(HeapError::HeapPinMissing { reference });
        };

        if count.get() == 1 {
            self.counts.remove(&reference);

            return Ok(());
        }

        let next_count =
            NonZeroUsize::new(count.get() - 1).ok_or(HeapError::HeapPinMissing { reference })?;
        let Some(count) = self.counts.get_mut(&reference) else {
            return Err(HeapError::HeapPinMissing { reference });
        };
        *count = next_count;

        Ok(())
    }

    /// Return whether any scoped pin is active.
    pub(crate) fn is_active(&self) -> bool {
        !self.counts.is_empty()
    }

    /// Return whether one heap reference is currently pinned.
    pub(crate) fn contains(&self, reference: HeapReference) -> bool {
        self.counts.contains_key(&reference)
    }

    /// Return every currently pinned heap reference.
    pub(crate) fn references(&self) -> impl Iterator<Item = HeapReference> + '_ {
        self.counts.keys().copied()
    }

    /// Add two non-zero pin counts.
    fn checked_add_nonzero(left: NonZeroUsize, right: NonZeroUsize) -> HeapResult<NonZeroUsize> {
        let count = left
            .get()
            .checked_add(right.get())
            .ok_or(HeapError::InvariantViolation {
                context: "heap pin count overflow",
            })?;
        let count = NonZeroUsize::new(count).ok_or(HeapError::InvariantViolation {
            context: "heap pin count overflow",
        })?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keep exact scoped pin counts per heap reference.
    #[test]
    fn test_pin_tracks_exact_counts() {
        let reference = HeapReference::new(7);
        let mut pins = PinSet::default();

        // repeated pins should stay active until the final unpin
        pins.pin(reference).expect("first pin should succeed");
        pins.pin(reference).expect("second pin should succeed");

        assert!(pins.is_active());
        assert_eq!(pins.references().collect::<Vec<_>>(), vec![reference]);
        assert_eq!(
            pins.counts.get(&reference).map(|count| count.get()),
            Some(2)
        );

        pins.unpin(reference).expect("first unpin should succeed");

        assert!(pins.is_active());
        assert_eq!(pins.references().collect::<Vec<_>>(), vec![reference]);
        assert_eq!(
            pins.counts.get(&reference).map(|count| count.get()),
            Some(1)
        );

        pins.unpin(reference).expect("second unpin should succeed");

        assert!(!pins.is_active());
        assert!(pins.references().next().is_none());
    }

    /// Reject unpinning one reference with no active scoped pin.
    #[test]
    fn test_unpin_rejects_missing_reference() {
        let reference = HeapReference::new(7);
        let mut pins = PinSet::default();

        // missing pins should fail loudly
        let error = pins
            .unpin(reference)
            .expect_err("missing heap pin should fail");

        assert_eq!(error, HeapError::HeapPinMissing { reference });
    }
}
