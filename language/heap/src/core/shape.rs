use std::cmp::Ordering;

use destack_mir::ReferenceMap;
use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapResult, SizeClassTable};

/// The shape facts required to allocate one managed heap payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationLayout<'a> {
    /// The exact payload byte length.
    pub byte_len: usize,
    /// The exact heap reference map.
    pub reference_map: &'a ReferenceMap,
}

impl<'a> AllocationLayout<'a> {
    /// Create one managed allocation shape.
    pub const fn new(byte_len: usize, reference_map: &'a ReferenceMap) -> Self {
        Self {
            byte_len,
            reference_map,
        }
    }
}

/// One small-span size and scan class.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SmallSpanClass {
    /// The slot payload size in bytes.
    pub(crate) size_class: usize,
    /// Whether every slot in this span has no references.
    pub(crate) is_noscan: bool,
}

impl SmallSpanClass {
    /// Return the number of reusable small-span buckets for one size-class table.
    pub(crate) fn bucket_count(size_classes: &SizeClassTable) -> usize {
        size_classes.classes.len().saturating_mul(2)
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

        Ok(class_index.saturating_mul(2) + self.is_noscan as usize)
    }
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
            .then_with(|| self.is_noscan.cmp(&other.is_noscan))
    }
}

/// Return the allocation shape for one repeated element layout.
pub fn repeated_layout(
    element_byte_len: usize,
    element_alignment: usize,
    element_reference_map: &ReferenceMap,
    count: usize,
) -> HeapResult<(usize, ReferenceMap)> {
    let element_stride = layout_stride(element_byte_len, element_alignment);
    let byte_len = element_stride
        .checked_mul(count)
        .ok_or(HeapError::InvariantOverflow {
            context: "repeated payload byte length",
        })?;
    let reference_map = repeated_reference_map(element_reference_map, element_stride, count)?;

    Ok((byte_len, reference_map))
}

/// Return the aligned storage stride for one layout payload.
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
            count: u32::try_from(count).map_err(|_| HeapError::InvariantOverflow {
                context: "repeated payload element count",
            })?,
            stride: u32::try_from(element_stride).map_err(|_| HeapError::InvariantOverflow {
                context: "repeated payload element stride",
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
            .ok_or(HeapError::InvariantOverflow {
                context: "nested repeated payload stride",
            })?;

    if element_stride == contiguous_stride {
        return Ok(ReferenceMap::RepeatedReference {
            count: u32::try_from(outer_count.checked_mul(inner_count).ok_or(
                HeapError::InvariantOverflow {
                    context: "nested repeated payload count",
                },
            )?)
            .map_err(|_| HeapError::InvariantOverflow {
                context: "nested repeated payload count",
            })?,
            stride: u32::try_from(inner_stride).map_err(|_| HeapError::InvariantOverflow {
                context: "nested repeated payload stride",
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
        .ok_or(HeapError::InvariantOverflow {
            context: "nested repeated payload offsets",
        })?;
    let mut expanded = Vec::with_capacity(total_offsets);

    for element_index in 0..outer_count {
        let element_base =
            element_index
                .checked_mul(element_stride)
                .ok_or(HeapError::InvariantOverflow {
                    context: "nested repeated payload element base",
                })?;

        for inner_index in 0..inner_count {
            let inner_base =
                inner_index
                    .checked_mul(inner_stride)
                    .ok_or(HeapError::InvariantOverflow {
                        context: "nested repeated payload inner base",
                    })?;
            let base =
                element_base
                    .checked_add(inner_base)
                    .ok_or(HeapError::InvariantOverflow {
                        context: "nested repeated payload base",
                    })?;

            for offset in offsets {
                let offset =
                    base.checked_add(*offset as usize)
                        .ok_or(HeapError::InvariantOverflow {
                            context: "nested repeated payload offset",
                        })?;
                let offset = u32::try_from(offset).map_err(|_| HeapError::InvariantOverflow {
                    context: "nested repeated payload offset",
                })?;
                expanded.push(offset);
            }
        }
    }

    Ok(expanded.into_boxed_slice())
}
