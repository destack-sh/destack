use destack_mir::ReferenceMap;

use crate::allocator::Bitmap;
use crate::{HeapError, HeapReference, HeapResult, SharedHeapReference};

/// One traced reference encoding.
trait TracedReference: Copy {
    /// The encoded byte width of one traced reference.
    const BYTE_LEN: usize = std::mem::size_of::<usize>();

    /// Read one reference from a native-width byte window.
    fn read_from_bytes(bytes: &[u8]) -> HeapResult<Self>;
}

impl TracedReference for HeapReference {
    fn read_from_bytes(bytes: &[u8]) -> HeapResult<Self> {
        HeapReference::read_from_bytes(bytes)
    }
}

impl TracedReference for SharedHeapReference {
    fn read_from_bytes(bytes: &[u8]) -> HeapResult<Self> {
        SharedHeapReference::read_from_bytes(bytes)
    }
}

/// One heap edge map over MIR reference metadata.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EdgeMap<'a> {
    /// The underlying MIR reference map.
    reference_map: &'a ReferenceMap,
}

impl<'a> EdgeMap<'a> {
    /// Create one heap edge map.
    pub(crate) const fn new(reference_map: &'a ReferenceMap) -> Self {
        Self { reference_map }
    }

    /// Return heap edge offsets.
    fn local_offsets(self) -> EdgeOffsets<'a> {
        match self.reference_map {
            ReferenceMap::None => EdgeOffsets::None,
            ReferenceMap::Reference { local_offsets, .. } => EdgeOffsets::Direct(local_offsets),
            ReferenceMap::RepeatedReference {
                count,
                stride,
                local_offsets,
                ..
            } => EdgeOffsets::Repeated {
                count: *count,
                stride: *stride,
                offsets: local_offsets,
            },
        }
    }

    /// Return shared heap edge offsets.
    fn shared_offsets(self) -> EdgeOffsets<'a> {
        match self.reference_map {
            ReferenceMap::None => EdgeOffsets::None,
            ReferenceMap::Reference { shared_offsets, .. } => EdgeOffsets::Direct(shared_offsets),
            ReferenceMap::RepeatedReference {
                count,
                stride,
                shared_offsets,
                ..
            } => EdgeOffsets::Repeated {
                count: *count,
                stride: *stride,
                offsets: shared_offsets,
            },
        }
    }

    /// Return heap edge offsets in one dirty byte range.
    fn local_offsets_in_range(self, start: usize, len: usize, width: usize) -> EdgeCursor<'a> {
        EdgeCursor::in_range(self.local_offsets(), start, len, width)
    }

    /// Return shared heap edge offsets in one dirty byte range.
    fn shared_offsets_in_range(self, start: usize, len: usize, width: usize) -> EdgeCursor<'a> {
        EdgeCursor::in_range(self.shared_offsets(), start, len, width)
    }
}

/// One edge offset table for one heap.
#[derive(Debug, Clone, Copy)]
enum EdgeOffsets<'a> {
    /// No matching edges.
    None,
    /// One direct offset table.
    Direct(&'a [u32]),
    /// One repeated offset table.
    Repeated {
        /// The repeated element count.
        count: u32,
        /// The repeated element stride.
        stride: u32,
        /// The per-element edge offsets.
        offsets: &'a [u32],
    },
}

impl EdgeOffsets<'_> {
    /// Return whether these edges overlap one byte range.
    fn overlaps(self, start: usize, len: usize, width: usize) -> bool {
        if len == 0 {
            return false;
        }

        let end = start + len;

        match self {
            Self::None => false,
            Self::Direct(offsets) => offsets
                .iter()
                .copied()
                .any(|offset| ranges_overlap(start, end, offset as usize, width)),
            Self::Repeated {
                count,
                stride,
                offsets,
            } => offsets.iter().copied().any(|offset| {
                overlapping_repeated_index_range(
                    start,
                    end,
                    count as usize,
                    stride as usize,
                    offset as usize,
                    width,
                )
                .is_some()
            }),
        }
    }
}

/// One cursor over concrete heap edge offsets.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EdgeCursor<'a> {
    /// The source edge offsets.
    offsets: EdgeOffsets<'a>,
    /// The current repeated element index.
    element_index: usize,
    /// The current per-element offset index.
    offset_index: usize,
    /// The optional overlap window.
    range: Option<EdgeRange>,
}

impl<'a> EdgeCursor<'a> {
    /// Create one cursor over every edge offset.
    fn new(offsets: EdgeOffsets<'a>) -> Self {
        Self {
            offsets,
            element_index: 0,
            offset_index: 0,
            range: None,
        }
    }

    /// Create one cursor over edge offsets overlapping a byte range.
    fn in_range(offsets: EdgeOffsets<'a>, start: usize, len: usize, width: usize) -> Self {
        let end = start + len;

        Self {
            offsets,
            element_index: 0,
            offset_index: 0,
            range: Some(EdgeRange { start, end, width }),
        }
    }

    /// Return the next concrete edge offset.
    fn next_offset(&mut self) -> HeapResult<Option<usize>> {
        loop {
            let Some(offset) = self.next_unfiltered_offset()? else {
                return Ok(None);
            };

            if let Some(range) = self.range
                && !ranges_overlap(range.start, range.end, offset, range.width)
            {
                continue;
            }

            return Ok(Some(offset));
        }
    }

    /// Return the next concrete edge offset before range filtering.
    fn next_unfiltered_offset(&mut self) -> HeapResult<Option<usize>> {
        match self.offsets {
            EdgeOffsets::None => Ok(None),
            EdgeOffsets::Direct(offsets) => {
                let Some(offset) = offsets.get(self.offset_index).copied() else {
                    return Ok(None);
                };
                self.offset_index += 1;

                Ok(Some(offset as usize))
            }
            EdgeOffsets::Repeated {
                count,
                stride,
                offsets,
            } => {
                while self.element_index < count as usize {
                    let Some(offset) = offsets.get(self.offset_index).copied() else {
                        self.element_index += 1;
                        self.offset_index = 0;
                        continue;
                    };
                    self.offset_index += 1;

                    let base = self.element_index * stride as usize;
                    let offset = base + offset as usize;

                    return Ok(Some(offset));
                }

                Ok(None)
            }
        }
    }
}

/// One edge cursor overlap window.
#[derive(Debug, Clone, Copy)]
struct EdgeRange {
    /// The start byte offset.
    start: usize,
    /// The exclusive end byte offset.
    end: usize,
    /// The edge byte width.
    width: usize,
}

/// Return the exact reference map encoded for one small slot.
pub(crate) fn slot_reference_map(
    local_reference_bits: &Bitmap,
    shared_reference_bits: &Bitmap,
    slot_index: usize,
    size_class: usize,
    byte_len: usize,
) -> ReferenceMap {
    let word_bytes = std::mem::size_of::<usize>();
    let word_count = size_class.div_ceil(word_bytes);
    let bit_start = slot_index * word_count;
    let mut local_offsets = Vec::new();
    let mut shared_offsets = Vec::new();

    // decode one direct reference map from the span-local slot bits
    for word_index in 0..word_count {
        let bit_index = bit_start + word_index;
        let byte_offset = word_index * word_bytes;
        let byte_end = byte_offset + word_bytes;
        if byte_end > byte_len {
            break;
        }

        if local_reference_bits.contains(bit_index) {
            local_offsets.push(byte_offset as u32);
        }

        if shared_reference_bits.contains(bit_index) {
            shared_offsets.push(byte_offset as u32);
        }
    }

    if local_offsets.is_empty() && shared_offsets.is_empty() {
        return ReferenceMap::None;
    }

    ReferenceMap::Reference {
        local_offsets: local_offsets.into_boxed_slice(),
        shared_offsets: shared_offsets.into_boxed_slice(),
    }
}

/// Return whether one write range may overlap any heap-edge bytes.
pub(crate) fn overlaps_heap_range(reference_map: &ReferenceMap, start: usize, len: usize) -> bool {
    EdgeMap::new(reference_map)
        .local_offsets()
        .overlaps(start, len, HeapReference::BYTE_LEN)
}

/// Return whether one write range may overlap any shared heap-edge bytes.
pub(crate) fn overlaps_shared_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
) -> bool {
    EdgeMap::new(reference_map)
        .shared_offsets()
        .overlaps(start, len, SharedHeapReference::BYTE_LEN)
}

/// Return the exact reference map encoded for one allocation byte range.
pub(crate) fn allocation_reference_map(
    local_reference_bits: &Bitmap,
    shared_reference_bits: &Bitmap,
    byte_offset: usize,
    byte_len: usize,
) -> ReferenceMap {
    let word_bytes = std::mem::size_of::<usize>();
    let bit_start = byte_offset / word_bytes;
    let word_count = byte_len.div_ceil(word_bytes);
    let mut local_offsets = Vec::new();
    let mut shared_offsets = Vec::new();

    // decode direct reference bits relative to the allocation base
    for word_index in 0..word_count {
        let bit_index = bit_start + word_index;
        let local_byte_offset = word_index * word_bytes;
        let local_byte_end = local_byte_offset + word_bytes;
        if local_byte_end > byte_len {
            break;
        }

        if local_reference_bits.contains(bit_index) {
            local_offsets.push(local_byte_offset as u32);
        }

        if shared_reference_bits.contains(bit_index) {
            shared_offsets.push(local_byte_offset as u32);
        }
    }

    if local_offsets.is_empty() && shared_offsets.is_empty() {
        return ReferenceMap::None;
    }

    ReferenceMap::Reference {
        local_offsets: local_offsets.into_boxed_slice(),
        shared_offsets: shared_offsets.into_boxed_slice(),
    }
}

/// Report whether one byte range overlaps one fixed-width field range.
pub(crate) fn ranges_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_len: usize,
) -> bool {
    let Some(right_end) = right_start.checked_add(right_len) else {
        return true;
    };

    left_start < right_end && right_start < left_end
}

/// Return the overlapping repeated-element index range for the given byte window.
pub(crate) fn overlapping_repeated_index_range(
    start: usize,
    end: usize,
    count: usize,
    stride: usize,
    offset: usize,
    width: usize,
) -> Option<(usize, usize)> {
    if count == 0 || width == 0 {
        return None;
    }

    if stride == 0 {
        return ranges_overlap(start, end, offset, width).then_some((0, count - 1));
    }

    let start = start as i128;
    let end = end as i128;
    let count = count as i128;
    let stride = stride as i128;
    let offset = offset as i128;
    let width = width as i128;
    let low_numerator = start - offset - width + 1;
    let high_numerator = end - offset - 1;
    let low = div_ceil_i128(low_numerator, stride).max(0);
    let high = div_floor_i128(high_numerator, stride).min(count - 1);

    (low <= high).then_some((low as usize, high as usize))
}

/// Return floor(lhs / rhs) for signed integers with positive rhs.
fn div_floor_i128(lhs: i128, rhs: i128) -> i128 {
    debug_assert!(rhs > 0);

    let quotient = lhs / rhs;
    let remainder = lhs % rhs;

    if remainder < 0 {
        quotient - 1
    } else {
        quotient
    }
}

/// Return ceil(lhs / rhs) for signed integers with positive rhs.
fn div_ceil_i128(lhs: i128, rhs: i128) -> i128 {
    debug_assert!(rhs > 0);

    let quotient = lhs / rhs;
    let remainder = lhs % rhs;

    if remainder > 0 {
        quotient + 1
    } else {
        quotient
    }
}

/// Clear the exact reference bits for one small slot.
pub(crate) fn clear_slot_reference_bits(
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    slot_index: usize,
    size_class: usize,
) {
    let bit_len = size_class.div_ceil(std::mem::size_of::<usize>());
    let bit_start = slot_index * bit_len;

    local_reference_bits.clear_range(bit_start, bit_len);
    shared_reference_bits.clear_range(bit_start, bit_len);
}

/// Clear the exact reference bits for one allocation byte range.
pub(crate) fn clear_allocation_reference_bits(
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    byte_offset: usize,
    byte_len: usize,
) {
    let word_bytes = std::mem::size_of::<usize>();
    let bit_start = byte_offset / word_bytes;
    let bit_len = byte_len.div_ceil(word_bytes);

    local_reference_bits.clear_range(bit_start, bit_len);
    shared_reference_bits.clear_range(bit_start, bit_len);
}

/// Encode one exact reference map into one small-slot bit range.
pub(crate) fn write_slot_reference_bits(
    reference_map: &ReferenceMap,
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    slot_index: usize,
    size_class: usize,
) -> HeapResult<()> {
    if !reference_map.has_reference() {
        return Ok(());
    }

    clear_slot_reference_bits(
        local_reference_bits,
        shared_reference_bits,
        slot_index,
        size_class,
    );

    let edge_map = EdgeMap::new(reference_map);
    set_slot_edge_bits(
        local_reference_bits,
        slot_index,
        size_class,
        EdgeCursor::new(edge_map.local_offsets()),
    )?;
    set_slot_edge_bits(
        shared_reference_bits,
        slot_index,
        size_class,
        EdgeCursor::new(edge_map.shared_offsets()),
    )?;

    Ok(())
}

/// Encode one exact reference map into one allocation byte range.
pub(crate) fn write_allocation_reference_bits(
    reference_map: &ReferenceMap,
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    byte_offset: usize,
    byte_len: usize,
) -> HeapResult<()> {
    if !reference_map.has_reference() {
        return Ok(());
    }

    clear_allocation_reference_bits(
        local_reference_bits,
        shared_reference_bits,
        byte_offset,
        byte_len,
    );

    let edge_map = EdgeMap::new(reference_map);
    set_allocation_edge_bits(
        local_reference_bits,
        byte_offset,
        EdgeCursor::new(edge_map.local_offsets()),
    )?;
    set_allocation_edge_bits(
        shared_reference_bits,
        byte_offset,
        EdgeCursor::new(edge_map.shared_offsets()),
    )?;

    Ok(())
}

/// Visit each heap reference encoded in the given payload bytes.
pub fn visit_heap_references(
    reference_map: &ReferenceMap,
    bytes: &[u8],
    visit: impl FnMut(HeapReference),
) -> HeapResult<()> {
    visit_heap_references_in_reader(
        reference_map,
        |start, buffer| {
            let end = start + buffer.len();
            let Some(window) = bytes.get(start..end) else {
                return Err(HeapError::TruncatedReferenceReaderWindow {
                    start,
                    width: buffer.len(),
                });
            };

            buffer.copy_from_slice(window);
            Ok(())
        },
        visit,
    )
}

/// Visit each heap reference encoded by one scan through one reader.
pub(crate) fn visit_heap_references_in_reader(
    reference_map: &ReferenceMap,
    read_edge: impl FnMut(usize, &mut [u8]) -> HeapResult<()>,
    visit: impl FnMut(HeapReference),
) -> HeapResult<()> {
    let cursor = EdgeCursor::new(EdgeMap::new(reference_map).local_offsets());

    visit_edges_in_reader(cursor, read_edge, visit)
}

/// Visit each shared heap reference encoded by one scan through one reader.
pub fn visit_shared_references_in_reader(
    reference_map: &ReferenceMap,
    read_edge: impl FnMut(usize, &mut [u8]) -> HeapResult<()>,
    visit: impl FnMut(SharedHeapReference),
) -> HeapResult<()> {
    let cursor = EdgeCursor::new(EdgeMap::new(reference_map).shared_offsets());

    visit_edges_in_reader(cursor, read_edge, visit)
}

/// Visit each overlapping heap reference encoded by one scan through one reader.
pub(crate) fn visit_heap_references_in_reader_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
    read_edge: impl FnMut(usize, &mut [u8]) -> HeapResult<()>,
    visit: impl FnMut(HeapReference),
) -> HeapResult<()> {
    let cursor =
        EdgeMap::new(reference_map).local_offsets_in_range(start, len, HeapReference::BYTE_LEN);

    visit_edges_in_reader(cursor, read_edge, visit)
}

/// Visit each overlapping shared heap reference encoded by one scan through one reader.
pub(crate) fn visit_shared_references_in_reader_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
    read_edge: impl FnMut(usize, &mut [u8]) -> HeapResult<()>,
    visit: impl FnMut(SharedHeapReference),
) -> HeapResult<()> {
    let cursor = EdgeMap::new(reference_map).shared_offsets_in_range(
        start,
        len,
        SharedHeapReference::BYTE_LEN,
    );

    visit_edges_in_reader(cursor, read_edge, visit)
}

/// Visit each traced edge through one random-access reader.
fn visit_edges_in_reader<R: TracedReference>(
    mut cursor: EdgeCursor<'_>,
    mut read_edge: impl FnMut(usize, &mut [u8]) -> HeapResult<()>,
    mut visit: impl FnMut(R),
) -> HeapResult<()> {
    let mut window = [0u8; std::mem::size_of::<usize>()];

    while let Some(start) = cursor.next_offset()? {
        read_edge(start, &mut window[..R::BYTE_LEN])?;
        visit(R::read_from_bytes(&window[..R::BYTE_LEN])?);
    }

    Ok(())
}

/// Encode one edge cursor into one slot bitmap.
fn set_slot_edge_bits(
    reference_bits: &mut Bitmap,
    slot_index: usize,
    size_class: usize,
    mut cursor: EdgeCursor<'_>,
) -> HeapResult<()> {
    let word_bytes = std::mem::size_of::<usize>();
    let word_count = size_class.div_ceil(word_bytes);
    let bit_start = slot_index * word_count;

    while let Some(offset) = cursor.next_offset()? {
        let word_index = offset / word_bytes;
        let bit_index = bit_start + word_index;

        reference_bits.set(bit_index);
    }

    Ok(())
}

/// Encode one edge cursor into one allocation bitmap.
fn set_allocation_edge_bits(
    reference_bits: &mut Bitmap,
    byte_offset: usize,
    mut cursor: EdgeCursor<'_>,
) -> HeapResult<()> {
    let word_bytes = std::mem::size_of::<usize>();

    while let Some(offset) = cursor.next_offset()? {
        let absolute_offset = byte_offset + offset;
        let bit_index = absolute_offset / word_bytes;

        reference_bits.set(bit_index);
    }

    Ok(())
}
