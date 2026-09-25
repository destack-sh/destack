use serde::{Deserialize, Serialize};
use tspp_heap as heap;
use tspp_heap::{
    DEFAULT_GC_GROWTH_PERCENT, DEFAULT_GC_MINIMUM_HEAP_BYTES, DEFAULT_GC_MINIMUM_WORK_BYTES,
    DEFAULT_GC_TRIGGER_PERCENT,
};
use tspp_serde::Reflect;

/// Runtime heap configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HeapOptions {
    /// Soft memory limit inherited by local and shared heap collectors.
    pub memory_limit_bytes: Option<u64>,
    /// The proportional heap growth target percentage.
    pub growth_percent: u32,
    /// Worker-local heap policy.
    pub local: LocalHeapOptions,
    /// Runtime-shared heap policy.
    pub shared: SharedHeapOptions,
}

impl Default for HeapOptions {
    fn default() -> Self {
        Self {
            memory_limit_bytes: None,
            growth_percent: DEFAULT_GC_GROWTH_PERCENT,
            local: LocalHeapOptions::default(),
            shared: SharedHeapOptions::default(),
        }
    }
}

impl HeapOptions {
    /// Resolve worker-local heap construction options.
    pub fn local_heap_options(&self) -> Result<heap::HeapOptions, heap::HeapError> {
        let options = heap::HeapOptions {
            gc: self.local_gc_options(),
            ..heap::HeapOptions::local()
        };

        options.validate_local()?;

        Ok(options)
    }

    /// Resolve runtime-shared heap construction options.
    pub fn shared_heap_options(&self) -> Result<heap::SharedHeapOptions, heap::HeapError> {
        let options = heap::SharedHeapOptions {
            gc: self.shared_gc_options(),
            ..heap::SharedHeapOptions::default()
        };

        options.validate()?;

        Ok(options)
    }

    /// Resolve local collector policy.
    fn local_gc_options(&self) -> heap::GcOptions {
        heap::GcOptions {
            growth_percent: self.growth_percent,
            trigger_percent: DEFAULT_GC_TRIGGER_PERCENT,
            soft_limit_bytes: self.memory_limit_bytes,
            minimum_heap_bytes: Some(self.local.minimum_heap_bytes()),
            minimum_work_bytes: DEFAULT_GC_MINIMUM_WORK_BYTES,
        }
    }

    /// Resolve shared collector policy.
    fn shared_gc_options(&self) -> heap::GcOptions {
        heap::GcOptions {
            growth_percent: self.growth_percent,
            trigger_percent: DEFAULT_GC_TRIGGER_PERCENT,
            soft_limit_bytes: self.memory_limit_bytes,
            minimum_heap_bytes: Some(self.shared.minimum_heap_bytes()),
            minimum_work_bytes: DEFAULT_GC_MINIMUM_WORK_BYTES,
        }
    }
}

/// Runtime local-heap policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LocalHeapOptions {
    /// Minimum heap bytes before normal growth pacing applies.
    pub min_bytes: Option<u64>,
    /// Hard limit for total retained local heap bytes.
    pub max_bytes: Option<u64>,
}

impl LocalHeapOptions {
    /// Return the initial limits for one worker-local heap.
    pub const fn limits(&self) -> heap::HeapLimits {
        heap::HeapLimits {
            max_bytes: self.max_bytes,
            retained_bytes: None,
        }
    }

    /// Resolve the minimum heap byte budget.
    fn minimum_heap_bytes(&self) -> u64 {
        self.min_bytes.unwrap_or(DEFAULT_GC_MINIMUM_HEAP_BYTES)
    }
}

/// Runtime shared-heap policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct SharedHeapOptions {
    /// Minimum heap bytes before normal growth pacing applies.
    pub min_bytes: Option<u64>,
    /// Hard limit for total retained shared heap bytes.
    pub max_bytes: Option<u64>,
}

impl SharedHeapOptions {
    /// Return the initial limits for one runtime-shared heap.
    pub const fn limits(&self) -> heap::SharedHeapLimits {
        heap::SharedHeapLimits {
            max_bytes: self.max_bytes,
            retained_bytes: None,
        }
    }

    /// Resolve the minimum heap byte budget.
    fn minimum_heap_bytes(&self) -> u64 {
        self.min_bytes.unwrap_or(DEFAULT_GC_MINIMUM_HEAP_BYTES)
    }
}
