use serde::{Deserialize, Serialize};

use crate::arena::{Arena, SizeClassPolicy, SizeClassTable};
use crate::{GcOptions, HeapError};

/// The standard heap page width aligned to common OS pages.
const DEFAULT_PAGE_BYTES: usize = 4 * 1024;
/// The standard arena segment width that amortizes mapping and metadata work.
const DEFAULT_ARENA_SEGMENT_BYTES: usize = 1024 * 1024;
/// The standard small-span width for size-classed allocation.
const DEFAULT_SMALL_BYTES: usize = 16 * 1024;
/// The standard remembered-card width for local write tracking.
const DEFAULT_CARD_BYTES: usize = 256;
/// The standard small young-space width for worker-local heaps.
const DEFAULT_YOUNG_BYTES: usize = 64 * 1024;
/// The standard nursery bypass threshold for larger payloads.
const DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES: usize = 4 * 1024;
/// The standard byte width for managed references inside traced payloads.
const DEFAULT_MANAGED_REFERENCE_BYTES: u8 = 8;
/// The standard alignment for configured small-allocation classes.
const DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES: usize = 8;
/// The standard entry count per copy on write metadata table chunk.
const DEFAULT_TABLE_CHUNK_LEN: usize = 256;

/// Constructor policy for resolving local heap options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalHeapPolicy {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The small-object allocation policy.
    pub small: SizeClassPolicy,
    /// The encoded byte width for managed references inside traced payloads.
    pub managed_reference_bytes: u8,
    /// The byte width for managed young space.
    pub managed_young_bytes: usize,
    /// The maximum payload size admitted into managed young space.
    pub max_managed_young_allocation_bytes: usize,
    /// The byte width for managed small-allocation spans.
    pub managed_small_bytes: usize,
    /// The byte width for raw small-allocation spans.
    pub raw_small_bytes: usize,
    /// The byte width for local heap pages.
    pub page_bytes: usize,
    /// The byte width for one physical arena segment.
    pub arena_segment_bytes: usize,
    /// The byte width for one remembered card.
    pub card_bytes: usize,
    /// The entry count per copy on write metadata table chunk.
    pub table_chunk_len: usize,
}

impl Default for LocalHeapPolicy {
    fn default() -> Self {
        Self {
            gc: GcOptions::local(),
            small: SizeClassPolicy::default(),
            managed_reference_bytes: DEFAULT_MANAGED_REFERENCE_BYTES,
            managed_young_bytes: DEFAULT_YOUNG_BYTES,
            max_managed_young_allocation_bytes: DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES,
            managed_small_bytes: DEFAULT_SMALL_BYTES,
            raw_small_bytes: DEFAULT_SMALL_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            arena_segment_bytes: DEFAULT_ARENA_SEGMENT_BYTES,
            card_bytes: DEFAULT_CARD_BYTES,
            table_chunk_len: DEFAULT_TABLE_CHUNK_LEN,
        }
    }
}

impl LocalHeapPolicy {
    /// Resolve this constructor policy into heap options.
    pub fn resolve(&self) -> Result<HeapOptions, HeapError> {
        let options = HeapOptions {
            gc: self.gc,
            size_classes: self.small.size_classes()?,
            managed_reference_bytes: self.managed_reference_bytes,
            managed_young_bytes: self.managed_young_bytes,
            max_managed_young_allocation_bytes: self.max_managed_young_allocation_bytes,
            managed_small_bytes: self.managed_small_bytes,
            raw_small_bytes: self.raw_small_bytes,
            page_bytes: self.page_bytes,
            arena_segment_bytes: self.arena_segment_bytes,
            card_bytes: self.card_bytes,
            small_allocation_alignment_bytes: self.small.alignment_bytes,
            table_chunk_len: self.table_chunk_len,
        };

        options.validate_local()?;

        Ok(options)
    }
}

/// Constructor policy for resolving shared heap options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapPolicy {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The small-object allocation policy.
    pub small: SizeClassPolicy,
    /// The byte width for managed small-allocation spans.
    pub managed_small_bytes: usize,
    /// The byte width for local heap pages.
    pub page_bytes: usize,
    /// The byte width for one physical arena segment.
    pub arena_segment_bytes: usize,
    /// The entry count per copy on write metadata table chunk.
    pub table_chunk_len: usize,
}

impl Default for SharedHeapPolicy {
    fn default() -> Self {
        Self {
            gc: GcOptions::shared(),
            small: SizeClassPolicy::default(),
            managed_small_bytes: DEFAULT_SMALL_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            arena_segment_bytes: DEFAULT_ARENA_SEGMENT_BYTES,
            table_chunk_len: DEFAULT_TABLE_CHUNK_LEN,
        }
    }
}

impl SharedHeapPolicy {
    /// Resolve this constructor policy into heap options.
    pub fn resolve(&self) -> Result<HeapOptions, HeapError> {
        let options = HeapOptions {
            gc: self.gc,
            size_classes: self.small.size_classes()?,
            managed_reference_bytes: DEFAULT_MANAGED_REFERENCE_BYTES,
            managed_young_bytes: 0,
            max_managed_young_allocation_bytes: 0,
            managed_small_bytes: self.managed_small_bytes,
            raw_small_bytes: DEFAULT_SMALL_BYTES,
            page_bytes: self.page_bytes,
            arena_segment_bytes: self.arena_segment_bytes,
            card_bytes: DEFAULT_CARD_BYTES,
            small_allocation_alignment_bytes: self.small.alignment_bytes,
            table_chunk_len: self.table_chunk_len,
        };

        options.validate_shared()?;

        Ok(options)
    }
}

/// The configuration for one heap instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapOptions {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The configured small-allocation class table.
    pub size_classes: SizeClassTable,
    /// The encoded byte width for managed references inside traced payloads.
    pub managed_reference_bytes: u8,
    /// The byte width for managed young space.
    pub managed_young_bytes: usize,
    /// The maximum payload size admitted into managed young space.
    pub max_managed_young_allocation_bytes: usize,
    /// The byte width for managed small-allocation spans.
    pub managed_small_bytes: usize,
    /// The byte width for raw small-allocation spans.
    pub raw_small_bytes: usize,
    /// The byte width for local heap pages.
    pub page_bytes: usize,
    /// The byte width for one physical arena segment.
    pub arena_segment_bytes: usize,
    /// The byte width for one remembered card.
    pub card_bytes: usize,
    /// The required alignment for configured small-allocation classes.
    pub small_allocation_alignment_bytes: usize,
    /// The entry count per copy on write metadata table chunk.
    pub table_chunk_len: usize,
}

impl HeapOptions {
    /// Build the default option set for one local heap.
    pub fn local() -> Self {
        Self {
            gc: GcOptions::local(),
            size_classes: SizeClassTable::default(),
            managed_reference_bytes: DEFAULT_MANAGED_REFERENCE_BYTES,
            managed_young_bytes: DEFAULT_YOUNG_BYTES,
            max_managed_young_allocation_bytes: DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES,
            managed_small_bytes: DEFAULT_SMALL_BYTES,
            raw_small_bytes: DEFAULT_SMALL_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            arena_segment_bytes: DEFAULT_ARENA_SEGMENT_BYTES,
            card_bytes: DEFAULT_CARD_BYTES,
            small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
            table_chunk_len: DEFAULT_TABLE_CHUNK_LEN,
        }
    }

    /// Build the default option set for one shared heap.
    pub fn shared() -> Self {
        Self {
            gc: GcOptions::shared(),
            size_classes: SizeClassTable::default(),
            managed_reference_bytes: DEFAULT_MANAGED_REFERENCE_BYTES,
            managed_young_bytes: 0,
            max_managed_young_allocation_bytes: 0,
            managed_small_bytes: DEFAULT_SMALL_BYTES,
            raw_small_bytes: DEFAULT_SMALL_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            arena_segment_bytes: DEFAULT_ARENA_SEGMENT_BYTES,
            card_bytes: DEFAULT_CARD_BYTES,
            small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
            table_chunk_len: DEFAULT_TABLE_CHUNK_LEN,
        }
    }

    /// Return the default table chunk length.
    pub const fn default_table_chunk_len() -> usize {
        DEFAULT_TABLE_CHUNK_LEN
    }

    /// Validate one configured heap page width.
    pub(crate) fn validate_page_bytes(page_bytes: usize) -> Result<usize, HeapError> {
        if page_bytes == 0 || !page_bytes.is_power_of_two() {
            Err(HeapError::InvalidPageBytes { bytes: page_bytes })
        } else {
            Ok(page_bytes)
        }
    }

    /// Validate one encoded managed-reference byte width.
    pub(crate) fn validate_managed_reference_bytes(
        managed_reference_bytes: u8,
    ) -> Result<usize, HeapError> {
        match managed_reference_bytes {
            4 | 8 => Ok(managed_reference_bytes as usize),
            _ => Err(HeapError::UnsupportedManagedReferenceWidth {
                bytes: managed_reference_bytes,
            }),
        }
    }

    /// Validate one configured remembered-card byte width.
    pub(crate) fn validate_card_bytes(card_bytes: usize) -> Result<usize, HeapError> {
        if card_bytes == 0 || !card_bytes.is_power_of_two() {
            Err(HeapError::InvalidCardBytes { bytes: card_bytes })
        } else {
            Ok(card_bytes)
        }
    }

    /// Validate one configured arena segment width.
    pub(crate) fn validate_arena_segment_bytes(
        page_bytes: usize,
        arena_segment_bytes: usize,
    ) -> Result<usize, HeapError> {
        if arena_segment_bytes == 0 {
            return Err(HeapError::InvalidArenaSegmentBytes {
                bytes: arena_segment_bytes,
            });
        }

        if !arena_segment_bytes.is_multiple_of(page_bytes) {
            return Err(HeapError::MisalignedArenaSegmentBytes {
                page_bytes,
                segment_bytes: arena_segment_bytes,
            });
        }

        Ok(arena_segment_bytes)
    }

    /// Validate one configured small-allocation alignment.
    pub(crate) fn validate_small_allocation_alignment_bytes(
        alignment_bytes: usize,
    ) -> Result<usize, HeapError> {
        if alignment_bytes == 0 || !alignment_bytes.is_power_of_two() {
            Err(HeapError::InvalidSmallAllocationAlignmentBytes {
                bytes: alignment_bytes,
            })
        } else {
            Ok(alignment_bytes)
        }
    }

    /// Validate one configured table chunk length.
    pub(crate) fn validate_table_chunk_len(table_chunk_len: usize) -> Result<usize, HeapError> {
        if table_chunk_len == 0 {
            Err(HeapError::InvalidTableChunkLen {
                len: table_chunk_len,
            })
        } else {
            Ok(table_chunk_len)
        }
    }

    /// Validate the common heap geometry shared by local and shared heaps.
    fn validate_common(&self) -> Result<(), HeapError> {
        self.gc.validate()?;
        Self::validate_page_bytes(self.page_bytes)?;
        Self::validate_arena_segment_bytes(self.page_bytes, self.arena_segment_bytes)?;
        Self::validate_table_chunk_len(self.table_chunk_len)?;
        Self::validate_small_allocation_alignment_bytes(self.small_allocation_alignment_bytes)?;

        // keep all size classes aligned to the configured small-slot boundary
        for class in &self.size_classes.classes {
            if class.bytes % self.small_allocation_alignment_bytes != 0 {
                return Err(HeapError::MisalignedSizeClass {
                    alignment_bytes: self.small_allocation_alignment_bytes,
                    class_bytes: class.bytes,
                });
            }
        }

        Ok(())
    }

    /// Validate these options for one local heap.
    pub fn validate_local(&self) -> Result<(), HeapError> {
        self.validate_common()?;
        Self::validate_managed_reference_bytes(self.managed_reference_bytes)?;
        Self::validate_card_bytes(self.card_bytes)?;

        // reject contradictory young-space policy
        if self.managed_young_bytes != 0
            && self.max_managed_young_allocation_bytes > self.managed_young_bytes
        {
            return Err(HeapError::ManagedYoungThresholdExceedsCapacity {
                threshold: self.max_managed_young_allocation_bytes,
                capacity: self.managed_young_bytes,
            });
        }

        Ok(())
    }

    /// Validate these options for one shared heap.
    pub fn validate_shared(&self) -> Result<(), HeapError> {
        self.validate_common()?;

        Ok(())
    }

    /// Validate that one explicit arena matches these heap options.
    pub(crate) fn validate_arena(&self, arena: &Arena) -> Result<(), HeapError> {
        if arena.page_bytes() != self.page_bytes {
            return Err(HeapError::ArenaPageBytesMismatch {
                option_page_bytes: self.page_bytes,
                arena_page_bytes: arena.page_bytes(),
            });
        }

        if arena.segment_bytes() != self.arena_segment_bytes {
            return Err(HeapError::ArenaSegmentBytesMismatch {
                option_segment_bytes: self.arena_segment_bytes,
                arena_segment_bytes: arena.segment_bytes(),
            });
        }

        Ok(())
    }
}
