use std::cmp::Ordering;

use destack_mir::ReferenceMap;
use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapResult, SizeClassTable};

/// The allocation facts used to place one managed heap payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationPlan<'a> {
    /// The exact payload byte length.
    pub byte_len: usize,
    /// The required allocation base alignment in bytes.
    pub alignment: usize,
    /// The exact heap reference map.
    pub reference_map: &'a ReferenceMap,
    /// Whether the payload contains no heap references.
    pub is_noscan: bool,
    /// Whether the payload may contain shared heap references.
    pub has_shared_reference: bool,
}

impl<'a> AllocationPlan<'a> {
    /// Create one allocation plan.
    #[inline(always)]
    pub fn new(byte_len: usize, alignment: usize, reference_map: &'a ReferenceMap) -> Self {
        debug_assert!(alignment == 0 || alignment.is_power_of_two());

        Self {
            byte_len,
            alignment: alignment.max(1),
            reference_map,
            is_noscan: !reference_map.has_reference(),
            has_shared_reference: reference_map.has_shared_reference(),
        }
    }

    /// Return whether this plan describes a valid non-empty heap allocation.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.byte_len == 0
    }
}

/// One allocator-ready allocation class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationClass {
    /// One allocation backed by a small-span slot.
    Small(SmallAllocationLayout),
    /// One allocation backed by a dedicated page run.
    Large,
}

impl AllocationClass {
    /// Return the small allocation class when this class uses small spans.
    pub const fn small(self) -> Option<SmallAllocationLayout> {
        match self {
            Self::Small(small) => Some(small),
            Self::Large => None,
        }
    }
}

/// One allocator-ready managed heap layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationLayout<'a> {
    /// The exact payload byte length.
    pub byte_len: usize,
    /// The required allocation base alignment in bytes.
    pub alignment: usize,
    /// The exact heap reference map.
    pub reference_map: &'a ReferenceMap,
    /// Whether the payload contains no heap references.
    pub is_noscan: bool,
    /// Whether the payload may contain shared heap references.
    pub has_shared_reference: bool,
    /// The allocator class used by this layout.
    pub class: AllocationClass,
}

impl<'a> AllocationLayout<'a> {
    /// Return whether this layout describes a valid non-empty heap allocation.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.byte_len == 0
    }
}

/// One allocator-ready small allocation layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmallAllocationLayout {
    /// The reusable bucket index for this class.
    pub(crate) bucket_index: usize,
    /// The exact size-class index.
    pub(crate) class_index: usize,
    /// The smallest payload byte length routed to this class.
    pub(crate) minimum_byte_len: usize,
    /// The small-span class used by local and shared spaces.
    pub(crate) class: SmallSpanClass,
}

/// One small-span size and scan class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SmallSpanClass {
    /// The slot payload size in bytes.
    pub(crate) size_class: usize,
    /// The span byte width for this size class.
    pub(crate) span_bytes: usize,
    /// Whether every slot in this span has no references.
    pub(crate) is_noscan: bool,
}

impl SmallSpanClass {
    /// Create one small-span class from a validated size class.
    pub(crate) const fn new(size_class: usize, span_bytes: usize, is_noscan: bool) -> Self {
        Self {
            size_class,
            span_bytes,
            is_noscan,
        }
    }

    /// Return the number of reusable small-span buckets for one size-class table.
    pub(crate) fn bucket_count(size_classes: &SizeClassTable) -> usize {
        size_classes.classes.len() * 2
    }

    /// Return the reusable-span bucket index for this class.
    pub(crate) fn bucket_index(&self, size_classes: &SizeClassTable) -> Result<usize, HeapError> {
        let Some(class_index) = size_classes.class_index_for(self.size_class) else {
            return Err(HeapError::InvalidSizeClass {
                class_bytes: self.size_class,
            });
        };
        let class_bytes = size_classes.classes[class_index].bytes;
        if class_bytes != self.size_class {
            return Err(HeapError::InvalidSizeClass {
                class_bytes: self.size_class,
            });
        }

        Ok(class_index * 2 + self.is_noscan as usize)
    }
}

/// Resolve one allocation plan against a concrete small allocation table.
pub(crate) fn allocation_layout<'a>(
    plan: AllocationPlan<'a>,
    size_classes: &SizeClassTable,
    page_bytes: usize,
    span_bytes: usize,
) -> AllocationLayout<'a> {
    let class = allocation_class(
        plan.byte_len,
        plan.alignment,
        plan.is_noscan,
        size_classes,
        page_bytes,
        span_bytes,
    );

    AllocationLayout {
        byte_len: plan.byte_len,
        alignment: plan.alignment,
        reference_map: plan.reference_map,
        is_noscan: plan.is_noscan,
        has_shared_reference: plan.has_shared_reference,
        class,
    }
}

/// Resolve one allocation class against a concrete small allocation table.
pub(crate) fn allocation_class(
    byte_len: usize,
    alignment: usize,
    is_noscan: bool,
    size_classes: &SizeClassTable,
    page_bytes: usize,
    span_bytes: usize,
) -> AllocationClass {
    let Some(class_index) = size_classes.class_index_for_layout(byte_len, alignment) else {
        return AllocationClass::Large;
    };
    let size_class = size_classes.classes[class_index];
    let minimum_byte_len = size_classes.class_minimum_byte_len(class_index, alignment);
    let span_bytes = size_class
        .span_bytes(page_bytes, span_bytes)
        .max(span_bytes);
    let class = SmallSpanClass::new(size_class.bytes, span_bytes, is_noscan);
    let bucket_index = class_index * 2 + is_noscan as usize;

    AllocationClass::Small(SmallAllocationLayout {
        bucket_index,
        class_index,
        minimum_byte_len,
        class,
    })
}

impl PartialOrd for SmallSpanClass {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SmallSpanClass {
    fn cmp(&self, other: &Self) -> Ordering {
        self.size_class
            .cmp(&other.size_class)
            .then_with(|| self.span_bytes.cmp(&other.span_bytes))
            .then_with(|| self.is_noscan.cmp(&other.is_noscan))
    }
}

/// Return the allocation layout for one repeated element layout.
pub fn repeated_layout(
    element_byte_len: usize,
    element_alignment: usize,
    element_reference_map: &ReferenceMap,
    count: usize,
) -> HeapResult<(usize, ReferenceMap)> {
    let element_stride = layout_stride(element_byte_len, element_alignment);
    let byte_len =
        element_stride
            .checked_mul(count)
            .ok_or(HeapError::RepresentationLimitExceeded {
                context: "repeated payload byte length",
            })?;
    let reference_map = repeated_reference_map(element_reference_map, element_stride, count)?;

    Ok((byte_len, reference_map))
}

/// Return the aligned stride for one payload.
fn layout_stride(byte_len: usize, alignment: usize) -> usize {
    let alignment = alignment.max(1);

    byte_len.div_ceil(alignment) * alignment
}

/// Return the repeated reference map for one repeated element layout.
fn repeated_reference_map(
    element_map: &ReferenceMap,
    element_stride: usize,
    count: usize,
) -> HeapResult<ReferenceMap> {
    if count == 0 {
        return Ok(ReferenceMap::None);
    }

    match element_map {
        ReferenceMap::None => Ok(ReferenceMap::None),
        ReferenceMap::Reference {
            local_offsets,
            shared_offsets,
        } if local_offsets.is_empty() && shared_offsets.is_empty() => Ok(ReferenceMap::None),
        ReferenceMap::Reference {
            local_offsets,
            shared_offsets,
        } => Ok(ReferenceMap::RepeatedReference {
            count: u32::try_from(count).map_err(|_| HeapError::RepresentationLimitExceeded {
                context: "repeated payload element count",
            })?,
            stride: u32::try_from(element_stride).map_err(|_| {
                HeapError::RepresentationLimitExceeded {
                    context: "repeated payload element stride",
                }
            })?,
            local_offsets: local_offsets.clone(),
            shared_offsets: shared_offsets.clone(),
        }),
        ReferenceMap::RepeatedReference {
            count: inner_count,
            stride,
            local_offsets,
            shared_offsets,
        } => nested_repeated_reference_map(
            *inner_count,
            *stride,
            local_offsets,
            shared_offsets,
            element_stride,
            count,
        ),
    }
}

/// Return the composed reference map for repeated elements that are themselves repeated.
fn nested_repeated_reference_map(
    inner_count: u32,
    inner_stride: u32,
    local_offsets: &[u32],
    shared_offsets: &[u32],
    element_stride: usize,
    outer_count: usize,
) -> HeapResult<ReferenceMap> {
    if inner_count == 0 || local_offsets.is_empty() && shared_offsets.is_empty() {
        return Ok(ReferenceMap::None);
    }

    let inner_count = inner_count as usize;
    let inner_stride = inner_stride as usize;
    let contiguous_stride =
        inner_count
            .checked_mul(inner_stride)
            .ok_or(HeapError::RepresentationLimitExceeded {
                context: "nested repeated payload stride",
            })?;

    if element_stride == contiguous_stride {
        return Ok(ReferenceMap::RepeatedReference {
            count: u32::try_from(outer_count.checked_mul(inner_count).ok_or(
                HeapError::RepresentationLimitExceeded {
                    context: "nested repeated payload count",
                },
            )?)
            .map_err(|_| HeapError::RepresentationLimitExceeded {
                context: "nested repeated payload count",
            })?,
            stride: u32::try_from(inner_stride).map_err(|_| {
                HeapError::RepresentationLimitExceeded {
                    context: "nested repeated payload stride",
                }
            })?,
            local_offsets: local_offsets.into(),
            shared_offsets: shared_offsets.into(),
        });
    }

    Ok(ReferenceMap::Reference {
        local_offsets: expand_repeated_offsets(
            local_offsets,
            element_stride,
            outer_count,
            inner_count,
            inner_stride,
        )?,
        shared_offsets: expand_repeated_offsets(
            shared_offsets,
            element_stride,
            outer_count,
            inner_count,
            inner_stride,
        )?,
    })
}

/// Return concrete offsets for one nested repeated reference map.
fn expand_repeated_offsets(
    offsets: &[u32],
    element_stride: usize,
    outer_count: usize,
    inner_count: usize,
    inner_stride: usize,
) -> HeapResult<Box<[u32]>> {
    let total_offsets = outer_count
        .checked_mul(inner_count)
        .and_then(|count| count.checked_mul(offsets.len()))
        .ok_or(HeapError::RepresentationLimitExceeded {
            context: "nested repeated payload offsets",
        })?;
    let mut expanded = Vec::with_capacity(total_offsets);

    for element_index in 0..outer_count {
        let element_base = element_index.checked_mul(element_stride).ok_or(
            HeapError::RepresentationLimitExceeded {
                context: "nested repeated payload element base",
            },
        )?;

        for inner_index in 0..inner_count {
            let inner_base = inner_index.checked_mul(inner_stride).ok_or(
                HeapError::RepresentationLimitExceeded {
                    context: "nested repeated payload inner base",
                },
            )?;
            let base = element_base.checked_add(inner_base).ok_or(
                HeapError::RepresentationLimitExceeded {
                    context: "nested repeated payload base",
                },
            )?;

            for offset in offsets {
                let offset = base.checked_add(*offset as usize).ok_or(
                    HeapError::RepresentationLimitExceeded {
                        context: "nested repeated payload offset",
                    },
                )?;
                let offset =
                    u32::try_from(offset).map_err(|_| HeapError::RepresentationLimitExceeded {
                        context: "nested repeated payload offset",
                    })?;
                expanded.push(offset);
            }
        }
    }

    Ok(expanded.into_boxed_slice())
}
