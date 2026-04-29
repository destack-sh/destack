use destack_heap::{
    DEFAULT_ALLOCATOR_CHUNK_BYTES, DEFAULT_GC_GROWTH_PERCENT, DEFAULT_GC_MINIMUM_HEAP_BYTES,
    DEFAULT_GC_TRIGGER_PERCENT, DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES, DEFAULT_PAGE_BYTES,
    DEFAULT_SHARED_SMALL_BYTES, DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES, DEFAULT_SMALL_BYTES,
    DEFAULT_SPACE_BYTES, DEFAULT_YOUNG_BYTES,
};
use serde::{Deserialize, Serialize};

/// Runtime heap size-class configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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
pub struct HeapGcOptions {
    /// Local-heap collector pacing.
    pub local: LocalGcOptions,
    /// Shared-heap collector pacing.
    pub shared: SharedGcOptions,
}

/// Runtime local-heap collector pacing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
pub struct HeapLimitOptions {
    /// Local-heap hard limits.
    pub local: LocalHeapLimitOptions,
    /// Shared-heap hard limits.
    pub shared: SharedHeapLimitOptions,
}

/// Hard limits for one local heap.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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
pub struct HeapOptions {
    /// Garbage-collection policy.
    pub gc: HeapGcOptions,
    /// Hard limits.
    pub limit: HeapLimitOptions,
    /// Layout and allocator geometry.
    pub layout: HeapLayoutOptions,
}

/// Runtime allocator configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeapAllocatorOptionsJson {
    /// The configured size-class table for small allocations.
    pub size_classes: Option<HeapSizeClassesJson>,
    /// The byte width for heap young space.
    pub young_bytes: Option<usize>,
    /// The maximum payload size admitted into heap young space.
    pub max_young_allocation_bytes: Option<usize>,
    /// The byte width for heap small-allocation spans.
    pub span_bytes: Option<usize>,
    /// The byte width for shared heap worker-allocation spans.
    pub shared_span_bytes: Option<usize>,
    /// The byte width for raw small-allocation spans.
    pub raw_span_bytes: Option<usize>,
    /// The virtual byte capacity for managed heap space.
    pub heap_space_bytes: Option<usize>,
    /// The virtual byte capacity for raw heap space.
    pub raw_space_bytes: Option<usize>,
    /// The byte width for heap pages and page-sized chunks.
    pub page_bytes: Option<usize>,
    /// The byte width for one allocator chunk.
    pub chunk_bytes: Option<usize>,
    /// The required alignment for configured small-allocation classes.
    pub small_alignment_bytes: Option<usize>,
}

impl HeapAllocatorOptionsJson {
    /// Inherit unset allocator settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.size_classes.is_none() {
            self.size_classes = parent.size_classes.clone();
        }

        if self.young_bytes.is_none() {
            self.young_bytes = parent.young_bytes;
        }

        if self.max_young_allocation_bytes.is_none() {
            self.max_young_allocation_bytes = parent.max_young_allocation_bytes;
        }

        if self.span_bytes.is_none() {
            self.span_bytes = parent.span_bytes;
        }

        if self.shared_span_bytes.is_none() {
            self.shared_span_bytes = parent.shared_span_bytes;
        }

        if self.raw_span_bytes.is_none() {
            self.raw_span_bytes = parent.raw_span_bytes;
        }

        if self.heap_space_bytes.is_none() {
            self.heap_space_bytes = parent.heap_space_bytes;
        }

        if self.raw_space_bytes.is_none() {
            self.raw_space_bytes = parent.raw_space_bytes;
        }

        if self.page_bytes.is_none() {
            self.page_bytes = parent.page_bytes;
        }

        if self.chunk_bytes.is_none() {
            self.chunk_bytes = parent.chunk_bytes;
        }

        if self.small_alignment_bytes.is_none() {
            self.small_alignment_bytes = parent.small_alignment_bytes;
        }
    }

    /// Apply allocator overrides to one base set of options.
    pub fn apply_to(&self, options: &mut HeapLayoutOptions) {
        if let Some(size_classes) = &self.size_classes {
            options.size_classes = size_classes.to_options();
        }

        if let Some(young_bytes) = self.young_bytes {
            options.heap_young_bytes = young_bytes;
        }

        if let Some(max_young_allocation_bytes) = self.max_young_allocation_bytes {
            options.max_heap_young_allocation_bytes = max_young_allocation_bytes;
        }

        if let Some(span_bytes) = self.span_bytes {
            options.heap_span_bytes = span_bytes;
        }

        if let Some(shared_span_bytes) = self.shared_span_bytes {
            options.shared_heap_span_bytes = shared_span_bytes;
        }

        if let Some(raw_span_bytes) = self.raw_span_bytes {
            options.raw_span_bytes = raw_span_bytes;
        }

        if let Some(heap_space_bytes) = self.heap_space_bytes {
            options.heap_space_bytes = heap_space_bytes;
        }

        if let Some(raw_space_bytes) = self.raw_space_bytes {
            options.raw_space_bytes = raw_space_bytes;
        }

        if let Some(page_bytes) = self.page_bytes {
            options.page_bytes = page_bytes;
        }

        if let Some(chunk_bytes) = self.chunk_bytes {
            options.chunk_bytes = chunk_bytes;
        }

        if let Some(small_alignment_bytes) = self.small_alignment_bytes {
            options.small_alignment_bytes = small_alignment_bytes;
        }
    }
}

/// Runtime heap size-class configuration for JSON deserialization.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum HeapSizeClassesJson {
    /// One named built-in size-class table.
    Named(String),
    /// One explicit size-class table.
    Explicit(Vec<usize>),
}

impl HeapSizeClassesJson {
    /// Convert this JSON configuration into runtime options.
    pub fn to_options(&self) -> HeapSizeClasses {
        match self {
            Self::Named(name) if name == "default" => HeapSizeClasses::Default,
            Self::Named(name) => HeapSizeClasses::Named(name.clone()),
            Self::Explicit(classes) => HeapSizeClasses::Explicit(classes.clone()),
        }
    }
}

/// Runtime heap-space policy for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeapSpaceOptionsJson {
    /// Heap growth target percentage.
    pub growth_percent: Option<u32>,
    /// Soft memory limit in bytes.
    pub memory_limit_bytes: Option<u64>,
    /// Hard memory limit in bytes.
    pub hard_limit_bytes: Option<u64>,
}

impl HeapSpaceOptionsJson {
    /// Inherit unset heap-space settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.growth_percent.is_none() {
            self.growth_percent = parent.growth_percent;
        }

        if self.memory_limit_bytes.is_none() {
            self.memory_limit_bytes = parent.memory_limit_bytes;
        }

        if self.hard_limit_bytes.is_none() {
            self.hard_limit_bytes = parent.hard_limit_bytes;
        }
    }

    /// Apply heap-space overrides to one base set of options.
    pub fn apply_to_local(&self, options: &mut HeapOptions) {
        if let Some(growth_percent) = self.growth_percent {
            options.gc.local.growth_percent = growth_percent;
        }

        if let Some(memory_limit_bytes) = self.memory_limit_bytes {
            options.gc.local.memory_limit_bytes = Some(memory_limit_bytes);
        }

        if let Some(hard_limit_bytes) = self.hard_limit_bytes {
            options.limit.local.max_bytes = Some(hard_limit_bytes);
        }
    }

    /// Apply heap-space overrides to one base set of options.
    pub fn apply_to_shared(&self, options: &mut HeapOptions) {
        if let Some(growth_percent) = self.growth_percent {
            options.gc.shared.growth_percent = growth_percent;
        }

        if let Some(memory_limit_bytes) = self.memory_limit_bytes {
            options.gc.shared.memory_limit_bytes = Some(memory_limit_bytes);
        }

        if let Some(hard_limit_bytes) = self.hard_limit_bytes {
            options.limit.shared.max_bytes = Some(hard_limit_bytes);
        }
    }
}

/// Runtime heap configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeapOptionsJson {
    /// Local-heap policy.
    pub local: Option<HeapSpaceOptionsJson>,
    /// Shared-heap policy.
    pub shared: Option<HeapSpaceOptionsJson>,
    /// Allocator sizing policy.
    pub allocator: Option<HeapAllocatorOptionsJson>,
}

impl HeapOptionsJson {
    /// Inherit unset heap settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.local.is_none() {
            self.local = parent.local.clone();
        } else if let (Some(local), Some(parent_local)) = (&mut self.local, &parent.local) {
            local.extend_from(parent_local);
        }

        if self.shared.is_none() {
            self.shared = parent.shared.clone();
        } else if let (Some(shared), Some(parent_shared)) = (&mut self.shared, &parent.shared) {
            shared.extend_from(parent_shared);
        }

        if self.allocator.is_none() {
            self.allocator = parent.allocator.clone();
        } else if let (Some(allocator), Some(parent_allocator)) =
            (&mut self.allocator, &parent.allocator)
        {
            allocator.extend_from(parent_allocator);
        }
    }

    /// Apply heap overrides to a base set of options.
    pub fn apply_to(&self, options: &mut HeapOptions) {
        if let Some(local) = &self.local {
            local.apply_to_local(options);
        }

        if let Some(shared) = &self.shared {
            shared.apply_to_shared(options);
        }

        if let Some(allocator) = &self.allocator {
            allocator.apply_to(&mut options.layout);
        }
    }
}
