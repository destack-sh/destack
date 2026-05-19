use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::trace::TraceSequence;
use crate::world::{BranchId, Moment};

use super::{
    Observation, ObservationEntry, ObservationOptions, ObservationSequence,
    ObservationSubscriptionId,
};

/// World-owned observation stream kept separate from causal trace.
#[derive(Debug, Default)]
pub struct Observations {
    /// Next observation sequence number.
    next_sequence: AtomicU64,
    /// Next observation subscription identifier.
    next_subscription_id: AtomicU64,
    /// Live uncommitted observation tail.
    tail: RwLock<Vec<ObservationEntry>>,
    /// Live observation subscriptions keyed by identifier.
    subscriptions: RwLock<BTreeMap<ObservationSubscriptionId, ObservationSubscription>>,
}

impl Observations {
    /// Record one observation at one exact execution coordinate.
    pub fn record_at(&self, moment: Moment, observation: Observation) -> ObservationSequence {
        let sequence = ObservationSequence::new(self.next_sequence.fetch_add(1, Ordering::SeqCst));
        let mut tail = self.tail.write();
        tail.push(ObservationEntry {
            sequence,
            moment,
            observation,
        });

        sequence
    }

    /// Return every filtered observation entry after the optional sequence.
    pub fn records_after(
        &self,
        after: Option<ObservationSequence>,
        options: ObservationOptions,
    ) -> Vec<ObservationEntry> {
        let tail = self.tail.read();

        let start_index = match after {
            Some(after) => tail.partition_point(|entry| entry.sequence <= after),
            None => 0,
        };

        tail[start_index..]
            .iter()
            .filter(|entry| options.allows(entry.observation.category))
            .cloned()
            .collect()
    }

    /// Return every observation entry within one moment range.
    pub fn records_between(&self, start: Moment, end: Moment) -> Vec<ObservationEntry> {
        let tail = self.tail.read();

        tail.iter()
            .filter(|entry| {
                entry.moment.branch_id == start.branch_id
                    && entry.moment.sequence.get() > start.sequence.get()
                    && entry.moment.sequence.get() <= end.sequence.get()
            })
            .cloned()
            .collect()
    }

    /// Drain every observation entry up to one exact committed sequence.
    pub fn drain_through(
        &self,
        branch_id: BranchId,
        sequence: TraceSequence,
    ) -> Vec<ObservationEntry> {
        let mut tail = self.tail.write();
        let split_index = tail.partition_point(|entry| {
            entry.moment.branch_id == branch_id && entry.moment.sequence.get() <= sequence.get()
        });

        tail.drain(..split_index).collect()
    }

    /// Reset the live observation tail and subscriptions.
    pub fn reset(&self) {
        self.next_sequence.store(0, Ordering::SeqCst);
        self.next_subscription_id.store(0, Ordering::SeqCst);
        self.tail.write().clear();
        self.subscriptions.write().clear();
    }

    /// Open one live observation subscription.
    pub fn open(&self, options: ObservationOptions) -> ObservationSubscriptionId {
        let subscription_id = ObservationSubscriptionId::new(
            self.next_subscription_id.fetch_add(1, Ordering::SeqCst),
        );
        let next_sequence = ObservationSequence::new(self.next_sequence.load(Ordering::SeqCst));
        let mut subscriptions = self.subscriptions.write();
        subscriptions.insert(
            subscription_id,
            ObservationSubscription {
                options,
                next_sequence,
            },
        );

        subscription_id
    }

    /// Close one live observation subscription.
    pub fn close(&self, subscription_id: ObservationSubscriptionId) -> RuntimeResult<()> {
        let mut subscriptions = self.subscriptions.write();
        if subscriptions.remove(&subscription_id).is_none() {
            return Err(RuntimeError::ObservationSubscriptionNotFound {
                subscription_id: subscription_id.get(),
            }
            .boxed());
        }

        Ok(())
    }

    /// Read the next batch of observation entries from one subscription.
    pub fn next(
        &self,
        subscription_id: ObservationSubscriptionId,
        limit: usize,
    ) -> RuntimeResult<Vec<ObservationEntry>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut subscriptions = self.subscriptions.write();
        let Some(subscription) = subscriptions.get_mut(&subscription_id) else {
            return Err(RuntimeError::ObservationSubscriptionNotFound {
                subscription_id: subscription_id.get(),
            }
            .boxed());
        };

        let tail = self.tail.read();
        let start_index = tail.partition_point(|entry| entry.sequence < subscription.next_sequence);
        let mut batch = Vec::new();

        // scan the live observation tail from the subscription cursor
        for entry in &tail[start_index..] {
            if !subscription.options.allows(entry.observation.category) {
                continue;
            }

            batch.push(entry.clone());
            if batch.len() == limit {
                break;
            }
        }

        // advance the subscription cursor after a successful batch
        if let Some(entry) = batch.last() {
            subscription.next_sequence = entry.sequence.next();
        }

        Ok(batch)
    }
}

/// Live cursor state for one observation subscription.
#[derive(Debug, Clone, Copy)]
struct ObservationSubscription {
    /// Filter options for this subscription.
    options: ObservationOptions,
    /// Next sequence visible through this subscription.
    next_sequence: ObservationSequence,
}
