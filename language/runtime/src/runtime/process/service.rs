#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Weak};

pub(crate) mod affinity;
pub(crate) mod executor;
pub(crate) mod registry;
#[cfg(target_os = "macos")]
pub(crate) mod unix;
#[cfg(windows)]
pub(crate) mod windows;

pub(crate) use registry::{CachedServiceHandle, global_service};

/// Process-global weak subscriber registry keyed by one stable runtime or agent id.
#[derive(Debug)]
pub struct ProcessSubscriberRegistry<K, T> {
    /// Weak subscribers keyed by one stable subscriber id.
    subscribers: HashMap<K, Weak<T>>,
}

impl<K, T> Default for ProcessSubscriberRegistry<K, T> {
    /// Build one empty process subscriber registry.
    fn default() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }
}

impl<K, T> ProcessSubscriberRegistry<K, T>
where
    K: Copy + Eq + Hash,
{
    /// Register one live subscriber.
    pub fn register(&mut self, key: K, subscriber: &Arc<T>) {
        self.prune();
        self.subscribers.insert(key, Arc::downgrade(subscriber));
    }

    /// Unregister one subscriber.
    pub fn unregister(&mut self, key: K) {
        self.subscribers.remove(&key);
    }

    /// Return whether the registry is empty after pruning dead subscribers.
    pub fn is_empty(&mut self) -> bool {
        self.prune();
        self.subscribers.is_empty()
    }

    /// Return one snapshot of the live subscribers.
    pub fn snapshot(&mut self) -> Vec<Arc<T>> {
        self.prune();

        self.subscribers
            .values()
            .filter_map(Weak::upgrade)
            .collect()
    }

    /// Prune dead weak subscribers.
    pub fn prune(&mut self) {
        self.subscribers.retain(|_, weak| weak.strong_count() > 0);
    }
}

// FUGU #Architecture: cleanup
