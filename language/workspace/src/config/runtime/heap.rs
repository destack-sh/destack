use destack_heap::{
    DEFAULT_ALLOCATOR_CHUNK_BYTES, DEFAULT_GC_GROWTH_PERCENT, DEFAULT_GC_MINIMUM_HEAP_BYTES,
    DEFAULT_GC_TRIGGER_PERCENT, DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES, DEFAULT_PAGE_BYTES,
    DEFAULT_SHARED_SMALL_BYTES, DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES, DEFAULT_SMALL_BYTES,
    DEFAULT_SPACE_BYTES, DEFAULT_YOUNG_BYTES,
};
use serde::{Deserialize, Serialize};

/// Runtime heap size-class configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum HeapSizeClasses {
    /// The built-in default size-class table.
    #[default]
    Default,
    /// One named built-in size-class table.
    Named(String),
    /// One explicit size-class table in bytes.
    Explicit(Vec<usize>),
}

/// Runtime heap garbage-collection configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HeapGcOptions {
    /// Local-heap collector pacing.
    pub local: LocalGcOptions,
    /// Shared-heap collector pacing.
    pub shared: SharedGcOptions,
}

/// Runtime local-heap collector pacing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LocalGcOptions {
    /// The proportional heap growth target percentage.
    pub growth_percent: u32,
    /// Heap trigger as a percentage of the current goal.
    pub trigger_percent: u32,
    /// Optional soft memory limit in bytes.
    pub memory_limit_bytes: Option<u64>,
    /// Minimum heap floor in bytes.
    pub minimum_heap_bytes: Option<u64>,
}

impl Default for LocalGcOptions {
    fn default() -> Self {
        Self {
            growth_percent: DEFAULT_GC_GROWTH_PERCENT,
            trigger_percent: DEFAULT_GC_TRIGGER_PERCENT,
            memory_limit_bytes: None,
            minimum_heap_bytes: Some(DEFAULT_GC_MINIMUM_HEAP_BYTES),
        }
    }
}

/// Runtime shared-heap collector pacing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct SharedGcOptions {
    /// The proportional heap growth target percentage.
    pub growth_percent: u32,
    /// Heap trigger as a percentage of the current goal.
    pub trigger_percent: u32,
    /// Optional soft memory limit in bytes.
    pub memory_limit_bytes: Option<u64>,
    /// Minimum heap floor in bytes.
    pub minimum_heap_bytes: Option<u64>,
}

impl Default for SharedGcOptions {
    fn default() -> Self {
        Self {
            growth_percent: DEFAULT_GC_GROWTH_PERCENT,
            trigger_percent: DEFAULT_GC_TRIGGER_PERCENT,
            memory_limit_bytes: None,
            minimum_heap_bytes: Some(DEFAULT_GC_MINIMUM_HEAP_BYTES),
        }
    }
}

/// Runtime heap hard-limit configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HeapLimitOptions {
    /// Local-heap hard limits.
    pub local: LocalHeapLimitOptions,
    /// Shared-heap hard limits.
    pub shared: SharedHeapLimitOptions,
}

/// Hard limits for one local heap.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LocalHeapLimitOptions {
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained heap bytes.
    pub heap_max_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub raw_max_bytes: Option<u64>,
}

/// Hard limits for one shared heap.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct SharedHeapLimitOptions {
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained heap bytes.
    pub heap_max_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub raw_max_bytes: Option<u64>,
}

/// Runtime heap layout and allocator geometry configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HeapLayoutOptions {
    /// The configured size-class table for small allocations.
    pub size_classes: HeapSizeClasses,
    /// The byte width for heap young space.
    pub heap_young_bytes: usize,
    /// The maximum payload size admitted into heap young space.
    pub max_heap_young_allocation_bytes: usize,
    /// The byte width for heap small-allocation spans.
    pub heap_span_bytes: usize,
    /// The byte width for shared heap worker-allocation spans.
    pub shared_heap_span_bytes: usize,
    /// The byte width for raw small-allocation spans.
    pub raw_span_bytes: usize,
    /// The virtual byte capacity for managed heap space.
    pub heap_space_bytes: usize,
    /// The virtual byte capacity for raw heap space.
    pub raw_space_bytes: usize,
    /// The byte width for heap pages and page-sized chunks.
    pub page_bytes: usize,
    /// The byte width for one allocator chunk.
    pub chunk_bytes: usize,
    /// The required alignment for configured small-allocation classes.
    pub small_alignment_bytes: usize,
}

impl Default for HeapLayoutOptions {
    fn default() -> Self {
        Self {
            size_classes: HeapSizeClasses::Default,
            heap_young_bytes: DEFAULT_YOUNG_BYTES,
            max_heap_young_allocation_bytes: DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES,
            heap_span_bytes: DEFAULT_SMALL_BYTES,
            shared_heap_span_bytes: DEFAULT_SHARED_SMALL_BYTES,
            raw_span_bytes: DEFAULT_SMALL_BYTES,
            heap_space_bytes: DEFAULT_SPACE_BYTES,
            raw_space_bytes: DEFAULT_SPACE_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            chunk_bytes: DEFAULT_ALLOCATOR_CHUNK_BYTES,
            small_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
        }
    }
}

/// Runtime heap configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HeapOptions {
    /// Garbage-collection policy.
    pub gc: HeapGcOptions,
    /// Hard limits.
    pub limit: HeapLimitOptions,
    /// Layout and allocator geometry.
    pub layout: HeapLayoutOptions,
}
