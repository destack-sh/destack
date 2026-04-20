use serde::{Deserialize, Serialize};

/// The default heap growth target after one cycle.
const DEFAULT_HEAP_GROWTH_PERCENT: u32 = 100;
/// The default built-in managed young-space width.
const DEFAULT_MANAGED_YOUNG_BYTES: usize = 64 * 1024;
/// The default maximum payload size admitted into managed young space.
const DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES: usize = 4 * 1024;
/// The default managed small-allocation span width.
const DEFAULT_MANAGED_SPAN_BYTES: usize = 16 * 1024;
/// The default raw small-allocation span width.
const DEFAULT_RAW_SPAN_BYTES: usize = 16 * 1024;
/// The default heap page width.
const DEFAULT_PAGE_BYTES: usize = 4 * 1024;
/// The default heap segment width.
const DEFAULT_SEGMENT_BYTES: usize = 1024 * 1024;
/// The default remembered-card width.
const DEFAULT_CARD_BYTES: usize = 256;
/// The default small-allocation alignment.
const DEFAULT_SMALL_ALIGNMENT_BYTES: usize = 8;

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeapGcOptions {
    /// Local-heap collector pacing.
    pub local: LocalGcOptions,
    /// Shared-heap collector pacing.
    pub shared: SharedGcOptions,
}

impl Default for HeapGcOptions {
    fn default() -> Self {
        Self {
            local: LocalGcOptions::default(),
            shared: SharedGcOptions::default(),
        }
    }
}

/// Runtime local-heap collector pacing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalGcOptions {
    /// Go-style heap growth target percentage.
    pub growth_percent: u32,
    /// Go-style soft memory limit in bytes.
    pub memory_limit_bytes: Option<u64>,
}

impl Default for LocalGcOptions {
    fn default() -> Self {
        Self {
            growth_percent: DEFAULT_HEAP_GROWTH_PERCENT,
            memory_limit_bytes: None,
        }
    }
}

/// Runtime shared-heap collector pacing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SharedGcOptions {
    /// Go-style heap growth target percentage.
    pub growth_percent: u32,
    /// Go-style soft memory limit in bytes.
    pub memory_limit_bytes: Option<u64>,
}

impl Default for SharedGcOptions {
    fn default() -> Self {
        Self {
            growth_percent: DEFAULT_HEAP_GROWTH_PERCENT,
            memory_limit_bytes: None,
        }
    }
}

/// Runtime heap hard-limit configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeapLimitOptions {
    /// Local-heap hard limits.
    pub local: LocalHeapLimitOptions,
    /// Shared-heap hard limits.
    pub shared: SharedHeapLimitOptions,
}

impl Default for HeapLimitOptions {
    fn default() -> Self {
        Self {
            local: LocalHeapLimitOptions::default(),
            shared: SharedHeapLimitOptions::default(),
        }
    }
}

/// Hard limits for one local heap.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct LocalHeapLimitOptions {
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained managed heap bytes.
    pub managed_max_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub raw_max_bytes: Option<u64>,
}

/// Hard limits for one shared heap.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct SharedHeapLimitOptions {
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained managed heap bytes.
    pub managed_max_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub raw_max_bytes: Option<u64>,
}

/// Runtime heap layout and allocator geometry configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeapLayoutOptions {
    /// The configured size-class table for small allocations.
    pub size_classes: HeapSizeClasses,
    /// The byte width for managed young space.
    pub managed_young_bytes: usize,
    /// The maximum payload size admitted into managed young space.
    pub max_managed_young_allocation_bytes: usize,
    /// The byte width for managed small-allocation spans.
    pub managed_span_bytes: usize,
    /// The byte width for raw small-allocation spans.
    pub raw_span_bytes: usize,
    /// The byte width for heap pages and page-sized chunks.
    pub page_bytes: usize,
    /// The byte width for one physical segment.
    pub segment_bytes: usize,
    /// The byte width for one remembered card.
    pub card_bytes: usize,
    /// The required alignment for configured small-allocation classes.
    pub small_alignment_bytes: usize,
}

impl Default for HeapLayoutOptions {
    fn default() -> Self {
        Self {
            size_classes: HeapSizeClasses::Default,
            managed_young_bytes: DEFAULT_MANAGED_YOUNG_BYTES,
            max_managed_young_allocation_bytes: DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES,
            managed_span_bytes: DEFAULT_MANAGED_SPAN_BYTES,
            raw_span_bytes: DEFAULT_RAW_SPAN_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            segment_bytes: DEFAULT_SEGMENT_BYTES,
            card_bytes: DEFAULT_CARD_BYTES,
            small_alignment_bytes: DEFAULT_SMALL_ALIGNMENT_BYTES,
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

/// Runtime local-heap collector pacing for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LocalGcOptionsJson {
    /// Heap growth target percentage.
    pub growth_percent: Option<u32>,
    /// Soft memory limit in bytes.
    pub memory_limit_bytes: Option<u64>,
}

impl LocalGcOptionsJson {
    /// Inherit unset GC settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.growth_percent.is_none() {
            self.growth_percent = parent.growth_percent;
        }

        if self.memory_limit_bytes.is_none() {
            self.memory_limit_bytes = parent.memory_limit_bytes;
        }
    }

    /// Apply GC overrides to one base set of options.
    pub fn apply_to(&self, options: &mut LocalGcOptions) {
        if let Some(growth_percent) = self.growth_percent {
            options.growth_percent = growth_percent;
        }

        if let Some(memory_limit_bytes) = self.memory_limit_bytes {
            options.memory_limit_bytes = Some(memory_limit_bytes);
        }
    }
}

/// Runtime shared-heap collector pacing for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SharedGcOptionsJson {
    /// Heap growth target percentage.
    pub growth_percent: Option<u32>,
    /// Soft memory limit in bytes.
    pub memory_limit_bytes: Option<u64>,
}

impl SharedGcOptionsJson {
    /// Inherit unset GC settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.growth_percent.is_none() {
            self.growth_percent = parent.growth_percent;
        }

        if self.memory_limit_bytes.is_none() {
            self.memory_limit_bytes = parent.memory_limit_bytes;
        }
    }

    /// Apply GC overrides to one base set of options.
    pub fn apply_to(&self, options: &mut SharedGcOptions) {
        if let Some(growth_percent) = self.growth_percent {
            options.growth_percent = growth_percent;
        }

        if let Some(memory_limit_bytes) = self.memory_limit_bytes {
            options.memory_limit_bytes = Some(memory_limit_bytes);
        }
    }
}

/// Runtime heap GC configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeapGcOptionsJson {
    /// Local-heap collector pacing.
    pub local: Option<LocalGcOptionsJson>,
    /// Shared-heap collector pacing.
    pub shared: Option<SharedGcOptionsJson>,
}

impl HeapGcOptionsJson {
    /// Inherit unset GC settings from one parent config.
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
    }

    /// Apply GC overrides to one base set of options.
    pub fn apply_to(&self, options: &mut HeapGcOptions) {
        if let Some(local) = &self.local {
            local.apply_to(&mut options.local);
        }

        if let Some(shared) = &self.shared {
            shared.apply_to(&mut options.shared);
        }
    }
}

/// Runtime local-heap hard limits for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LocalHeapLimitOptionsJson {
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained managed heap bytes.
    pub managed_max_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub raw_max_bytes: Option<u64>,
}

impl LocalHeapLimitOptionsJson {
    /// Inherit unset limit settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.max_bytes.is_none() {
            self.max_bytes = parent.max_bytes;
        }

        if self.managed_max_bytes.is_none() {
            self.managed_max_bytes = parent.managed_max_bytes;
        }

        if self.raw_max_bytes.is_none() {
            self.raw_max_bytes = parent.raw_max_bytes;
        }
    }

    /// Apply limit overrides to one base set of options.
    pub fn apply_to(&self, options: &mut LocalHeapLimitOptions) {
        if let Some(max_bytes) = self.max_bytes {
            options.max_bytes = Some(max_bytes);
        }

        if let Some(managed_max_bytes) = self.managed_max_bytes {
            options.managed_max_bytes = Some(managed_max_bytes);
        }

        if let Some(raw_max_bytes) = self.raw_max_bytes {
            options.raw_max_bytes = Some(raw_max_bytes);
        }
    }
}

/// Runtime shared-heap hard limits for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SharedHeapLimitOptionsJson {
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained managed heap bytes.
    pub managed_max_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub raw_max_bytes: Option<u64>,
}

impl SharedHeapLimitOptionsJson {
    /// Inherit unset limit settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.max_bytes.is_none() {
            self.max_bytes = parent.max_bytes;
        }

        if self.managed_max_bytes.is_none() {
            self.managed_max_bytes = parent.managed_max_bytes;
        }

        if self.raw_max_bytes.is_none() {
            self.raw_max_bytes = parent.raw_max_bytes;
        }
    }

    /// Apply limit overrides to one base set of options.
    pub fn apply_to(&self, options: &mut SharedHeapLimitOptions) {
        if let Some(max_bytes) = self.max_bytes {
            options.max_bytes = Some(max_bytes);
        }

        if let Some(managed_max_bytes) = self.managed_max_bytes {
            options.managed_max_bytes = Some(managed_max_bytes);
        }

        if let Some(raw_max_bytes) = self.raw_max_bytes {
            options.raw_max_bytes = Some(raw_max_bytes);
        }
    }
}

/// Runtime heap hard-limit configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeapLimitOptionsJson {
    /// Local-heap hard limits.
    pub local: Option<LocalHeapLimitOptionsJson>,
    /// Shared-heap hard limits.
    pub shared: Option<SharedHeapLimitOptionsJson>,
}

impl HeapLimitOptionsJson {
    /// Inherit unset limit settings from one parent config.
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
    }

    /// Apply limit overrides to one base set of options.
    pub fn apply_to(&self, options: &mut HeapLimitOptions) {
        if let Some(local) = &self.local {
            local.apply_to(&mut options.local);
        }

        if let Some(shared) = &self.shared {
            shared.apply_to(&mut options.shared);
        }
    }
}

/// Runtime heap layout configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeapLayoutOptionsJson {
    /// The configured size-class table for small allocations.
    pub size_classes: Option<HeapSizeClassesJson>,
    /// The byte width for managed young space.
    pub managed_young_bytes: Option<usize>,
    /// The maximum payload size admitted into managed young space.
    pub max_managed_young_allocation_bytes: Option<usize>,
    /// The byte width for managed small-allocation spans.
    pub managed_span_bytes: Option<usize>,
    /// The byte width for raw small-allocation spans.
    pub raw_span_bytes: Option<usize>,
    /// The byte width for heap pages and page-sized chunks.
    pub page_bytes: Option<usize>,
    /// The byte width for one physical segment.
    pub segment_bytes: Option<usize>,
    /// The byte width for one remembered card.
    pub card_bytes: Option<usize>,
    /// The required alignment for configured small-allocation classes.
    pub small_alignment_bytes: Option<usize>,
}

impl HeapLayoutOptionsJson {
    /// Inherit unset layout settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.size_classes.is_none() {
            self.size_classes = parent.size_classes.clone();
        }

        if self.managed_young_bytes.is_none() {
            self.managed_young_bytes = parent.managed_young_bytes;
        }

        if self.max_managed_young_allocation_bytes.is_none() {
            self.max_managed_young_allocation_bytes = parent.max_managed_young_allocation_bytes;
        }

        if self.managed_span_bytes.is_none() {
            self.managed_span_bytes = parent.managed_span_bytes;
        }

        if self.raw_span_bytes.is_none() {
            self.raw_span_bytes = parent.raw_span_bytes;
        }

        if self.page_bytes.is_none() {
            self.page_bytes = parent.page_bytes;
        }

        if self.segment_bytes.is_none() {
            self.segment_bytes = parent.segment_bytes;
        }

        if self.card_bytes.is_none() {
            self.card_bytes = parent.card_bytes;
        }

        if self.small_alignment_bytes.is_none() {
            self.small_alignment_bytes = parent.small_alignment_bytes;
        }
    }

    /// Apply layout overrides to one base set of options.
    pub fn apply_to(&self, options: &mut HeapLayoutOptions) {
        if let Some(size_classes) = &self.size_classes {
            options.size_classes = size_classes.to_options();
        }

        if let Some(managed_young_bytes) = self.managed_young_bytes {
            options.managed_young_bytes = managed_young_bytes;
        }

        if let Some(max_managed_young_allocation_bytes) = self.max_managed_young_allocation_bytes {
            options.max_managed_young_allocation_bytes = max_managed_young_allocation_bytes;
        }

        if let Some(managed_span_bytes) = self.managed_span_bytes {
            options.managed_span_bytes = managed_span_bytes;
        }

        if let Some(raw_span_bytes) = self.raw_span_bytes {
            options.raw_span_bytes = raw_span_bytes;
        }

        if let Some(page_bytes) = self.page_bytes {
            options.page_bytes = page_bytes;
        }

        if let Some(segment_bytes) = self.segment_bytes {
            options.segment_bytes = segment_bytes;
        }

        if let Some(card_bytes) = self.card_bytes {
            options.card_bytes = card_bytes;
        }

        if let Some(small_alignment_bytes) = self.small_alignment_bytes {
            options.small_alignment_bytes = small_alignment_bytes;
        }
    }
}

/// Runtime heap configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeapOptionsJson {
    /// Garbage-collection policy.
    pub gc: Option<HeapGcOptionsJson>,
    /// Hard limits.
    pub limit: Option<HeapLimitOptionsJson>,
    /// Layout and allocator geometry.
    pub layout: Option<HeapLayoutOptionsJson>,
}

impl HeapOptionsJson {
    /// Inherit unset heap settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.gc.is_none() {
            self.gc = parent.gc.clone();
        } else if let (Some(gc), Some(parent_gc)) = (&mut self.gc, &parent.gc) {
            gc.extend_from(parent_gc);
        }

        if self.limit.is_none() {
            self.limit = parent.limit.clone();
        } else if let (Some(limit), Some(parent_limit)) = (&mut self.limit, &parent.limit) {
            limit.extend_from(parent_limit);
        }

        if self.layout.is_none() {
            self.layout = parent.layout.clone();
        } else if let (Some(layout), Some(parent_layout)) = (&mut self.layout, &parent.layout) {
            layout.extend_from(parent_layout);
        }
    }

    /// Apply heap overrides to a base set of options.
    pub fn apply_to(&self, options: &mut HeapOptions) {
        if let Some(gc) = &self.gc {
            gc.apply_to(&mut options.gc);
        }

        if let Some(limit) = &self.limit {
            limit.apply_to(&mut options.limit);
        }

        if let Some(layout) = &self.layout {
            layout.apply_to(&mut options.layout);
        }
    }
}
