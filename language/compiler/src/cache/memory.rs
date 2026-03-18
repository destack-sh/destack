use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use dashmap::DashMap;
use destack_workspace::{CacheError, CachePolicy};

use super::context::BYTES_PER_MB;
use super::{CacheKey, CacheOptions, CacheRegistry};

/// Cached entry stored in memory with access metadata.
#[derive(Debug, Clone)]
pub(super) struct CacheMemoryEntry<T> {
    /// The cached entry payload.
    pub(super) entry: T,
    /// Serialized size of the entry in bytes.
    pub(super) size_bytes: u64,
    /// When the entry was created.
    created_at: Instant,
    /// When the entry was last accessed.
    last_access: Instant,
}

impl<T> CacheMemoryEntry<T> {
    /// Create a new memory cache entry.
    pub(super) fn new(entry: T, size_bytes: u64) -> Self {
        let now = Instant::now();
        Self {
            entry,
            size_bytes,
            created_at: now,
            last_access: now,
        }
    }

    /// Update the last access timestamp.
    pub(super) fn touch(&mut self) {
        self.last_access = Instant::now();
    }

    /// Get the eviction key for the configured policy.
    fn eviction_key(&self, policy: CachePolicy) -> Instant {
        match policy {
            CachePolicy::Lru => self.last_access,
            CachePolicy::Ttl => self.created_at,
        }
    }
}

/// Memory cache size tracking state.
#[derive(Debug)]
pub(super) struct CacheMemoryState {
    /// Cached memory size in bytes.
    pub(super) size_bytes: AtomicU64,
}

impl CacheMemoryState {
    /// Create a new memory state tracker.
    pub(super) fn new() -> Self {
        Self {
            size_bytes: AtomicU64::new(0),
        }
    }
}

/// Identify the cache map for in memory eviction.
#[derive(Debug, Clone, Copy)]
enum MemoryCacheKind {
    /// AST cache entry.
    Ast,
    /// Base DIR cache entry.
    DirBase,
    /// Prepared DIR cache entry.
    DirPrepared,
    /// Resolved DIR cache entry.
    DirResolved,
    /// Analyzed DIR cache entry.
    DirAnalyzed,
    /// Patched DIR cache entry.
    DirPatched,
    /// MIR cache entry.
    Mir,
}

/// Memory cache entry used for eviction.
#[derive(Debug)]
struct MemoryEvictionEntry {
    /// Cache map for the entry.
    kind: MemoryCacheKind,
    /// Cache key for the entry.
    key: CacheKey,
    /// Size of the cached entry in bytes.
    size_bytes: u64,
    /// Eviction key for ordering.
    eviction_key: Instant,
}

impl CacheRegistry {
    /// Insert a cache entry into a memory map and track size.
    pub(super) fn insert_memory_entry<T>(
        &self,
        map: &DashMap<CacheKey, CacheMemoryEntry<T>>,
        key: CacheKey,
        entry: CacheMemoryEntry<T>,
    ) {
        let size_bytes = entry.size_bytes;
        if let Some(existing) = map.insert(key, entry) {
            self.decrement_memory_size(existing.size_bytes);
        }
        self.increment_memory_size(size_bytes);
    }

    /// Remove a cache entry from a memory map and track size.
    pub(super) fn remove_memory_entry<T>(
        &self,
        map: &DashMap<CacheKey, CacheMemoryEntry<T>>,
        key: &CacheKey,
    ) -> Option<CacheMemoryEntry<T>> {
        map.remove(key).map(|(_, entry)| {
            self.decrement_memory_size(entry.size_bytes);
            entry
        })
    }

    /// Compute the serialized entry size in bytes when limits are enabled.
    pub(super) fn entry_size_bytes<T: serde::Serialize>(
        &self,
        options: &CacheOptions,
        entry: &T,
    ) -> Result<u64, CacheError> {
        let Some(max_size_mb) = options.max_size_mb else {
            return Ok(0);
        };
        if max_size_mb == 0 {
            return Ok(0);
        }

        let bytes = destack_workspace::serialize_cache_entry(entry)?;
        Ok(bytes.len() as u64)
    }

    /// Evict in memory cache entries when size limits are exceeded.
    pub(super) fn evict_memory_entries(&self, options: &CacheOptions) -> Result<(), CacheError> {
        let Some(max_size_mb) = options.max_size_mb else {
            return Ok(());
        };

        if max_size_mb == 0 {
            return Ok(());
        }

        let limit_bytes = max_size_mb.saturating_mul(BYTES_PER_MB);
        let mut total_size = self.memory_state.size_bytes.load(Ordering::Relaxed);
        if total_size <= limit_bytes {
            return Ok(());
        }

        let mut entries = Vec::new();
        self.collect_memory_entries(options.policy, &mut entries);
        entries.sort_by_key(|entry| entry.eviction_key);

        for entry in entries {
            if total_size <= limit_bytes {
                break;
            }

            let removed = match entry.kind {
                MemoryCacheKind::Ast => self.remove_memory_entry(&self.ast, &entry.key).is_some(),
                MemoryCacheKind::DirBase => self
                    .remove_memory_entry(&self.dir_base, &entry.key)
                    .is_some(),
                MemoryCacheKind::DirPrepared => self
                    .remove_memory_entry(&self.dir_prepared, &entry.key)
                    .is_some(),
                MemoryCacheKind::DirResolved => self
                    .remove_memory_entry(&self.dir_resolved, &entry.key)
                    .is_some(),
                MemoryCacheKind::DirAnalyzed => self
                    .remove_memory_entry(&self.dir_analyzed, &entry.key)
                    .is_some(),
                MemoryCacheKind::DirPatched => self
                    .remove_memory_entry(&self.dir_patched, &entry.key)
                    .is_some(),
                MemoryCacheKind::Mir => self.remove_memory_entry(&self.mir, &entry.key).is_some(),
            };

            if removed {
                total_size = total_size.saturating_sub(entry.size_bytes);
            }
        }

        Ok(())
    }

    /// Collect all memory cache entries for eviction.
    fn collect_memory_entries(&self, policy: CachePolicy, entries: &mut Vec<MemoryEvictionEntry>) {
        for entry in self.ast.iter() {
            entries.push(MemoryEvictionEntry {
                kind: MemoryCacheKind::Ast,
                key: entry.key().clone(),
                size_bytes: entry.value().size_bytes,
                eviction_key: entry.value().eviction_key(policy),
            });
        }

        for entry in self.dir_base.iter() {
            entries.push(MemoryEvictionEntry {
                kind: MemoryCacheKind::DirBase,
                key: entry.key().clone(),
                size_bytes: entry.value().size_bytes,
                eviction_key: entry.value().eviction_key(policy),
            });
        }

        for entry in self.dir_prepared.iter() {
            entries.push(MemoryEvictionEntry {
                kind: MemoryCacheKind::DirPrepared,
                key: entry.key().clone(),
                size_bytes: entry.value().size_bytes,
                eviction_key: entry.value().eviction_key(policy),
            });
        }

        for entry in self.dir_resolved.iter() {
            entries.push(MemoryEvictionEntry {
                kind: MemoryCacheKind::DirResolved,
                key: entry.key().clone(),
                size_bytes: entry.value().size_bytes,
                eviction_key: entry.value().eviction_key(policy),
            });
        }

        for entry in self.dir_analyzed.iter() {
            entries.push(MemoryEvictionEntry {
                kind: MemoryCacheKind::DirAnalyzed,
                key: entry.key().clone(),
                size_bytes: entry.value().size_bytes,
                eviction_key: entry.value().eviction_key(policy),
            });
        }

        for entry in self.dir_patched.iter() {
            entries.push(MemoryEvictionEntry {
                kind: MemoryCacheKind::DirPatched,
                key: entry.key().clone(),
                size_bytes: entry.value().size_bytes,
                eviction_key: entry.value().eviction_key(policy),
            });
        }

        for entry in self.mir.iter() {
            entries.push(MemoryEvictionEntry {
                kind: MemoryCacheKind::Mir,
                key: entry.key().clone(),
                size_bytes: entry.value().size_bytes,
                eviction_key: entry.value().eviction_key(policy),
            });
        }
    }

    /// Record memory size additions.
    fn increment_memory_size(&self, size_bytes: u64) {
        if size_bytes == 0 {
            return;
        }
        self.memory_state
            .size_bytes
            .fetch_add(size_bytes, Ordering::Relaxed);
    }

    /// Record memory size removals.
    fn decrement_memory_size(&self, size_bytes: u64) {
        if size_bytes == 0 {
            return;
        }

        let mut current = self.memory_state.size_bytes.load(Ordering::Relaxed);
        loop {
            let next = current.saturating_sub(size_bytes);
            match self.memory_state.size_bytes.compare_exchange(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(updated) => current = updated,
            }
        }
    }
}
