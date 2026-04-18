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

/// Runtime heap configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeapOptions {
    /// Growth target percentage.
    pub growth_percent: u32,
    /// Soft heap limit in bytes.
    pub soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub initial_bytes: Option<u64>,
    /// The configured size-class table for small allocations.
    pub size_classes: HeapSizeClasses,
    /// The byte width for managed young space.
    pub managed_young_bytes: usize,
    /// The maximum payload size admitted into managed young space.
    pub max_managed_young_allocation_bytes: usize,
    /// The byte width for managed small-object spans.
    pub managed_small_bytes: usize,
    /// The byte width for raw small-object spans.
    pub raw_small_bytes: usize,
    /// The byte width for local heap pages and page-sized chunks.
    pub page_bytes: usize,
    /// The byte width for one physical arena segment.
    pub arena_segment_bytes: usize,
    /// The byte width for one remembered card.
    pub card_bytes: usize,
    /// The required alignment for configured small-allocation classes.
    pub small_allocation_alignment_bytes: usize,
    /// The entry count per heap metadata table chunk.
    pub table_chunk_len: usize,
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained managed heap bytes.
    pub max_managed_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub max_raw_bytes: Option<u64>,
    /// Hard limit for retained shared-memory bytes.
    pub max_shared_bytes: Option<u64>,
}

impl Default for HeapOptions {
    fn default() -> Self {
        Self {
            growth_percent: 100,
            soft_limit_bytes: None,
            initial_bytes: None,
            size_classes: HeapSizeClasses::Default,
            managed_young_bytes: 64 * 1024,
            max_managed_young_allocation_bytes: 4 * 1024,
            managed_small_bytes: 16 * 1024,
            raw_small_bytes: 16 * 1024,
            page_bytes: 4 * 1024,
            arena_segment_bytes: 1024 * 1024,
            card_bytes: 256,
            small_allocation_alignment_bytes: 8,
            table_chunk_len: 256,
            max_bytes: None,
            max_managed_bytes: None,
            max_raw_bytes: None,
            max_shared_bytes: None,
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

/// Runtime heap configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeapOptionsJson {
    /// Growth target percentage.
    pub growth_percent: Option<u32>,
    /// Soft heap limit in bytes.
    pub soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub initial_bytes: Option<u64>,
    /// The configured size-class table for small allocations.
    pub size_classes: Option<HeapSizeClassesJson>,
    /// The byte width for managed young space.
    pub managed_young_bytes: Option<usize>,
    /// The maximum payload size admitted into managed young space.
    pub max_managed_young_allocation_bytes: Option<usize>,
    /// The byte width for managed small-object spans.
    pub managed_small_bytes: Option<usize>,
    /// The byte width for raw small-object spans.
    pub raw_small_bytes: Option<usize>,
    /// The byte width for local heap pages and page-sized chunks.
    pub page_bytes: Option<usize>,
    /// The byte width for one physical arena segment.
    pub arena_segment_bytes: Option<usize>,
    /// The byte width for one remembered card.
    pub card_bytes: Option<usize>,
    /// The required alignment for configured small-allocation classes.
    pub small_allocation_alignment_bytes: Option<usize>,
    /// The entry count per heap metadata table chunk.
    pub table_chunk_len: Option<usize>,
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained managed heap bytes.
    pub max_managed_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub max_raw_bytes: Option<u64>,
    /// Hard limit for retained shared-memory bytes.
    pub max_shared_bytes: Option<u64>,
}

impl HeapOptionsJson {
    /// Inherit unset heap settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.growth_percent.is_none() {
            self.growth_percent = parent.growth_percent;
        }
        if self.soft_limit_bytes.is_none() {
            self.soft_limit_bytes = parent.soft_limit_bytes;
        }
        if self.initial_bytes.is_none() {
            self.initial_bytes = parent.initial_bytes;
        }
        if self.size_classes.is_none() {
            self.size_classes = parent.size_classes.clone();
        }
        if self.managed_young_bytes.is_none() {
            self.managed_young_bytes = parent.managed_young_bytes;
        }
        if self.max_managed_young_allocation_bytes.is_none() {
            self.max_managed_young_allocation_bytes = parent.max_managed_young_allocation_bytes;
        }
        if self.managed_small_bytes.is_none() {
            self.managed_small_bytes = parent.managed_small_bytes;
        }
        if self.raw_small_bytes.is_none() {
            self.raw_small_bytes = parent.raw_small_bytes;
        }
        if self.page_bytes.is_none() {
            self.page_bytes = parent.page_bytes;
        }
        if self.arena_segment_bytes.is_none() {
            self.arena_segment_bytes = parent.arena_segment_bytes;
        }
        if self.card_bytes.is_none() {
            self.card_bytes = parent.card_bytes;
        }
        if self.small_allocation_alignment_bytes.is_none() {
            self.small_allocation_alignment_bytes = parent.small_allocation_alignment_bytes;
        }
        if self.table_chunk_len.is_none() {
            self.table_chunk_len = parent.table_chunk_len;
        }
        if self.max_bytes.is_none() {
            self.max_bytes = parent.max_bytes;
        }
        if self.max_managed_bytes.is_none() {
            self.max_managed_bytes = parent.max_managed_bytes;
        }
        if self.max_raw_bytes.is_none() {
            self.max_raw_bytes = parent.max_raw_bytes;
        }
        if self.max_shared_bytes.is_none() {
            self.max_shared_bytes = parent.max_shared_bytes;
        }
    }

    /// Apply heap overrides to a base set of options.
    pub fn apply_to(&self, options: &mut HeapOptions) {
        // apply pacing overrides
        if let Some(growth_percent) = self.growth_percent {
            options.growth_percent = growth_percent;
        }

        // apply memory limit overrides
        if let Some(soft_limit_bytes) = self.soft_limit_bytes {
            options.soft_limit_bytes = Some(soft_limit_bytes);
        }
        if let Some(initial_bytes) = self.initial_bytes {
            options.initial_bytes = Some(initial_bytes);
        }
        if let Some(size_classes) = &self.size_classes {
            options.size_classes = size_classes.to_options();
        }
        if let Some(managed_young_bytes) = self.managed_young_bytes {
            options.managed_young_bytes = managed_young_bytes;
        }
        if let Some(max_managed_young_allocation_bytes) = self.max_managed_young_allocation_bytes {
            options.max_managed_young_allocation_bytes = max_managed_young_allocation_bytes;
        }
        if let Some(managed_small_bytes) = self.managed_small_bytes {
            options.managed_small_bytes = managed_small_bytes;
        }
        if let Some(raw_small_bytes) = self.raw_small_bytes {
            options.raw_small_bytes = raw_small_bytes;
        }
        if let Some(page_bytes) = self.page_bytes {
            options.page_bytes = page_bytes;
        }
        if let Some(arena_segment_bytes) = self.arena_segment_bytes {
            options.arena_segment_bytes = arena_segment_bytes;
        }
        if let Some(card_bytes) = self.card_bytes {
            options.card_bytes = card_bytes;
        }
        if let Some(small_allocation_alignment_bytes) = self.small_allocation_alignment_bytes {
            options.small_allocation_alignment_bytes = small_allocation_alignment_bytes;
        }
        if let Some(table_chunk_len) = self.table_chunk_len {
            options.table_chunk_len = table_chunk_len;
        }

        // apply hard limit overrides
        if let Some(max_bytes) = self.max_bytes {
            options.max_bytes = Some(max_bytes);
        }
        if let Some(max_managed_bytes) = self.max_managed_bytes {
            options.max_managed_bytes = Some(max_managed_bytes);
        }
        if let Some(max_raw_bytes) = self.max_raw_bytes {
            options.max_raw_bytes = Some(max_raw_bytes);
        }
        if let Some(max_shared_bytes) = self.max_shared_bytes {
            options.max_shared_bytes = Some(max_shared_bytes);
        }
    }
}
