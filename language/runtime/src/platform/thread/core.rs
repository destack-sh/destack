use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::resource::{ResourceId, ThreadLocalKey};

/// Next thread-local key identifier.
static NEXT_THREAD_LOCAL_KEY: AtomicU64 = AtomicU64::new(1);

/// Global registry of active thread-local keys.
static THREAD_LOCAL_KEYS: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();

thread_local! {
    /// Per-thread value map for created thread-local keys.
    static THREAD_LOCAL_VALUES: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
}

/// Return the global thread-local key registry.
fn thread_local_key_registry() -> &'static Mutex<HashSet<u64>> {
    THREAD_LOCAL_KEYS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Borrow the key registry mutably.
fn with_key_registry<T>(
    run: impl FnOnce(&mut HashSet<u64>) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let mut keys = thread_local_key_registry().lock().map_err(|_| {
        RuntimeError::from(PlatformError::io("thread local key registry lock poisoned")).boxed()
    })?;

    run(&mut keys)
}

/// Validate that a thread-local key exists.
fn ensure_thread_local_key_exists(key: u64) -> RuntimeResult<()> {
    let exists = with_key_registry(|keys| Ok(keys.contains(&key)))?;
    if exists {
        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "key",
        format!("thread local key not found: {key}"),
    ))
    .boxed())
}

/// Create one thread-local key.
pub(crate) fn thread_local_create() -> RuntimeResult<ThreadLocalKey> {
    let key = NEXT_THREAD_LOCAL_KEY.fetch_add(1, Ordering::Relaxed);
    with_key_registry(|keys| {
        keys.insert(key);
        Ok(())
    })?;

    Ok(ThreadLocalKey(ResourceId(key)))
}

/// Delete one thread-local key.
pub(crate) fn thread_local_delete(key: ThreadLocalKey) -> RuntimeResult<()> {
    let key_id = key.0.0;

    let removed = with_key_registry(|keys| Ok(keys.remove(&key_id)))?;
    if !removed {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "key",
            format!("thread local key not found: {key_id}"),
        ))
        .boxed());
    }

    THREAD_LOCAL_VALUES.with(|values| {
        values.borrow_mut().remove(&key_id);
    });

    Ok(())
}

/// Read one thread-local value.
pub(crate) fn thread_local_get(key: ThreadLocalKey) -> RuntimeResult<u64> {
    let key_id = key.0.0;
    ensure_thread_local_key_exists(key_id)?;

    let value =
        THREAD_LOCAL_VALUES.with(|values| values.borrow().get(&key_id).copied().unwrap_or(0));
    Ok(value)
}

/// Store one thread-local value.
pub(crate) fn thread_local_set(key: ThreadLocalKey, value: u64) -> RuntimeResult<()> {
    let key_id = key.0.0;
    ensure_thread_local_key_exists(key_id)?;

    THREAD_LOCAL_VALUES.with(|values| {
        values.borrow_mut().insert(key_id, value);
    });

    Ok(())
}
