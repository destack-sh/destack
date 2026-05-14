use destack_mir::{ReferenceMap, ReferenceVariant};

use crate::allocator::Bitmap;
use crate::{HeapError, HeapReference, HeapResult, RootSlot, SharedHeapReference};

/// Native reference field width.
const REFERENCE_BYTES: usize = std::mem::size_of::<usize>();

/// Return conservative local reference offsets from one map.
pub(crate) fn local_reference_offsets(reference_map: &ReferenceMap) -> Box<[u32]> {
    let mut offsets = Vec::new();
    append_reference_offsets::<LocalReference>(reference_map, 0, &mut offsets);

    offsets.into_boxed_slice()
}

/// Return conservative shared reference offsets from one map.
pub(crate) fn shared_reference_offsets(reference_map: &ReferenceMap) -> Box<[u32]> {
    let mut offsets = Vec::new();
    append_reference_offsets::<SharedReference>(reference_map, 0, &mut offsets);

    offsets.into_boxed_slice()
}

/// Return exact local-reference offsets selected by mapped payload tags.
pub(crate) fn heap_reference_offsets(
    reference_map: &ReferenceMap,
    base_address: usize,
) -> HeapResult<Vec<usize>> {
    let mut offsets = Vec::new();
    push_reference_offsets_from_memory::<LocalReference>(
        reference_map,
        base_address,
        0,
        None,
        &mut offsets,
    )?;

    Ok(offsets)
}

/// Append concrete reference offsets from one map.
fn append_reference_offsets<R: ReferenceScan>(
    reference_map: &ReferenceMap,
    base_offset: u32,
    offsets: &mut Vec<u32>,
) {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct { .. } => {
            offsets.extend(
                R::offsets(reference_map)
                    .iter()
                    .map(|offset| base_offset + *offset),
            );
        }
        ReferenceMap::Offset { byte_offset, map } => {
            append_reference_offsets::<R>(map, base_offset + *byte_offset, offsets);
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                append_reference_offsets::<R>(map, base_offset, offsets);
            }
        }
        ReferenceMap::Repeat {
            count,
            stride,
            element,
        } => {
            for index in 0..*count {
                let element_offset = base_offset + index * *stride;

                append_reference_offsets::<R>(element, element_offset, offsets);
            }
        }
        ReferenceMap::Tagged { variants, .. } => {
            for variant in variants {
                let variant_offset = base_offset + variant.payload_offset;

                append_reference_offsets::<R>(&variant.map, variant_offset, offsets);
            }
        }
    }
}

/// Return whether one byte range overlaps flattened reference offsets.
fn reference_offsets_overlap<R: ReferenceScan>(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
) -> bool {
    if len == 0 {
        return false;
    }

    // tagged maps are conservatively flattened for write barriers
    let end = start + len;
    let mut offsets = Vec::new();
    append_reference_offsets::<R>(reference_map, 0, &mut offsets);

    offsets
        .iter()
        .any(|offset| ranges_overlap(start, end, *offset as usize, R::BYTE_LEN))
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

/// Reference class selected from one reference map.
trait ReferenceScan {
    /// Concrete reference type produced by the scan.
    type Reference;
    /// The encoded byte width.
    const BYTE_LEN: usize;

    /// Return the offsets for this reference class.
    fn offsets(reference_map: &ReferenceMap) -> &[u32];

    /// Decode one reference from native bits.
    fn from_bits(bits: usize) -> Self::Reference;
}

/// Worker-local reference fields.
struct LocalReference;

impl ReferenceScan for LocalReference {
    type Reference = HeapReference;
    const BYTE_LEN: usize = HeapReference::BYTE_LEN;

    fn offsets(reference_map: &ReferenceMap) -> &[u32] {
        let ReferenceMap::Direct { local_offsets, .. } = reference_map else {
            return &[];
        };

        local_offsets
    }

    fn from_bits(bits: usize) -> Self::Reference {
        HeapReference::from_bits(bits)
    }
}

/// Runtime-shared reference fields.
struct SharedReference;

impl ReferenceScan for SharedReference {
    type Reference = SharedHeapReference;
    const BYTE_LEN: usize = SharedHeapReference::BYTE_LEN;

    fn offsets(reference_map: &ReferenceMap) -> &[u32] {
        let ReferenceMap::Direct { shared_offsets, .. } = reference_map else {
            return &[];
        };

        shared_offsets
    }

    fn from_bits(bits: usize) -> Self::Reference {
        SharedHeapReference::from_bits(bits)
    }
}

/// Heap reference edge encoded in object or frame bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeapEdge {
    /// Worker-local heap reference.
    Local(HeapReference),
    /// Runtime-shared heap reference.
    Shared(SharedHeapReference),
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
    let word_count = size_class.div_ceil(REFERENCE_BYTES);
    let bit_start = slot_index * word_count;

    direct_reference_map(
        local_reference_bits,
        shared_reference_bits,
        bit_start,
        word_count,
        byte_len,
    )
}

/// Return the exact reference map encoded in one bitmap range.
fn direct_reference_map(
    local_reference_bits: &Bitmap,
    shared_reference_bits: &Bitmap,
    bit_start: usize,
    word_count: usize,
    byte_len: usize,
) -> ReferenceMap {
    let mut local_offsets = Vec::new();
    let mut shared_offsets = Vec::new();

    // decode one direct map from side bits
    for word_index in 0..word_count {
        let bit_index = bit_start + word_index;
        let byte_offset = word_index * REFERENCE_BYTES;
        let byte_end = byte_offset + REFERENCE_BYTES;
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
    reference_offsets_overlap::<LocalReference>(reference_map, start, len)
}

/// Return whether one write range may overlap any shared reference bytes.
pub(crate) fn overlaps_shared_range(
    reference_map: &ReferenceMap,
    start: usize,
    len: usize,
) -> bool {
    reference_offsets_overlap::<SharedReference>(reference_map, start, len)
}

/// Return the exact reference map encoded for one allocation byte range.
pub(crate) fn allocation_reference_map(
    local_reference_bits: &Bitmap,
    shared_reference_bits: &Bitmap,
    byte_offset: usize,
    byte_len: usize,
) -> ReferenceMap {
    // locate this allocation in the side bitmaps
    let bit_start = byte_offset / REFERENCE_BYTES;
    let word_count = byte_len.div_ceil(REFERENCE_BYTES);

    direct_reference_map(
        local_reference_bits,
        shared_reference_bits,
        bit_start,
        word_count,
        byte_len,
    )
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
    let bit_len = size_class.div_ceil(REFERENCE_BYTES);
    let bit_start = slot_index * bit_len;

    clear_reference_bits(
        local_reference_bits,
        shared_reference_bits,
        bit_start,
        bit_len,
    );
}

/// Clear the exact reference bits for one allocation byte range.
pub(crate) fn clear_allocation_reference_bits(
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    byte_offset: usize,
    byte_len: usize,
) {
    // map the allocation payload to bitmap word indexes
    let bit_start = byte_offset / REFERENCE_BYTES;
    let bit_len = byte_len.div_ceil(REFERENCE_BYTES);

    clear_reference_bits(
        local_reference_bits,
        shared_reference_bits,
        bit_start,
        bit_len,
    );
}

/// Clear one exact reference bitmap range.
fn clear_reference_bits(
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    bit_start: usize,
    bit_len: usize,
) {
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

    // derive the slot bitmap range
    let bit_len = size_class.div_ceil(REFERENCE_BYTES);
    let bit_start = slot_index * bit_len;

    write_direct_reference_bits(
        reference_map,
        local_reference_bits,
        shared_reference_bits,
        bit_start,
        bit_len,
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

    // derive the allocation bitmap range
    let bit_start = byte_offset / REFERENCE_BYTES;
    let bit_len = byte_len.div_ceil(REFERENCE_BYTES);

    write_direct_reference_bits(
        reference_map,
        local_reference_bits,
        shared_reference_bits,
        bit_start,
        bit_len,
    );
}

/// Visit heap edges encoded in the given payload bytes.
pub fn visit_heap_edges_in_bytes(
    reference_map: &ReferenceMap,
    bytes: &[u8],
    visit: &mut dyn FnMut(HeapEdge) -> HeapResult<()>,
) -> HeapResult<()> {
    visit_heap_edges_from_bytes(reference_map, 0, bytes, 0, None, visit)
}

/// Visit mutable local heap reference slots encoded in the given payload bytes.
pub fn visit_heap_root_slots_in_bytes(
    reference_map: &ReferenceMap,
    bytes: &mut [u8],
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> HeapResult<()> {
    visit_heap_root_slots_from_bytes(reference_map, 0, bytes, 0, None, visit)
}

/// Scan heap references from one mapped allocation base address.
pub(crate) fn scan_heap_references(
    reference_map: &ReferenceMap,
    base_address: usize,
    references: &mut Vec<HeapReference>,
) -> HeapResult<()> {
    push_references_from_memory::<LocalReference>(reference_map, base_address, 0, None, references)
}

/// Scan shared heap references from one mapped allocation base address.
pub(crate) fn scan_shared_references(
    reference_map: &ReferenceMap,
    base_address: usize,
    references: &mut Vec<SharedHeapReference>,
) -> HeapResult<()> {
    push_references_from_memory::<SharedReference>(reference_map, base_address, 0, None, references)
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

    push_references_from_memory::<LocalReference>(
        reference_map,
        base_address,
        0,
        Some(range),
        references,
    )
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

    push_references_from_memory::<SharedReference>(
        reference_map,
        base_address,
        0,
        Some(range),
        references,
    )
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

    visit_references_from_bytes::<SharedReference>(
        reference_map,
        start,
        bytes,
        0,
        Some(range),
        &mut |reference| {
            references.push(reference);

            Ok(())
        },
    )
}

/// Push references from mapped payload memory.
fn push_references_from_memory<R: ReferenceScan>(
    reference_map: &ReferenceMap,
    base_address: usize,
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<R::Reference>,
) -> HeapResult<()> {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct { .. } => {
            push_direct_references::<R>(
                R::offsets(reference_map),
                base_address,
                base_offset,
                range,
                references,
            );
        }
        ReferenceMap::Offset { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            push_references_from_memory::<R>(map, base_address, byte_offset, range, references)?;
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                push_references_from_memory::<R>(
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

                push_references_from_memory::<R>(
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

            push_references_from_memory::<R>(
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

/// Visit references from caller-provided bytes.
fn visit_references_from_bytes<R: ReferenceScan>(
    reference_map: &ReferenceMap,
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    visit: &mut dyn FnMut(R::Reference) -> HeapResult<()>,
) -> HeapResult<()> {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct { .. } => {
            visit_direct_references_from_bytes::<R>(
                R::offsets(reference_map),
                start,
                bytes,
                base_offset,
                range,
                visit,
            )?;
        }
        ReferenceMap::Offset { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            visit_references_from_bytes::<R>(map, start, bytes, byte_offset, range, visit)?;
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                visit_references_from_bytes::<R>(map, start, bytes, base_offset, range, visit)?;
            }
        }
        ReferenceMap::Repeat {
            count,
            stride,
            element,
        } => {
            for index in 0..*count {
                let element_offset = base_offset + index as usize * *stride as usize;

                visit_references_from_bytes::<R>(
                    element,
                    start,
                    bytes,
                    element_offset,
                    range,
                    visit,
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
                visit_reference_variants_from_bytes::<R>(
                    variants,
                    start,
                    bytes,
                    base_offset,
                    range,
                    visit,
                )?;

                return Ok(());
            };
            let Some(variant) = variants.iter().find(|variant| variant.tag == tag) else {
                return Ok(());
            };
            let variant_offset = base_offset + variant.payload_offset as usize;

            visit_references_from_bytes::<R>(
                &variant.map,
                start,
                bytes,
                variant_offset,
                range,
                visit,
            )?;
        }
    }

    Ok(())
}

/// Visit heap edges from caller-provided bytes.
fn visit_heap_edges_from_bytes(
    reference_map: &ReferenceMap,
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    visit: &mut dyn FnMut(HeapEdge) -> HeapResult<()>,
) -> HeapResult<()> {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct {
            local_offsets,
            shared_offsets,
        } => {
            visit_direct_references_from_bytes::<LocalReference>(
                local_offsets,
                start,
                bytes,
                base_offset,
                range,
                &mut |reference| visit(HeapEdge::Local(reference)),
            )?;
            visit_direct_references_from_bytes::<SharedReference>(
                shared_offsets,
                start,
                bytes,
                base_offset,
                range,
                &mut |reference| visit(HeapEdge::Shared(reference)),
            )?;
        }
        ReferenceMap::Offset { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            visit_heap_edges_from_bytes(map, start, bytes, byte_offset, range, visit)?;
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                visit_heap_edges_from_bytes(map, start, bytes, base_offset, range, visit)?;
            }
        }
        ReferenceMap::Repeat {
            count,
            stride,
            element,
        } => {
            for index in 0..*count {
                let element_offset = base_offset + index as usize * *stride as usize;

                visit_heap_edges_from_bytes(element, start, bytes, element_offset, range, visit)?;
            }
        }
        ReferenceMap::Tagged {
            tag_offset,
            tag_bytes,
            variants,
        } => {
            let tag_offset = base_offset + *tag_offset as usize;
            let Some(tag) = reference_tag_from_bytes(bytes, start, tag_offset, *tag_bytes) else {
                visit_heap_edge_variants_from_bytes(
                    variants,
                    start,
                    bytes,
                    base_offset,
                    range,
                    visit,
                )?;

                return Ok(());
            };
            let Some(variant) = variants.iter().find(|variant| variant.tag == tag) else {
                return Ok(());
            };
            let variant_offset = base_offset + variant.payload_offset as usize;

            visit_heap_edges_from_bytes(&variant.map, start, bytes, variant_offset, range, visit)?;
        }
    }

    Ok(())
}

/// Push direct references from mapped payload memory.
fn push_direct_references<R: ReferenceScan>(
    offsets: &[u32],
    base_address: usize,
    base_offset: usize,
    range: Option<ReferenceRange>,
    references: &mut Vec<R::Reference>,
) {
    for offset in offsets {
        let offset = base_offset + *offset as usize;
        if !reference_offset_overlaps_range(offset, range) {
            continue;
        }

        let bits = unsafe { read_reference_bits(base_address + offset) };
        references.push(R::from_bits(bits));
    }
}

/// Visit direct references from caller-provided bytes.
fn visit_direct_references_from_bytes<R: ReferenceScan>(
    offsets: &[u32],
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    visit: &mut dyn FnMut(R::Reference) -> HeapResult<()>,
) -> HeapResult<()> {
    for offset in offsets {
        let offset = base_offset + *offset as usize;
        if !reference_offset_overlaps_range(offset, range) {
            continue;
        }

        let bits = reference_bits_from_bytes(bytes, start, offset)?;
        visit(R::from_bits(bits))?;
    }

    Ok(())
}

/// Visit references from all tagged variants in one byte window.
fn visit_reference_variants_from_bytes<R: ReferenceScan>(
    variants: &[ReferenceVariant],
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    visit: &mut dyn FnMut(R::Reference) -> HeapResult<()>,
) -> HeapResult<()> {
    for variant in variants {
        let variant_offset = base_offset + variant.payload_offset as usize;

        visit_references_from_bytes::<R>(&variant.map, start, bytes, variant_offset, range, visit)?;
    }

    Ok(())
}

/// Visit heap edges from all tagged variants in one byte window.
fn visit_heap_edge_variants_from_bytes(
    variants: &[ReferenceVariant],
    start: usize,
    bytes: &[u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    visit: &mut dyn FnMut(HeapEdge) -> HeapResult<()>,
) -> HeapResult<()> {
    for variant in variants {
        let variant_offset = base_offset + variant.payload_offset as usize;

        visit_heap_edges_from_bytes(&variant.map, start, bytes, variant_offset, range, visit)?;
    }

    Ok(())
}

/// Visit mutable heap root slots selected by caller-provided bytes.
fn visit_heap_root_slots_from_bytes(
    reference_map: &ReferenceMap,
    start: usize,
    bytes: &mut [u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> HeapResult<()> {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct {
            local_offsets,
            shared_offsets,
        } => {
            visit_direct_heap_root_slots_from_bytes(
                local_offsets,
                start,
                bytes,
                base_offset,
                range,
                visit,
            )?;
            visit_direct_shared_root_slots_from_bytes(
                shared_offsets,
                start,
                bytes,
                base_offset,
                range,
                visit,
            )?;
        }
        ReferenceMap::Offset { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            visit_heap_root_slots_from_bytes(map, start, bytes, byte_offset, range, visit)?;
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                visit_heap_root_slots_from_bytes(map, start, bytes, base_offset, range, visit)?;
            }
        }
        ReferenceMap::Repeat {
            count,
            stride,
            element,
        } => {
            for index in 0..*count {
                let element_offset = base_offset + index as usize * *stride as usize;

                visit_heap_root_slots_from_bytes(
                    element,
                    start,
                    bytes,
                    element_offset,
                    range,
                    visit,
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
                visit_heap_root_slot_variants_from_bytes(
                    variants,
                    start,
                    bytes,
                    base_offset,
                    range,
                    visit,
                )?;

                return Ok(());
            };
            let Some(variant) = variants.iter().find(|variant| variant.tag == tag) else {
                return Ok(());
            };
            let variant_offset = base_offset + variant.payload_offset as usize;

            visit_heap_root_slots_from_bytes(
                &variant.map,
                start,
                bytes,
                variant_offset,
                range,
                visit,
            )?;
        }
    }

    Ok(())
}

/// Visit direct mutable worker heap root slots from caller-provided bytes.
fn visit_direct_heap_root_slots_from_bytes(
    offsets: &[u32],
    start: usize,
    bytes: &mut [u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> HeapResult<()> {
    for offset in offsets {
        let offset = base_offset + *offset as usize;
        if !reference_offset_overlaps_range(offset, range) {
            continue;
        }

        let local_start = offset - start;
        let local_end = local_start + HeapReference::BYTE_LEN;
        let Some(slot) = bytes.get_mut(local_start..local_end) else {
            return Err(HeapError::TruncatedReferenceBytes {
                start: local_start,
                width: HeapReference::BYTE_LEN,
            });
        };

        visit(RootSlot::HeapBytes(slot))?;
    }

    Ok(())
}

/// Visit direct mutable runtime heap root slots from caller-provided bytes.
fn visit_direct_shared_root_slots_from_bytes(
    offsets: &[u32],
    start: usize,
    bytes: &mut [u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> HeapResult<()> {
    for offset in offsets {
        let offset = base_offset + *offset as usize;
        if !reference_offset_overlaps_range(offset, range) {
            continue;
        }

        let local_start = offset - start;
        let local_end = local_start + SharedHeapReference::BYTE_LEN;
        let Some(slot) = bytes.get_mut(local_start..local_end) else {
            return Err(HeapError::TruncatedReferenceBytes {
                start: local_start,
                width: SharedHeapReference::BYTE_LEN,
            });
        };

        visit(RootSlot::SharedHeapBytes(slot))?;
    }

    Ok(())
}

/// Visit mutable heap root slots from all tagged variants in one byte window.
fn visit_heap_root_slot_variants_from_bytes(
    variants: &[ReferenceVariant],
    start: usize,
    bytes: &mut [u8],
    base_offset: usize,
    range: Option<ReferenceRange>,
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> HeapResult<()> {
    for variant in variants {
        let variant_offset = base_offset + variant.payload_offset as usize;

        visit_heap_root_slots_from_bytes(&variant.map, start, bytes, variant_offset, range, visit)?;
    }

    Ok(())
}

/// Push reference offsets selected by mapped payload tags.
fn push_reference_offsets_from_memory<R: ReferenceScan>(
    reference_map: &ReferenceMap,
    base_address: usize,
    base_offset: usize,
    range: Option<ReferenceRange>,
    offsets: &mut Vec<usize>,
) -> HeapResult<()> {
    match reference_map {
        ReferenceMap::None => {}
        ReferenceMap::Direct { .. } => {
            push_direct_reference_offsets(R::offsets(reference_map), base_offset, range, offsets);
        }
        ReferenceMap::Offset { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            push_reference_offsets_from_memory::<R>(
                map,
                base_address,
                byte_offset,
                range,
                offsets,
            )?;
        }
        ReferenceMap::Group { maps } => {
            for map in maps {
                push_reference_offsets_from_memory::<R>(
                    map,
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

                push_reference_offsets_from_memory::<R>(
                    element,
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

            push_reference_offsets_from_memory::<R>(
                &variant.map,
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

/// Encode one exact reference map into one bitmap range.
fn write_direct_reference_bits(
    reference_map: &ReferenceMap,
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    bit_start: usize,
    bit_len: usize,
) {
    // clear stale reference bits before writing exact offsets
    clear_reference_bits(
        local_reference_bits,
        shared_reference_bits,
        bit_start,
        bit_len,
    );

    // encode local and shared reference offsets independently
    set_reference_offsets(
        local_reference_bits,
        bit_start,
        &local_reference_offsets(reference_map),
    );
    set_reference_offsets(
        shared_reference_bits,
        bit_start,
        &shared_reference_offsets(reference_map),
    );
}

/// Encode direct reference offsets into one bitmap range.
fn set_reference_offsets(reference_bits: &mut Bitmap, bit_start: usize, offsets: &[u32]) {
    // set one bitmap bit per reference word
    for offset in offsets {
        let word_index = *offset as usize / REFERENCE_BYTES;
        let bit_index = bit_start + word_index;

        reference_bits.set(bit_index);
    }
}
