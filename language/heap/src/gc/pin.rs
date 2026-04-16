use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::value::ManagedReference;

#[cfg(test)]
use crate::{HeapError, HeapResult};

/// Active pin state for scoped managed borrows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PinSet {
    /// The active scoped pins keyed by managed reference.
    entries: BTreeMap<ManagedReference, PinEntry>,
    /// The total number of active scoped pins across all references.
    active_count: usize,
}

impl PinSet {
    /// Pin one managed reference.
    #[cfg(test)]
    pub(crate) fn pin(&mut self, reference: ManagedReference) -> HeapResult<()> {
        let Some(entry) = self.entries.get_mut(&reference) else {
            self.active_count = self.active_count.checked_add(1).ok_or(
                HeapError::ManagedPinActiveCountOverflow {
                    active_count: self.active_count,
                },
            )?;
            self.entries.insert(
                reference,
                PinEntry {
                    count: NonZeroU32::MIN,
                },
            );

            return Ok(());
        };

        let count = entry
            .count
            .get()
            .checked_add(1)
            .ok_or(HeapError::ManagedPinCountOverflow {
                reference,
                count: entry.count.get(),
            })?;
        self.active_count =
            self.active_count
                .checked_add(1)
                .ok_or(HeapError::ManagedPinActiveCountOverflow {
                    active_count: self.active_count,
                })?;
        let Some(count) = NonZeroU32::new(count) else {
            return Err(HeapError::ManagedPinCountOverflow { reference, count });
        };

        entry.count = count;

        Ok(())
    }

    /// Unpin one managed reference.
    #[cfg(test)]
    pub(crate) fn unpin(&mut self, reference: ManagedReference) -> HeapResult<()> {
        let Some(entry) = self.entries.get(&reference) else {
            return Err(HeapError::ManagedPinMissing { reference });
        };
        let count = entry.count.get();
        self.active_count =
            self.active_count
                .checked_sub(1)
                .ok_or(HeapError::ManagedPinActiveCountUnderflow {
                    reference,
                    active_count: self.active_count,
                })?;

        if count == 1 {
            self.entries.remove(&reference);

            return Ok(());
        }

        let Some(entry) = self.entries.get_mut(&reference) else {
            return Err(HeapError::ManagedPinMissing { reference });
        };
        let Some(count) = NonZeroU32::new(count - 1) else {
            return Err(HeapError::ManagedPinMissing { reference });
        };

        entry.count = count;

        Ok(())
    }

    /// Return whether one managed reference is currently pinned.
    #[cfg(test)]
    pub(crate) fn is_pinned(&self, reference: ManagedReference) -> bool {
        self.entries.contains_key(&reference)
    }

    /// Return whether any scoped pin is active.
    pub(crate) const fn is_active(&self) -> bool {
        self.active_count != 0
    }
}

/// One active scoped pin entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PinEntry {
    /// The active scoped pin count for one managed reference.
    count: NonZeroU32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keep exact scoped pin counts per managed reference.
    #[test]
    fn test_pin_tracks_exact_counts() {
        let reference = ManagedReference::new(7);
        let mut pins = PinSet::default();

        // repeated pins should stay active until the final unpin
        pins.pin(reference).expect("first pin should succeed");
        pins.pin(reference).expect("second pin should succeed");

        assert!(pins.is_active());
        assert!(pins.is_pinned(reference));
        assert_eq!(pins.active_count, 2);
        assert_eq!(
            pins.entries.get(&reference).map(|entry| entry.count.get()),
            Some(2)
        );

        pins.unpin(reference).expect("first unpin should succeed");

        assert!(pins.is_active());
        assert!(pins.is_pinned(reference));
        assert_eq!(pins.active_count, 1);
        assert_eq!(
            pins.entries.get(&reference).map(|entry| entry.count.get()),
            Some(1)
        );

        pins.unpin(reference).expect("second unpin should succeed");

        assert!(!pins.is_active());
        assert!(!pins.is_pinned(reference));
        assert_eq!(pins.active_count, 0);
    }

    /// Reject scoped pin increments that exceed the encoded per-reference count.
    #[test]
    fn test_pin_rejects_count_overflow() {
        let reference = ManagedReference::new(7);
        let mut pins = PinSet {
            entries: BTreeMap::from([(
                reference,
                PinEntry {
                    count: NonZeroU32::MAX,
                },
            )]),
            active_count: 1,
        };

        // pin counts must fail loudly instead of saturating
        let error = pins.pin(reference).expect_err("pin overflow should fail");

        assert_eq!(
            error,
            HeapError::ManagedPinCountOverflow {
                reference,
                count: u32::MAX,
            }
        );
        assert_eq!(pins.active_count, 1);
        assert_eq!(
            pins.entries.get(&reference).map(|entry| entry.count.get()),
            Some(u32::MAX)
        );
    }

    /// Reject scoped pin increments that exceed the exact active count.
    #[test]
    fn test_pin_rejects_active_count_overflow() {
        let reference = ManagedReference::new(7);
        let mut pins = PinSet {
            entries: BTreeMap::new(),
            active_count: usize::MAX,
        };

        // active counts must fail loudly instead of saturating
        let error = pins
            .pin(reference)
            .expect_err("active pin overflow should fail");

        assert_eq!(
            error,
            HeapError::ManagedPinActiveCountOverflow {
                active_count: usize::MAX,
            }
        );
        assert!(!pins.is_pinned(reference));
        assert_eq!(pins.active_count, usize::MAX);
    }

    /// Reject unpinning one reference with no active scoped pin.
    #[test]
    fn test_unpin_rejects_missing_reference() {
        let reference = ManagedReference::new(7);
        let mut pins = PinSet::default();

        // missing pins should fail loudly
        let error = pins
            .unpin(reference)
            .expect_err("missing managed pin should fail");

        assert_eq!(error, HeapError::ManagedPinMissing { reference });
    }

    /// Reject unpinning when the table lost its active-count invariant.
    #[test]
    fn test_unpin_rejects_active_count_underflow() {
        let reference = ManagedReference::new(7);
        let mut pins = PinSet {
            entries: BTreeMap::from([(
                reference,
                PinEntry {
                    count: NonZeroU32::MIN,
                },
            )]),
            active_count: 0,
        };

        // active counts must fail loudly instead of panicking
        let error = pins
            .unpin(reference)
            .expect_err("active pin underflow should fail");

        assert_eq!(
            error,
            HeapError::ManagedPinActiveCountUnderflow {
                reference,
                active_count: 0,
            }
        );
        assert!(pins.is_pinned(reference));
        assert_eq!(pins.active_count, 0);
    }
}
