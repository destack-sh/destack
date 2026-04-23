use destack_mir::{Layout, ReferenceMap};

use crate::allocator::{Allocator, PageView};
use crate::{HeapError, HeapResult};

/// One heap allocation payload source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payload<'a> {
    /// Caller-provided bytes.
    Bytes(&'a [u8]),
    /// Zeroed bytes.
    Zeroed,
    /// Bytes copied from an existing page view.
    PageView {
        /// The source logical page view.
        page_view: &'a PageView,
        /// The source byte offset.
        start: usize,
        /// The copied byte length.
        byte_len: usize,
    },
}

impl<'a> Payload<'a> {
    /// Return the explicit payload byte length when this payload has one.
    pub(crate) fn byte_len(&self) -> Option<usize> {
        match self {
            Self::Bytes(bytes) => Some(bytes.len()),
            Self::Zeroed => None,
            Self::PageView { byte_len, .. } => Some(*byte_len),
        }
    }

    /// Initialize this payload into one allocated page view.
    pub(crate) fn initialize(
        &self,
        allocator: &Allocator,
        pages: &mut PageView,
        byte_offset: usize,
    ) -> HeapResult<()> {
        match self {
            Self::Bytes(bytes) => allocator.set_bytes(pages, byte_offset, bytes),
            Self::Zeroed => Ok(()),
            Self::PageView {
                page_view,
                start,
                byte_len,
            } => allocator.copy_bytes_between_page_views(
                page_view,
                *start,
                pages,
                byte_offset,
                *byte_len,
            ),
        }
    }
}

/// Return the heap payload facts for one repeated element layout.
pub(crate) fn repeated_payload(
    element: &Layout,
    count: usize,
) -> HeapResult<(usize, ReferenceMap)> {
    let element_stride = layout_stride(element.size as usize, element.alignment as usize);
    let byte_len = element_stride
        .checked_mul(count)
        .ok_or(HeapError::InvariantOverflow {
            context: "repeated payload byte length",
        })?;
    let reference_map = repeated_reference_map(&element.reference_map, element_stride, count)?;

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
