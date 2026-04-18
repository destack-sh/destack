#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Weak};

use crate::diagnostic::RuntimeResult;
use crate::runtime::process::{ExecutionLifetime, ExecutionMode, ExecutionPolicy};

pub(crate) mod executor;
pub(crate) mod registry;
#[cfg(target_os = "macos")]
pub(crate) mod unix;
#[cfg(windows)]
pub(crate) mod windows;

pub(crate) use registry::ServiceHandle;

#[cfg(windows)]
use self::executor::thread::ServiceThreadExecutor;

/// One process-global service with one declared execution policy.
pub(crate) trait Service: Sized {
    /// The execution policy for this service.
    const POLICY: ExecutionPolicy;

    /// Return one shared process-global service with one declared execution policy.
    fn global(builder: impl FnOnce() -> RuntimeResult<Self>) -> RuntimeResult<Arc<Self>>
    where
        Self: Send + Sync + 'static,
    {
        assert_global_service_policy::<Self>();

        registry::global_service(builder)
    }

    /// Return one shared process-global service when it is already live.
    ///
    /// This is for callback ingress paths that may race service teardown.
    fn active() -> Option<Arc<Self>>
    where
        Self: Send + Sync + 'static,
    {
        assert_global_service_policy::<Self>();

        registry::global_service_if_initialized()
    }
}

/// Open one owned service thread with one explicit execution policy.
#[cfg(windows)]
pub(crate) fn spawn_service_thread<State>(
    name: &str,
    policy: ExecutionPolicy,
    build: impl FnOnce() -> RuntimeResult<State> + Send + 'static,
) -> RuntimeResult<ServiceThreadExecutor<State>>
where
    State: 'static,
{
    ServiceThreadExecutor::spawn(name, policy, build)
}

/// Anchor one declared execution policy at the global-service boundary.
fn assert_global_service_policy<S>()
where
    S: Service,
{
    let policy = S::POLICY;

    if policy.lifetime != ExecutionLifetime::Global {
        panic!("global service must declare one global lifetime");
    }

    match policy.mode {
        ExecutionMode::Inline => {}
        ExecutionMode::Host => {}
        ExecutionMode::Thread => {}
        ExecutionMode::Loop => {}
        ExecutionMode::Polling => {}
        ExecutionMode::Blocking => {
            panic!("global service cannot declare blocking mode");
        }
    }
}

/// Process-global weak subscriber registry keyed by one stable runtime or worker id.
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
