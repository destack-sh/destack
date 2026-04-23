use destack_mir::ReferenceMap;

use super::{overlapping_repeated_index_range, ranges_overlap};
use crate::allocator::Bitmap;
use crate::{HeapError, HeapReference, HeapResult, SharedHeapReference};

/// One traced reference encoding.
trait TracedReference: Copy {
    /// The encoded byte width of one traced reference.
    const BYTE_LEN: usize = std::mem::size_of::<usize>();

    /// Restore one reference from native-width bits.
    fn from_bits(bits: usize) -> Self;
}

impl TracedReference for HeapReference {
    fn from_bits(bits: usize) -> Self {
        HeapReference::from_bits(bits)
    }
}

impl TracedReference for SharedHeapReference {
    fn from_bits(bits: usize) -> Self {
        SharedHeapReference::from_bits(bits)
    }
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
    let bit_start = slot_index.saturating_mul(word_count);
    let mut local_offsets = Vec::new();
    let mut shared_offsets = Vec::new();

    // decode one direct reference map from the span-local slot bits
    for word_index in 0..word_count {
        let bit_index = bit_start.saturating_add(word_index);
        let byte_offset = word_index.saturating_mul(word_bytes);
        let byte_end = byte_offset.saturating_add(word_bytes);
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

/// Clear the exact reference bits for one small slot.
pub(crate) fn clear_slot_reference_bits(
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    slot_index: usize,
    size_class: usize,
) {
    let bit_len = size_class.div_ceil(std::mem::size_of::<usize>());
    let bit_start = slot_index.saturating_mul(bit_len);

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
) {
    clear_slot_reference_bits(
        local_reference_bits,
        shared_reference_bits,
        slot_index,
        size_class,
    );

    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Reference {
            local_offsets,
            shared_offsets,
        } => {
            set_reference_offsets(local_reference_bits, slot_index, size_class, local_offsets);
            set_reference_offsets(
                shared_reference_bits,
                slot_index,
                size_class,
                shared_offsets,
            );
        }
        ReferenceMap::RepeatedReference {
            count,
            stride,
            local_offsets,
            shared_offsets,
        } => {
            set_repeated_reference_offsets(
                local_reference_bits,
                slot_index,
                size_class,
                *count,
                *stride,
                local_offsets,
            );
            set_repeated_reference_offsets(
                shared_reference_bits,
                slot_index,
                size_class,
                *count,
                *stride,
                shared_offsets,
            );
        }
    }
}

/// One trace offset table for one reference kind.
enum ReferenceOffsets<'a> {
    /// No references of this kind.
    None,
    /// One direct offset table.
    Direct(&'a [u32]),
    /// One repeated offset table.
    Repeated {
        /// The repeated element count.
        count: u32,
        /// The repeated element stride.
        stride: u32,
        /// The per-element reference offsets.
        offsets: &'a [u32],
    },
}

/// Visit each local heap reference encoded in the given payload bytes.
pub fn visit_heap_references(
    reference_map: &ReferenceMap,
    bytes: &[u8],
    visit: impl FnMut(HeapReference),
) -> HeapResult<()> {
    visit_heap_references_in_reader(
        reference_map,
        |start, buffer| {
            let Some(end) = start.checked_add(buffer.len()) else {
                return false;
            };
            let Some(window) = bytes.get(start..end) else {
                return false;
            };

            buffer.copy_from_slice(window);
            true
        },
        visit,
    )
}

/// Visit each local heap reference encoded by one scan through one reader.
pub(crate) fn visit_heap_references_in_reader(
    reference_map: &ReferenceMap,
    fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    visit: impl FnMut(HeapReference),
) -> HeapResult<()> {
    visit_reference_map_in_reader(local_reference_offsets(reference_map), fill_bytes, visit)
}

/// Visit each shared heap reference encoded by one scan through one reader.
pub(crate) fn visit_shared_references_in_reader(
    reference_map: &ReferenceMap,
    fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    visit: impl FnMut(SharedHeapReference),
) -> HeapResult<()> {
    visit_reference_map_in_reader(shared_reference_offsets(reference_map), fill_bytes, visit)
}

/// Visit each overlapping local heap reference encoded by one scan through one reader.
pub(crate) fn visit_heap_references_in_reader_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
    fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    visit: impl FnMut(HeapReference),
) -> HeapResult<()> {
    visit_trace_references_in_reader_range(
        local_reference_offsets(reference_map),
        start,
        len,
        fill_bytes,
        visit,
    )
}

/// Visit each overlapping shared heap reference encoded by one scan through one reader.
pub(crate) fn visit_shared_references_in_reader_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
    fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    visit: impl FnMut(SharedHeapReference),
) -> HeapResult<()> {
    visit_trace_references_in_reader_range(
        shared_reference_offsets(reference_map),
        start,
        len,
        fill_bytes,
        visit,
    )
}

/// Return the local-reference offsets for one trace.
fn local_reference_offsets(reference_map: &ReferenceMap) -> ReferenceOffsets<'_> {
    match reference_map {
        ReferenceMap::None => ReferenceOffsets::None,
        ReferenceMap::Reference { local_offsets, .. } => ReferenceOffsets::Direct(local_offsets),
        ReferenceMap::RepeatedReference {
            count,
            stride,
            local_offsets,
            ..
        } => ReferenceOffsets::Repeated {
            count: *count,
            stride: *stride,
            offsets: local_offsets,
        },
    }
}

/// Return the shared-reference offsets for one trace.
fn shared_reference_offsets(reference_map: &ReferenceMap) -> ReferenceOffsets<'_> {
    match reference_map {
        ReferenceMap::None => ReferenceOffsets::None,
        ReferenceMap::Reference { shared_offsets, .. } => ReferenceOffsets::Direct(shared_offsets),
        ReferenceMap::RepeatedReference {
            count,
            stride,
            shared_offsets,
            ..
        } => ReferenceOffsets::Repeated {
            count: *count,
            stride: *stride,
            offsets: shared_offsets,
        },
    }
}

/// Visit each traced reference through one random-access reader.
fn visit_reference_map_in_reader<R: TracedReference>(
    reference_offsets: ReferenceOffsets<'_>,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(R),
) -> HeapResult<()> {
    match reference_offsets {
        ReferenceOffsets::None => Ok(()),
        ReferenceOffsets::Direct(offsets) => {
            visit_direct_reference_offsets_in_reader(offsets, &mut fill_bytes, &mut visit)
        }
        ReferenceOffsets::Repeated {
            count,
            stride,
            offsets,
        } => visit_repeated_reference_offsets_in_reader(
            count,
            stride,
            offsets,
            &mut fill_bytes,
            &mut visit,
        ),
    }
}

/// Visit each overlapping traced reference through one random-access reader.
fn visit_trace_references_in_reader_range<R: TracedReference>(
    reference_offsets: ReferenceOffsets<'_>,
    start: usize,
    len: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(R),
) -> HeapResult<()> {
    // empty writes cannot overlap anything
    if len == 0 {
        return Ok(());
    }

    // reject invalid dirty windows
    let Some(end) = start.checked_add(len) else {
        return Err(HeapError::TraceOffsetOverflow { start, width: len });
    };

    match reference_offsets {
        ReferenceOffsets::None => Ok(()),
        ReferenceOffsets::Direct(offsets) => visit_direct_reference_offsets_in_reader_range(
            offsets,
            start,
            end,
            &mut fill_bytes,
            &mut visit,
        ),
        ReferenceOffsets::Repeated {
            count,
            stride,
            offsets,
        } => visit_repeated_reference_offsets_in_reader_range(
            count,
            stride,
            offsets,
            start,
            end,
            &mut fill_bytes,
            &mut visit,
        ),
    }
}

/// Decode one traced reference from one native-width window.
fn decode_reference_window<R: TracedReference>(window: &[u8]) -> HeapResult<R> {
    if window.len() != R::BYTE_LEN {
        return Err(HeapError::InvalidReferenceWindowWidth {
            bytes: window.len(),
        });
    }

    let mut raw = [0u8; std::mem::size_of::<usize>()];
    raw.copy_from_slice(window);

    Ok(R::from_bits(usize::from_le_bytes(raw)))
}

/// Encode one direct reference-offset table into one slot bitmap.
fn set_reference_offsets(
    reference_bits: &mut Bitmap,
    slot_index: usize,
    size_class: usize,
    offsets: &[u32],
) {
    let word_bytes = std::mem::size_of::<usize>();
    let word_count = size_class.div_ceil(word_bytes);
    let bit_start = slot_index.saturating_mul(word_count);

    for offset in offsets.iter().copied() {
        let word_index = (offset as usize) / word_bytes;
        let bit_index = bit_start.saturating_add(word_index);

        reference_bits.set(bit_index);
    }
}

/// Encode one repeated reference-offset table into one slot bitmap.
fn set_repeated_reference_offsets(
    reference_bits: &mut Bitmap,
    slot_index: usize,
    size_class: usize,
    count: u32,
    stride: u32,
    offsets: &[u32],
) {
    let word_bytes = std::mem::size_of::<usize>();
    let word_count = size_class.div_ceil(word_bytes);
    let bit_start = slot_index.saturating_mul(word_count);

    for index in 0..count as usize {
        let element_offset = index.saturating_mul(stride as usize);

        for offset in offsets.iter().copied() {
            let byte_offset = element_offset.saturating_add(offset as usize);
            let word_index = byte_offset / word_bytes;
            let bit_index = bit_start.saturating_add(word_index);

            reference_bits.set(bit_index);
        }
    }
}

/// Visit each direct reference offset through one random-access reader.
fn visit_direct_reference_offsets_in_reader<R: TracedReference>(
    offsets: &[u32],
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut window = [0u8; std::mem::size_of::<usize>()];

    for offset in offsets.iter().copied() {
        let start = offset as usize;

        if !fill_bytes(start, &mut window[..R::BYTE_LEN]) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start,
                width: R::BYTE_LEN,
            });
        }

        visit(decode_reference_window::<R>(&window[..R::BYTE_LEN])?);
    }

    Ok(())
}

/// Visit each repeated reference offset through one random-access reader.
fn visit_repeated_reference_offsets_in_reader<R: TracedReference>(
    count: u32,
    stride: u32,
    offsets: &[u32],
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut window = [0u8; std::mem::size_of::<usize>()];

    for index in 0..count as usize {
        let base = index
            .checked_mul(stride as usize)
            .ok_or(HeapError::InvariantOverflow {
                context: "repeated scan base",
            })?;

        for offset in offsets.iter().copied() {
            let start =
                base.checked_add(offset as usize)
                    .ok_or(HeapError::TraceOffsetOverflow {
                        start: base,
                        width: offset as usize,
                    })?;

            if !fill_bytes(start, &mut window[..R::BYTE_LEN]) {
                return Err(HeapError::TruncatedReferenceReaderWindow {
                    start,
                    width: R::BYTE_LEN,
                });
            }

            visit(decode_reference_window::<R>(&window[..R::BYTE_LEN])?);
        }
    }

    Ok(())
}

/// Visit each overlapping direct reference offset through one random-access reader.
fn visit_direct_reference_offsets_in_reader_range<R: TracedReference>(
    offsets: &[u32],
    start: usize,
    end: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut window = [0u8; std::mem::size_of::<usize>()];

    for offset in offsets.iter().copied() {
        let offset = offset as usize;
        if !ranges_overlap(start, end, offset, R::BYTE_LEN) {
            continue;
        }

        if !fill_bytes(offset, &mut window[..R::BYTE_LEN]) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start: offset,
                width: R::BYTE_LEN,
            });
        }

        visit(decode_reference_window::<R>(&window[..R::BYTE_LEN])?);
    }

    Ok(())
}

/// Visit each overlapping repeated reference offset through one random-access reader.
fn visit_repeated_reference_offsets_in_reader_range<R: TracedReference>(
    count: u32,
    stride: u32,
    offsets: &[u32],
    start: usize,
    end: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut window = [0u8; std::mem::size_of::<usize>()];

    for offset in offsets.iter().copied() {
        let Some((first, last)) = overlapping_repeated_index_range(
            start,
            end,
            count as usize,
            stride as usize,
            offset as usize,
            R::BYTE_LEN,
        ) else {
            continue;
        };

        for index in first..=last {
            let base = index
                .checked_mul(stride as usize)
                .ok_or(HeapError::InvariantOverflow {
                    context: "repeated scan base",
                })?;
            let offset =
                base.checked_add(offset as usize)
                    .ok_or(HeapError::TraceOffsetOverflow {
                        start: base,
                        width: offset as usize,
                    })?;

            if !fill_bytes(offset, &mut window[..R::BYTE_LEN]) {
                return Err(HeapError::TruncatedReferenceReaderWindow {
                    start: offset,
                    width: R::BYTE_LEN,
                });
            }

            visit(decode_reference_window::<R>(&window[..R::BYTE_LEN])?);
        }
    }

    Ok(())
}
