use destack_mir::{ReferenceMap, ReferenceVariant};

use crate::allocator::Bitmap;
use crate::{HeapError, HeapReference, HeapResult, SharedHeapReference};

/// Return conservative local reference offsets from one map.
pub(crate) fn local_reference_offsets(reference_map: &ReferenceMap) -> Box<[u32]> {
    let mut offsets = Vec::new();
    append_reference_offsets(reference_map, true, 0, &mut offsets);

    offsets.into_boxed_slice()
}

/// Return conservative shared reference offsets from one map.
pub(crate) fn shared_reference_offsets(reference_map: &ReferenceMap) -> Box<[u32]> {
    let mut offsets = Vec::new();
    append_reference_offsets(reference_map, false, 0, &mut offsets);

    offsets.into_boxed_slice()
}

/// Return exact local-reference offsets selected by mapped payload tags.
pub(crate) fn heap_reference_offsets(
    reference_map: &ReferenceMap,
    base_address: usize,
) -> HeapResult<Vec<usize>> {
    let mut offsets = Vec::new();
    push_reference_offsets_from_memory(reference_map, true, base_address, 0, None, &mut offsets)?;

    Ok(offsets)
}

/// Append concrete reference offsets from one map.
fn append_reference_offsets(
    reference_map: &ReferenceMap,
    is_local: bool,
    base_offset: u32,
    offsets: &mut Vec<u32>,
) {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct {
            local_offsets,
            shared_offsets,
        } => {
            let selected = if is_local {
                local_offsets
            } else {
                shared_offsets
            };

            offsets.extend(selected.iter().map(|offset| base_offset + *offset));
        }
        ReferenceMap::Offset { byte_offset, map } => {
            append_reference_offsets(map, is_local, base_offset + *byte_offset, offsets);
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                append_reference_offsets(map, is_local, base_offset, offsets);
            }
        }
        ReferenceMap::Repeat {
            count,
            stride,
            element,
        } => {
            for index in 0..*count {
                let element_offset = base_offset + index * *stride;

                append_reference_offsets(element, is_local, element_offset, offsets);
            }
        }
        ReferenceMap::Tagged { variants, .. } => {
            for variant in variants {
                let variant_offset = base_offset + variant.payload_offset;

                append_reference_offsets(&variant.map, is_local, variant_offset, offsets);
            }
        }
    }
}

/// Return whether one byte range overlaps flattened reference offsets.
fn reference_offsets_overlap(
    reference_map: &ReferenceMap,
    is_local: bool,
    start: usize,
    len: usize,
    width: usize,
) -> bool {
    if len == 0 {
        return false;
    }

    // tagged maps are conservatively flattened for write barriers
    let end = start + len;
    let offsets = if is_local {
        local_reference_offsets(reference_map)
    } else {
        shared_reference_offsets(reference_map)
    };

    offsets
        .iter()
        .any(|offset| ranges_overlap(start, end, *offset as usize, width))
}

/// One byte range used to filter reference offsets.
#[derive(Debug, Clone, Copy)]
struct ReferenceRange {
    /// The start byte offset.
    start: usize,
    /// The exclusive end byte offset.
    end: usize,
    /// The reference byte width.
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
    // locate this slot in the span reference bitmaps
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

    // empty maps collapse to the noscan form
    if local_offsets.is_empty() && shared_offsets.is_empty() {
        return ReferenceMap::None;
    }

    ReferenceMap::Direct {
        local_offsets: local_offsets.into_boxed_slice(),
        shared_offsets: shared_offsets.into_boxed_slice(),
    }
}

/// Return whether one write range may overlap any local reference bytes.
pub(crate) fn overlaps_heap_range(reference_map: &ReferenceMap, start: usize, len: usize) -> bool {
    reference_offsets_overlap(reference_map, true, start, len, HeapReference::BYTE_LEN)
}

/// Return whether one write range may overlap any shared reference bytes.
pub(crate) fn overlaps_shared_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
) -> bool {
    reference_offsets_overlap(
        reference_map,
        false,
        start,
        len,
        SharedHeapReference::BYTE_LEN,
    )
}

/// Return the exact reference map encoded for one allocation byte range.
pub(crate) fn allocation_reference_map(
    local_reference_bits: &Bitmap,
    shared_reference_bits: &Bitmap,
    byte_offset: usize,
    byte_len: usize,
) -> ReferenceMap {
    // locate this allocation in the side bitmaps
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

    // empty maps collapse to the noscan form
    if local_offsets.is_empty() && shared_offsets.is_empty() {
        return ReferenceMap::None;
    }

    ReferenceMap::Direct {
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
    let right_end = right_start + right_len;

    left_start < right_end && right_start < left_end
}

/// Clear the exact reference bits for one small slot.
pub(crate) fn clear_slot_reference_bits(
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    slot_index: usize,
    size_class: usize,
) {
    // map the slot payload to bitmap word indexes
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
    // map the allocation payload to bitmap word indexes
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
) {
    debug_assert!(!reference_map.has_tagged_reference());

    // noscan layouts have no side bits
    if !reference_map.has_reference() {
        return;
    }

    // clear stale reference bits before writing exact offsets
    clear_slot_reference_bits(
        local_reference_bits,
        shared_reference_bits,
        slot_index,
        size_class,
    );

    // encode local and shared reference offsets independently
    set_slot_reference_offsets(
        local_reference_bits,
        slot_index,
        size_class,
        &local_reference_offsets(reference_map),
    );
    set_slot_reference_offsets(
        shared_reference_bits,
        slot_index,
        size_class,
        &shared_reference_offsets(reference_map),
    );
}

/// Encode one exact reference map into one allocation byte range.
pub(crate) fn write_allocation_reference_bits(
    reference_map: &ReferenceMap,
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    byte_offset: usize,
    byte_len: usize,
) {
    debug_assert!(!reference_map.has_tagged_reference());

    // noscan layouts have no side bits
    if !reference_map.has_reference() {
        return;
    }

    // clear stale reference bits before writing exact offsets
    clear_allocation_reference_bits(
        local_reference_bits,
        shared_reference_bits,
        byte_offset,
        byte_len,
    );

    // encode local and shared reference offsets independently
    set_allocation_reference_offsets(
        local_reference_bits,
        byte_offset,
        &local_reference_offsets(reference_map),
    );
    set_allocation_reference_offsets(
        shared_reference_bits,
        byte_offset,
        &shared_reference_offsets(reference_map),
    );
}

/// Scan heap references encoded in the given payload bytes.
pub fn scan_heap_references_in_bytes(
    reference_map: &ReferenceMap,
    bytes: &[u8],
    references: &mut Vec<HeapReference>,
) -> HeapResult<()> {
    // scan the complete byte payload
    scan_heap_references_in_bytes_range(reference_map, 0, bytes.len(), bytes, references)
}

/// Scan heap references from one mapped allocation base address.
pub(crate) fn scan_heap_references(
    reference_map: &ReferenceMap,
    base_address: usize,
    references: &mut Vec<HeapReference>,
) -> HeapResult<()> {
    push_heap_references_from_memory(reference_map, base_address, 0, None, references)
}

/// Scan shared heap references from one mapped allocation base address.
pub(crate) fn scan_shared_references(
    reference_map: &ReferenceMap,
    base_address: usize,
    references: &mut Vec<SharedHeapReference>,
) -> HeapResult<()> {
    push_shared_references_from_memory(reference_map, base_address, 0, None, references)
}

/// Scan overlapping heap references from one mapped allocation base address.
pub(crate) fn scan_heap_references_in_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
    base_address: usize,
    references: &mut Vec<HeapReference>,
) -> HeapResult<()> {
    let range = ReferenceRange {
        start,
        end: start + len,
        width: HeapReference::BYTE_LEN,
    };

    push_heap_references_from_memory(reference_map, base_address, 0, Some(range), references)
}

/// Scan overlapping shared heap references from one mapped allocation base address.
pub(crate) fn scan_shared_references_in_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
    base_address: usize,
    references: &mut Vec<SharedHeapReference>,
) -> HeapResult<()> {
    let range = ReferenceRange {
        start,
        end: start + len,
        width: SharedHeapReference::BYTE_LEN,
    };

    push_shared_references_from_memory(reference_map, base_address, 0, Some(range), references)
}

/// Scan overlapping heap references from one byte window.
pub(crate) fn scan_heap_references_in_bytes_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
    bytes: &[u8],
    references: &mut Vec<HeapReference>,
) -> HeapResult<()> {
    let range = ReferenceRange {
        start,
        end: start + len,
        width: HeapReference::BYTE_LEN,
    };

    push_heap_references_from_bytes(reference_map, start, bytes, 0, Some(range), references)
}

/// Scan overlapping shared heap references from one byte window.
pub(crate) fn scan_shared_references_in_bytes_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
    bytes: &[u8],
    references: &mut Vec<SharedHeapReference>,
) -> HeapResult<()> {
    let range = ReferenceRange {
        start,
        end: start + len,
        width: SharedHeapReference::BYTE_LEN,
    };

    push_shared_references_from_bytes(reference_map, start, bytes, 0, Some(range), references)
}

/// Push heap references from mapped payload memory.
fn push_heap_references_from_memory(
    reference_map: &ReferenceMap,
    base_address: usize,
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<HeapReference>,
) -> HeapResult<()> {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct { local_offsets, .. } => {
            push_direct_heap_references(
                local_offsets,
                base_address,
                base_offset,
                range,
                references,
            );
        }
        ReferenceMap::Offset { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            push_heap_references_from_memory(map, base_address, byte_offset, range, references)?;
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                push_heap_references_from_memory(
                    map,
                    base_address,
                    base_offset,
                    range,
                    references,
                )?;
            }
        }
        ReferenceMap::Repeat {
            count,
            stride,
            element,
        } => {
            for index in 0..*count {
                let element_offset = base_offset + index as usize * *stride as usize;

                push_heap_references_from_memory(
                    element,
                    base_address,
                    element_offset,
                    range,
                    references,
                )?;
            }
        }
        ReferenceMap::Tagged {
            tag_offset,
            tag_bytes,
            variants,
        } => {
            let tag = unsafe {
                read_reference_tag(
                    base_address + base_offset + *tag_offset as usize,
                    *tag_bytes,
                )
            };
            let Some(variant) = variants.iter().find(|variant| variant.tag == tag) else {
                return Ok(());
            };
            let variant_offset = base_offset + variant.payload_offset as usize;

            push_heap_references_from_memory(
                &variant.map,
                base_address,
                variant_offset,
                range,
                references,
            )?;
        }
    }

    Ok(())
}

/// Push shared heap references from mapped payload memory.
fn push_shared_references_from_memory(
    reference_map: &ReferenceMap,
    base_address: usize,
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<SharedHeapReference>,
) -> HeapResult<()> {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct { shared_offsets, .. } => {
            push_direct_shared_references(
                shared_offsets,
                base_address,
                base_offset,
                range,
                references,
            );
        }
        ReferenceMap::Offset { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            push_shared_references_from_memory(map, base_address, byte_offset, range, references)?;
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                push_shared_references_from_memory(
                    map,
                    base_address,
                    base_offset,
                    range,
                    references,
                )?;
            }
        }
        ReferenceMap::Repeat {
            count,
            stride,
            element,
        } => {
            for index in 0..*count {
                let element_offset = base_offset + index as usize * *stride as usize;

                push_shared_references_from_memory(
                    element,
                    base_address,
                    element_offset,
                    range,
                    references,
                )?;
            }
        }
        ReferenceMap::Tagged {
            tag_offset,
            tag_bytes,
            variants,
        } => {
            let tag = unsafe {
                read_reference_tag(
                    base_address + base_offset + *tag_offset as usize,
                    *tag_bytes,
                )
            };
            let Some(variant) = variants.iter().find(|variant| variant.tag == tag) else {
                return Ok(());
            };
            let variant_offset = base_offset + variant.payload_offset as usize;

            push_shared_references_from_memory(
                &variant.map,
                base_address,
                variant_offset,
                range,
                references,
            )?;
        }
    }

    Ok(())
}

/// Push heap references from caller-provided bytes.
fn push_heap_references_from_bytes(
    reference_map: &ReferenceMap,
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<HeapReference>,
) -> HeapResult<()> {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct { local_offsets, .. } => {
            push_direct_heap_references_from_bytes(
                local_offsets,
                start,
                bytes,
                base_offset,
                range,
                references,
            )?;
        }
        ReferenceMap::Offset { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            push_heap_references_from_bytes(map, start, bytes, byte_offset, range, references)?;
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                push_heap_references_from_bytes(map, start, bytes, base_offset, range, references)?;
            }
        }
        ReferenceMap::Repeat {
            count,
            stride,
            element,
        } => {
            for index in 0..*count {
                let element_offset = base_offset + index as usize * *stride as usize;

                push_heap_references_from_bytes(
                    element,
                    start,
                    bytes,
                    element_offset,
                    range,
                    references,
                )?;
            }
        }
        ReferenceMap::Tagged {
            tag_offset,
            tag_bytes,
            variants,
        } => {
            let tag_offset = base_offset + *tag_offset as usize;
            let Some(tag) = reference_tag_from_bytes(bytes, start, tag_offset, *tag_bytes) else {
                push_heap_reference_variants_from_bytes(
                    variants,
                    start,
                    bytes,
                    base_offset,
                    range,
                    references,
                )?;

                return Ok(());
            };
            let Some(variant) = variants.iter().find(|variant| variant.tag == tag) else {
                return Ok(());
            };
            let variant_offset = base_offset + variant.payload_offset as usize;

            push_heap_references_from_bytes(
                &variant.map,
                start,
                bytes,
                variant_offset,
                range,
                references,
            )?;
        }
    }

    Ok(())
}

/// Push shared heap references from caller-provided bytes.
fn push_shared_references_from_bytes(
    reference_map: &ReferenceMap,
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<SharedHeapReference>,
) -> HeapResult<()> {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct { shared_offsets, .. } => {
            push_direct_shared_references_from_bytes(
                shared_offsets,
                start,
                bytes,
                base_offset,
                range,
                references,
            )?;
        }
        ReferenceMap::Offset { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            push_shared_references_from_bytes(map, start, bytes, byte_offset, range, references)?;
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                push_shared_references_from_bytes(
                    map,
                    start,
                    bytes,
                    base_offset,
                    range,
                    references,
                )?;
            }
        }
        ReferenceMap::Repeat {
            count,
            stride,
            element,
        } => {
            for index in 0..*count {
                let element_offset = base_offset + index as usize * *stride as usize;

                push_shared_references_from_bytes(
                    element,
                    start,
                    bytes,
                    element_offset,
                    range,
                    references,
                )?;
            }
        }
        ReferenceMap::Tagged {
            tag_offset,
            tag_bytes,
            variants,
        } => {
            let tag_offset = base_offset + *tag_offset as usize;
            let Some(tag) = reference_tag_from_bytes(bytes, start, tag_offset, *tag_bytes) else {
                push_shared_reference_variants_from_bytes(
                    variants,
                    start,
                    bytes,
                    base_offset,
                    range,
                    references,
                )?;

                return Ok(());
            };
            let Some(variant) = variants.iter().find(|variant| variant.tag == tag) else {
                return Ok(());
            };
            let variant_offset = base_offset + variant.payload_offset as usize;

            push_shared_references_from_bytes(
                &variant.map,
                start,
                bytes,
                variant_offset,
                range,
                references,
            )?;
        }
    }

    Ok(())
}

/// Push direct heap references from mapped payload memory.
fn push_direct_heap_references(
    offsets: &[u32],
    base_address: usize,
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<HeapReference>,
) {
    for offset in offsets {
        let offset = base_offset + *offset as usize;
        if !reference_offset_overlaps_range(offset, range) {
            continue;
        }

        let bits = unsafe { read_reference_bits(base_address + offset) };
        references.push(HeapReference::from_bits(bits));
    }
}

/// Push direct shared references from mapped payload memory.
fn push_direct_shared_references(
    offsets: &[u32],
    base_address: usize,
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<SharedHeapReference>,
) {
    for offset in offsets {
        let offset = base_offset + *offset as usize;
        if !reference_offset_overlaps_range(offset, range) {
            continue;
        }

        let bits = unsafe { read_reference_bits(base_address + offset) };
        references.push(SharedHeapReference::from_bits(bits));
    }
}

/// Push direct heap references from caller-provided bytes.
fn push_direct_heap_references_from_bytes(
    offsets: &[u32],
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<HeapReference>,
) -> HeapResult<()> {
    for offset in offsets {
        let offset = base_offset + *offset as usize;
        if !reference_offset_overlaps_range(offset, range) {
            continue;
        }

        let bits = reference_bits_from_bytes(bytes, start, offset)?;
        references.push(HeapReference::from_bits(bits));
    }

    Ok(())
}

/// Push direct shared references from caller-provided bytes.
fn push_direct_shared_references_from_bytes(
    offsets: &[u32],
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<SharedHeapReference>,
) -> HeapResult<()> {
    for offset in offsets {
        let offset = base_offset + *offset as usize;
        if !reference_offset_overlaps_range(offset, range) {
            continue;
        }

        let bits = reference_bits_from_bytes(bytes, start, offset)?;
        references.push(SharedHeapReference::from_bits(bits));
    }

    Ok(())
}

/// Push heap references from all tagged variants in one byte window.
fn push_heap_reference_variants_from_bytes(
    variants: &[ReferenceVariant],
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<HeapReference>,
) -> HeapResult<()> {
    for variant in variants {
        let variant_offset = base_offset + variant.payload_offset as usize;

        push_heap_references_from_bytes(
            &variant.map,
            start,
            bytes,
            variant_offset,
            range,
            references,
        )?;
    }

    Ok(())
}

/// Push shared references from all tagged variants in one byte window.
fn push_shared_reference_variants_from_bytes(
    variants: &[ReferenceVariant],
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<SharedHeapReference>,
) -> HeapResult<()> {
    for variant in variants {
        let variant_offset = base_offset + variant.payload_offset as usize;

        push_shared_references_from_bytes(
            &variant.map,
            start,
            bytes,
            variant_offset,
            range,
            references,
        )?;
    }

    Ok(())
}

/// Push reference offsets selected by mapped payload tags.
fn push_reference_offsets_from_memory(
    reference_map: &ReferenceMap,
    is_local: bool,
    base_address: usize,
    base_offset: usize,
    range: Option<ReferenceRange>,
    offsets: &mut Vec<usize>,
) -> HeapResult<()> {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct {
            local_offsets,
            shared_offsets,
        } => {
            let selected = if is_local {
                local_offsets
            } else {
                shared_offsets
            };

            push_direct_reference_offsets(selected, base_offset, range, offsets);
        }
        ReferenceMap::Offset { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            push_reference_offsets_from_memory(
                map,
                is_local,
                base_address,
                byte_offset,
                range,
                offsets,
            )?;
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                push_reference_offsets_from_memory(
                    map,
                    is_local,
                    base_address,
                    base_offset,
                    range,
                    offsets,
                )?;
            }
        }
        ReferenceMap::Repeat {
            count,
            stride,
            element,
        } => {
            for index in 0..*count {
                let element_offset = base_offset + index as usize * *stride as usize;

                push_reference_offsets_from_memory(
                    element,
                    is_local,
                    base_address,
                    element_offset,
                    range,
                    offsets,
                )?;
            }
        }
        ReferenceMap::Tagged {
            tag_offset,
            tag_bytes,
            variants,
        } => {
            let tag = unsafe {
                read_reference_tag(
                    base_address + base_offset + *tag_offset as usize,
                    *tag_bytes,
                )
            };
            let Some(variant) = variants.iter().find(|variant| variant.tag == tag) else {
                return Ok(());
            };
            let variant_offset = base_offset + variant.payload_offset as usize;

            push_reference_offsets_from_memory(
                &variant.map,
                is_local,
                base_address,
                variant_offset,
                range,
                offsets,
            )?;
        }
    }

    Ok(())
}

/// Push direct reference offsets after applying range filtering.
fn push_direct_reference_offsets(
    offsets: &[u32],
    base_offset: usize,
    range: Option<ReferenceRange>,
    target: &mut Vec<usize>,
) {
    for offset in offsets {
        let offset = base_offset + *offset as usize;
        if !reference_offset_overlaps_range(offset, range) {
            continue;
        }

        target.push(offset);
    }
}

/// Return whether one reference offset overlaps an optional byte range.
fn reference_offset_overlaps_range(offset: usize, range: Option<ReferenceRange>) -> bool {
    let Some(range) = range else {
        return true;
    };

    ranges_overlap(range.start, range.end, offset, range.width)
}

/// Return one native-width reference payload from mapped memory.
unsafe fn read_reference_bits(address: usize) -> usize {
    // heap layouts guarantee pointer-width reference fields
    debug_assert_eq!(address % std::mem::align_of::<usize>(), 0);

    unsafe { (address as *const usize).read() }
}

/// Return one unsigned reference-map tag from mapped memory.
unsafe fn read_reference_tag(address: usize, width: u8) -> u64 {
    let mut raw = [0u8; std::mem::size_of::<u64>()];

    for (index, byte) in raw.iter_mut().take(width as usize).enumerate() {
        *byte = unsafe { (address as *const u8).add(index).read() };
    }

    u64::from_le_bytes(raw)
}

/// Return one unsigned reference-map tag from caller-provided bytes.
fn reference_tag_from_bytes(bytes: &[u8], start: usize, offset: usize, width: u8) -> Option<u64> {
    if offset < start {
        return None;
    }

    let local_start = offset - start;
    let local_end = local_start + width as usize;
    let window = bytes.get(local_start..local_end)?;

    let mut raw = [0u8; std::mem::size_of::<u64>()];
    raw[..width as usize].copy_from_slice(window);

    Some(u64::from_le_bytes(raw))
}

/// Return one native-width reference payload from caller-provided bytes.
fn reference_bits_from_bytes(bytes: &[u8], start: usize, offset: usize) -> HeapResult<usize> {
    // project the absolute reference offset into the byte window
    let local_start = offset - start;
    let local_end = local_start + std::mem::size_of::<usize>();
    let Some(window) = bytes.get(local_start..local_end) else {
        return Err(HeapError::TruncatedReferenceBytes {
            start: local_start,
            width: std::mem::size_of::<usize>(),
        });
    };

    // decode the native-width reference payload
    let mut raw = [0u8; std::mem::size_of::<usize>()];
    raw.copy_from_slice(window);

    Ok(usize::from_le_bytes(raw))
}

/// Encode direct reference offsets into one slot bitmap.
fn set_slot_reference_offsets(
    reference_bits: &mut Bitmap,
    slot_index: usize,
    size_class: usize,
    offsets: &[u32],
) {
    // derive the slot bitmap base
    let word_bytes = std::mem::size_of::<usize>();
    let word_count = size_class.div_ceil(word_bytes);
    let bit_start = slot_index * word_count;

    // set one bitmap bit per reference word
    for offset in offsets {
        let word_index = *offset as usize / word_bytes;
        let bit_index = bit_start + word_index;

        reference_bits.set(bit_index);
    }
}

/// Encode direct reference offsets into one allocation bitmap.
fn set_allocation_reference_offsets(
    reference_bits: &mut Bitmap,
    byte_offset: usize,
    offsets: &[u32],
) {
    // derive allocation bitmap word indexes
    let word_bytes = std::mem::size_of::<usize>();

    // set one bitmap bit per reference word
    for offset in offsets {
        let absolute_offset = byte_offset + *offset as usize;
        let bit_index = absolute_offset / word_bytes;

        reference_bits.set(bit_index);
    }
}
