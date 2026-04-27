use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use crate::{HeapError, HeapReference, HeapResult};

/// Active pin state for scoped heap borrows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PinSet {
    /// The active scoped pins keyed by heap reference.
    counts: BTreeMap<HeapReference, NonZeroUsize>,
    /// The total number of active scoped pins across all references.
    active_count: usize,
}

impl PinSet {
    /// Pin one heap reference.
    pub(crate) fn pin(&mut self, reference: HeapReference) -> HeapResult<()> {
        if let Some(count) = self.counts.get_mut(&reference) {
            self.active_count += 1;
            let next_count = count.get() + 1;

            // adding to a nonzero count preserves nonzero
            *count = unsafe { NonZeroUsize::new_unchecked(next_count) };

            return Ok(());
        }

        self.active_count += 1;
        self.counts.insert(reference, NonZeroUsize::MIN);

        Ok(())
    }

    /// Unpin one heap reference.
    pub(crate) fn unpin(&mut self, reference: HeapReference) -> HeapResult<()> {
        let Some(count) = self.counts.get(&reference).copied() else {
            return Err(HeapError::HeapPinMissing { reference });
        };

        self.active_count -= 1;

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
    pub(crate) const fn is_active(&self) -> bool {
        self.active_count != 0
    }

    /// Return every currently pinned heap reference.
    pub(crate) fn references(&self) -> impl Iterator<Item = HeapReference> + '_ {
        self.counts.keys().copied()
    }

    /// Rewrite every pinned reference through one promotion map.
    pub(crate) fn rewrite(&mut self, references: &BTreeMap<HeapReference, HeapReference>) {
        if references.is_empty() {
            return;
        }

        let previous_counts = std::mem::take(&mut self.counts);
        let mut next_counts: BTreeMap<HeapReference, NonZeroUsize> = BTreeMap::new();

        for (reference, count) in previous_counts {
            let reference = references.get(&reference).copied().unwrap_or(reference);

            if let Some(previous_count) = next_counts.get_mut(&reference) {
                let merged_count = previous_count.get() + count.get();

                // merging nonzero counts preserves nonzero
                let merged_count = unsafe { NonZeroUsize::new_unchecked(merged_count) };

                *previous_count = merged_count;

                continue;
            }

            next_counts.insert(reference, count);
        }

        self.counts = next_counts;
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
        assert_eq!(pins.active_count, 2);
        assert_eq!(
            pins.counts.get(&reference).map(|count| count.get()),
            Some(2)
        );

        pins.unpin(reference).expect("first unpin should succeed");

        assert!(pins.is_active());
        assert_eq!(pins.references().collect::<Vec<_>>(), vec![reference]);
        assert_eq!(pins.active_count, 1);
        assert_eq!(
            pins.counts.get(&reference).map(|count| count.get()),
            Some(1)
        );

        pins.unpin(reference).expect("second unpin should succeed");

        assert!(!pins.is_active());
        assert!(pins.references().next().is_none());
        assert_eq!(pins.active_count, 0);
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
