use destack_mir::TraceMap;

use crate::allocator::Bitmap;
use crate::{
    HeapError, HeapReference, HeapRepresentationError, HeapResult, RootSlot, SharedHeapReference,
};

/// Native reference field width.
const REFERENCE_BYTES: usize = std::mem::size_of::<usize>();

/// One byte range used to filter reference offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceRange {
    /// Every reference offset is in range.
    All,
    /// One byte range is in scope.
    Bytes {
        /// The start byte offset.
        start: usize,
        /// The exclusive end byte offset.
        end: usize,
    },
}

impl ReferenceRange {
    /// Return one byte range.
    pub fn bytes(start: usize, len: usize) -> Self {
        Self::Bytes {
            start,
            end: start + len,
        }
    }

    /// Return whether this range overlaps one reference field.
    fn overlaps(self, offset: usize, width: usize) -> bool {
        match self {
            Self::All => true,
            Self::Bytes { start, end } => ranges_overlap(start, end, offset, width),
        }
    }

    /// Return the repeated element indexes whose strides overlap this range.
    fn element_window(self, base_offset: usize, stride: usize, count: u32) -> std::ops::Range<u32> {
        match self {
            Self::All => 0..count,
            Self::Bytes { start, end } => {
                // empty or fully preceding ranges cover no elements
                if end <= base_offset || stride == 0 {
                    return 0..0;
                }

                // intersect the byte range with the repeated extent
                let first = start.saturating_sub(base_offset) / stride;
                let last = (end - base_offset).div_ceil(stride);
                let first = (first.min(count as usize)) as u32;
                let last = (last.min(count as usize)) as u32;

                first..last
            }
        }
    }
}

/// Reference bytes consumed by one scan.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ReferenceInput<'a> {
    /// Mapped heap memory at a native address.
    Mapped {
        /// The mapped block base address.
        base_address: usize,
    },
    /// Caller-provided object bytes.
    Bytes {
        /// The absolute byte offset represented by `bytes`.
        start: usize,
        /// The byte window to scan.
        bytes: &'a [u8],
    },
}

impl<'a> ReferenceInput<'a> {
    /// Return a mapped-memory scan input.
    pub(crate) fn mapped(base_address: usize) -> Self {
        Self::Mapped { base_address }
    }

    /// Return a byte-window scan input.
    pub(crate) fn bytes(start: usize, bytes: &'a [u8]) -> Self {
        Self::Bytes { start, bytes }
    }
}

/// Reference class selected by one trace map.
pub(crate) trait ReferenceClass: Sized {
    /// The encoded byte width.
    const BYTE_LEN: usize;

    /// Return the offsets for this reference class.
    fn offsets<'a>(local_offsets: &'a [u32], shared_offsets: &'a [u32]) -> &'a [u32];

    /// Decode one reference from native bits.
    fn from_bits(bits: usize) -> Self;
}

/// Worker-local reference fields.
impl ReferenceClass for HeapReference {
    const BYTE_LEN: usize = HeapReference::BYTE_LEN;

    fn offsets<'a>(local_offsets: &'a [u32], _shared_offsets: &'a [u32]) -> &'a [u32] {
        local_offsets
    }

    fn from_bits(bits: usize) -> Self {
        HeapReference::from_bits(bits)
    }
}

/// Runtime-shared reference fields.
impl ReferenceClass for SharedHeapReference {
    const BYTE_LEN: usize = SharedHeapReference::BYTE_LEN;

    fn offsets<'a>(_local_offsets: &'a [u32], shared_offsets: &'a [u32]) -> &'a [u32] {
        shared_offsets
    }

    fn from_bits(bits: usize) -> Self {
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

/// Visit exact reference offsets for one untagged trace map.
pub(crate) fn visit_untagged_reference_offsets<R: ReferenceClass>(
    trace_map: &TraceMap,
    range: ReferenceRange,
    visit: &mut impl FnMut(usize),
) {
    debug_assert!(!trace_map.has_tagged_reference());

    walk_reference_offset_union::<R>(trace_map, 0, range, &mut |offset| {
        visit(offset);

        true
    });
}

/// Walk the union of reference offsets across tagged variants.
fn walk_reference_offset_union<R: ReferenceClass>(
    trace_map: &TraceMap,
    base_offset: usize,
    range: ReferenceRange,
    visit: &mut impl FnMut(usize) -> bool,
) -> bool {
    match trace_map {
        TraceMap::Empty => {}
        TraceMap::Fixed {
            local_offsets,
            shared_offsets,
        } => {
            for offset in R::offsets(local_offsets, shared_offsets) {
                let offset = base_offset + *offset as usize;
                if range.overlaps(offset, R::BYTE_LEN) && !visit(offset) {
                    return false;
                }
            }
        }
        TraceMap::Nested { byte_offset, map } => {
            let base_offset = base_offset + *byte_offset as usize;

            return walk_reference_offset_union::<R>(map, base_offset, range, visit);
        }
        TraceMap::Composite { maps } => {
            for map in maps {
                if !walk_reference_offset_union::<R>(map, base_offset, range, visit) {
                    return false;
                }
            }
        }
        TraceMap::Repeated {
            count,
            stride,
            element,
        } => {
            // element references lie within their stride, so prune by range
            let window = range.element_window(base_offset, *stride as usize, *count);
            for index in window {
                let element_offset = base_offset + index as usize * *stride as usize;

                if !walk_reference_offset_union::<R>(element, element_offset, range, visit) {
                    return false;
                }
            }
        }
        TraceMap::Tagged { variants, .. } => {
            for variant in variants {
                let variant_offset = base_offset + variant.payload_offset as usize;

                if !walk_reference_offset_union::<R>(&variant.map, variant_offset, range, visit) {
                    return false;
                }
            }
        }
    }

    true
}

/// Return the exact trace map encoded for one small slot.
pub(crate) fn slot_trace_map(
    local_reference_bits: &Bitmap,
    shared_reference_bits: &Bitmap,
    slot_index: usize,
    size_class: usize,
    byte_len: usize,
) -> TraceMap {
    slot_trace_map_with(
        |bit_index| local_reference_bits.contains(bit_index),
        |bit_index| shared_reference_bits.contains(bit_index),
        slot_index,
        size_class,
        byte_len,
    )
}

/// Return the exact trace map encoded for one small slot by bit predicates.
pub(crate) fn slot_trace_map_with(
    local_contains: impl Fn(usize) -> bool,
    shared_contains: impl Fn(usize) -> bool,
    slot_index: usize,
    size_class: usize,
    byte_len: usize,
) -> TraceMap {
    // locate this slot in the span reference bitmaps
    let word_count = size_class.div_ceil(REFERENCE_BYTES);
    let bit_start = slot_index * word_count;

    direct_trace_map(
        local_contains,
        shared_contains,
        bit_start,
        word_count,
        byte_len,
    )
}

/// Return the exact trace map encoded in one bit range.
fn direct_trace_map(
    local_contains: impl Fn(usize) -> bool,
    shared_contains: impl Fn(usize) -> bool,
    bit_start: usize,
    word_count: usize,
    byte_len: usize,
) -> TraceMap {
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

        if local_contains(bit_index) {
            local_offsets.push(byte_offset as u32);
        }

        if shared_contains(bit_index) {
            shared_offsets.push(byte_offset as u32);
        }
    }

    // empty maps collapse to the noscan form
    if local_offsets.is_empty() && shared_offsets.is_empty() {
        return TraceMap::Empty;
    }

    TraceMap::Fixed {
        local_offsets: local_offsets.into_boxed_slice(),
        shared_offsets: shared_offsets.into_boxed_slice(),
    }
}

/// Return whether one write range may overlap any local reference bytes.
pub(crate) fn overlaps_heap_range(trace_map: &TraceMap, start: usize, len: usize) -> bool {
    overlaps_reference_range::<HeapReference>(trace_map, start, len)
}

/// Return whether one write range may overlap any shared reference bytes.
pub(crate) fn overlaps_shared_range(trace_map: &TraceMap, start: usize, len: usize) -> bool {
    overlaps_reference_range::<SharedHeapReference>(trace_map, start, len)
}

/// Return whether one byte range may overlap any reference field of one class.
fn overlaps_reference_range<R: ReferenceClass>(
    trace_map: &TraceMap,
    start: usize,
    len: usize,
) -> bool {
    let range = ReferenceRange::bytes(start, len);
    let mut overlaps = false;

    walk_reference_offset_union::<R>(trace_map, 0, range, &mut |_| {
        overlaps = true;

        false
    });

    overlaps
}

/// Return the exact trace map encoded for one block byte range.
pub(crate) fn allocation_trace_map(
    local_reference_bits: &Bitmap,
    shared_reference_bits: &Bitmap,
    byte_offset: usize,
    byte_len: usize,
) -> TraceMap {
    // locate this block in the side bitmaps
    let bit_start = byte_offset / REFERENCE_BYTES;
    let word_count = byte_len.div_ceil(REFERENCE_BYTES);

    direct_trace_map(
        |bit_index| local_reference_bits.contains(bit_index),
        |bit_index| shared_reference_bits.contains(bit_index),
        bit_start,
        word_count,
        byte_len,
    )
}

/// Report whether one byte range overlaps one fixed-width field range.
fn ranges_overlap(
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

/// Clear the exact reference bits for one block byte range.
pub(crate) fn clear_allocation_reference_bits(
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    byte_offset: usize,
    byte_len: usize,
) {
    // map the block payload to bitmap word indexes
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

/// Encode one exact trace map into one small-slot bit range.
pub(crate) fn write_slot_reference_bits(
    trace_map: &TraceMap,
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    slot_index: usize,
    size_class: usize,
) {
    debug_assert!(!trace_map.has_tagged_reference());

    // noscan layouts have no side bits
    if !trace_map.has_reference() {
        return;
    }

    // derive the slot bitmap range
    let bit_len = size_class.div_ceil(REFERENCE_BYTES);
    let bit_start = slot_index * bit_len;

    write_direct_reference_bits(
        trace_map,
        local_reference_bits,
        shared_reference_bits,
        bit_start,
    );
}

/// Encode one exact trace map at one block byte offset.
pub(crate) fn write_allocation_reference_bits(
    trace_map: &TraceMap,
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    byte_offset: usize,
) {
    debug_assert!(!trace_map.has_tagged_reference());

    // noscan layouts have no side bits
    if !trace_map.has_reference() {
        return;
    }

    // derive the block bitmap start
    let bit_start = byte_offset / REFERENCE_BYTES;

    write_direct_reference_bits(
        trace_map,
        local_reference_bits,
        shared_reference_bits,
        bit_start,
    );
}

/// Encode one exact trace map into one clean bitmap range.
fn write_direct_reference_bits(
    trace_map: &TraceMap,
    local_reference_bits: &mut Bitmap,
    shared_reference_bits: &mut Bitmap,
    bit_start: usize,
) {
    // encode local and shared reference offsets independently
    set_reference_bits::<HeapReference>(trace_map, local_reference_bits, bit_start);
    set_reference_bits::<SharedHeapReference>(trace_map, shared_reference_bits, bit_start);
}

/// Encode one reference class into direct bitmap bits.
fn set_reference_bits<R: ReferenceClass>(
    trace_map: &TraceMap,
    reference_bits: &mut Bitmap,
    bit_start: usize,
) {
    debug_assert!(!trace_map.has_tagged_reference());

    visit_untagged_reference_offsets::<R>(trace_map, ReferenceRange::All, &mut |offset| {
        let word_index = offset / REFERENCE_BYTES;
        let bit_index = bit_start + word_index;

        reference_bits.set(bit_index);
    });
}

/// Visit heap edges encoded in the given byte window.
pub fn visit_heap_edges(
    trace_map: &TraceMap,
    start: usize,
    bytes: &[u8],
    range: ReferenceRange,
    visit: &mut dyn FnMut(HeapEdge) -> HeapResult<()>,
) -> HeapResult<()> {
    let mut walker = ByteEdgeWalker {
        start,
        bytes,
        visit,
    };

    walk_trace_map(trace_map, 0, range, &mut walker)
}

/// Visit mutable heap root slots encoded in the given byte window.
pub fn visit_heap_root_slots(
    trace_map: &TraceMap,
    start: usize,
    bytes: &mut [u8],
    range: ReferenceRange,
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> HeapResult<()> {
    let mut walker = ByteSlotWalker {
        start,
        bytes,
        visit,
    };

    walk_trace_map(trace_map, 0, range, &mut walker)
}

/// Scan read-only references from one input.
pub(crate) fn scan_references<R: ReferenceClass>(
    trace_map: &TraceMap,
    input: ReferenceInput<'_>,
    range: ReferenceRange,
    references: &mut Vec<R>,
) -> HeapResult<()> {
    visit_references::<R>(trace_map, input, range, &mut |reference| {
        references.push(reference);

        Ok(())
    })
}

/// Visit read-only references from one input.
pub(crate) fn visit_references<R: ReferenceClass>(
    trace_map: &TraceMap,
    input: ReferenceInput<'_>,
    range: ReferenceRange,
    visit: &mut dyn FnMut(R) -> HeapResult<()>,
) -> HeapResult<()> {
    match input {
        ReferenceInput::Mapped { base_address } => {
            let mut walker = MemoryReferenceVisitWalker::<R> {
                base_address,
                visit,
            };

            walk_trace_map(trace_map, 0, range, &mut walker)
        }
        ReferenceInput::Bytes { start, bytes } => {
            let mut walker = ByteReferenceVisitWalker::<R> {
                start,
                bytes,
                visit,
            };

            walk_trace_map(trace_map, 0, range, &mut walker)
        }
    }
}

/// Walk references selected by one trace map.
fn walk_trace_map<W: ReferenceWalker>(
    trace_map: &TraceMap,
    base_offset: usize,
    range: ReferenceRange,
    walker: &mut W,
) -> HeapResult<()> {
    match trace_map {
        TraceMap::Empty => {}
        TraceMap::Fixed {
            local_offsets,
            shared_offsets,
        } => {
            walker.fixed(local_offsets, shared_offsets, base_offset, range)?;
        }
        TraceMap::Nested { byte_offset, map } => {
            let byte_offset = base_offset + *byte_offset as usize;

            walk_trace_map(map, byte_offset, range, walker)?;
        }
        TraceMap::Composite { maps } => {
            for map in maps {
                walk_trace_map(map, base_offset, range, walker)?;
            }
        }
        TraceMap::Repeated {
            count,
            stride,
            element,
        } => {
            // element references lie within their stride, so prune by range
            let window = range.element_window(base_offset, *stride as usize, *count);
            for index in window {
                let element_offset = base_offset + index as usize * *stride as usize;

                walk_trace_map(element, element_offset, range, walker)?;
            }
        }
        TraceMap::Tagged {
            tag_bytes,
            variants,
        } => {
            let Some(tag) = walker.tag(base_offset, *tag_bytes)? else {
                for variant in variants {
                    let variant_offset = base_offset + variant.payload_offset as usize;

                    walk_trace_map(&variant.map, variant_offset, range, walker)?;
                }

                return Ok(());
            };
            let Some(variant) = variants.iter().find(|variant| variant.tag == tag) else {
                return Ok(());
            };
            let variant_offset = base_offset + variant.payload_offset as usize;

            walk_trace_map(&variant.map, variant_offset, range, walker)?;
        }
    }

    Ok(())
}

/// Walk direct reference offsets selected by the optional byte range.
fn walk_direct_offsets(
    offsets: &[u32],
    base_offset: usize,
    range: ReferenceRange,
    width: usize,
    mut visit: impl FnMut(usize) -> HeapResult<()>,
) -> HeapResult<()> {
    for offset in offsets {
        let offset = base_offset + *offset as usize;
        if !range.overlaps(offset, width) {
            continue;
        }

        visit(offset)?;
    }

    Ok(())
}

/// Reference walker selected by scan source and output shape.
trait ReferenceWalker {
    /// Walk one fixed trace map at the given byte offset.
    fn fixed(
        &mut self,
        local_offsets: &[u32],
        shared_offsets: &[u32],
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()>;

    /// Return the active variant tag at the given offset.
    fn tag(&mut self, offset: usize, width: u8) -> HeapResult<Option<u64>>;
}

/// Read-only reference visitor over mapped block memory.
struct MemoryReferenceVisitWalker<'a, R: ReferenceClass> {
    /// The mapped block base address.
    base_address: usize,
    /// The reference visitor.
    visit: &'a mut dyn FnMut(R) -> HeapResult<()>,
}

impl<R: ReferenceClass> ReferenceWalker for MemoryReferenceVisitWalker<'_, R> {
    fn fixed(
        &mut self,
        local_offsets: &[u32],
        shared_offsets: &[u32],
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()> {
        walk_direct_offsets(
            R::offsets(local_offsets, shared_offsets),
            base_offset,
            range,
            R::BYTE_LEN,
            |offset| {
                // SAFETY: mapped block references are aligned native words at trace-map offsets
                let bits = unsafe { read_reference_bits(self.base_address + offset) };

                (self.visit)(R::from_bits(bits))
            },
        )
    }

    fn tag(&mut self, offset: usize, width: u8) -> HeapResult<Option<u64>> {
        // SAFETY: mapped block tags are inside live payload memory
        let tag = unsafe { read_reference_tag(self.base_address + offset, width) };

        Ok(Some(tag))
    }
}

/// Read-only reference visitor over caller-provided bytes.
struct ByteReferenceVisitWalker<'a, R: ReferenceClass> {
    /// The absolute byte offset represented by `bytes`.
    start: usize,
    /// The byte window to read.
    bytes: &'a [u8],
    /// The reference visitor.
    visit: &'a mut dyn FnMut(R) -> HeapResult<()>,
}

impl<R: ReferenceClass> ReferenceWalker for ByteReferenceVisitWalker<'_, R> {
    fn fixed(
        &mut self,
        local_offsets: &[u32],
        shared_offsets: &[u32],
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()> {
        walk_direct_offsets(
            R::offsets(local_offsets, shared_offsets),
            base_offset,
            range,
            R::BYTE_LEN,
            |offset| {
                let bits = reference_bits_from_bytes(self.bytes, self.start, offset)?;

                (self.visit)(R::from_bits(bits))
            },
        )
    }

    fn tag(&mut self, offset: usize, width: u8) -> HeapResult<Option<u64>> {
        Ok(reference_tag_from_bytes(
            self.bytes, self.start, offset, width,
        ))
    }
}

/// Read-only walker over caller-provided bytes.
struct ByteEdgeWalker<'a> {
    /// The absolute byte offset represented by `bytes`.
    start: usize,
    /// The byte window to read.
    bytes: &'a [u8],
    /// The edge visitor.
    visit: &'a mut dyn FnMut(HeapEdge) -> HeapResult<()>,
}

impl ReferenceWalker for ByteEdgeWalker<'_> {
    fn fixed(
        &mut self,
        local_offsets: &[u32],
        shared_offsets: &[u32],
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()> {
        walk_direct_offsets(
            local_offsets,
            base_offset,
            range,
            HeapReference::BYTE_LEN,
            |offset| {
                let bits = reference_bits_from_bytes(self.bytes, self.start, offset)?;

                (self.visit)(HeapEdge::Local(HeapReference::from_bits(bits)))
            },
        )?;

        walk_direct_offsets(
            shared_offsets,
            base_offset,
            range,
            SharedHeapReference::BYTE_LEN,
            |offset| {
                let bits = reference_bits_from_bytes(self.bytes, self.start, offset)?;

                (self.visit)(HeapEdge::Shared(SharedHeapReference::from_bits(bits)))
            },
        )
    }

    fn tag(&mut self, offset: usize, width: u8) -> HeapResult<Option<u64>> {
        Ok(reference_tag_from_bytes(
            self.bytes, self.start, offset, width,
        ))
    }
}

/// Mutable walker over caller-provided bytes.
struct ByteSlotWalker<'a> {
    /// The absolute byte offset represented by `bytes`.
    start: usize,
    /// The byte window to mutate.
    bytes: &'a mut [u8],
    /// The slot visitor.
    visit: &'a mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
}

impl ReferenceWalker for ByteSlotWalker<'_> {
    fn fixed(
        &mut self,
        local_offsets: &[u32],
        shared_offsets: &[u32],
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()> {
        walk_direct_offsets(
            local_offsets,
            base_offset,
            range,
            HeapReference::BYTE_LEN,
            |offset| {
                let slot =
                    reference_bytes_mut(self.bytes, self.start, offset, HeapReference::BYTE_LEN)?;

                (self.visit)(RootSlot::HeapBytes(slot))
            },
        )?;

        walk_direct_offsets(
            shared_offsets,
            base_offset,
            range,
            SharedHeapReference::BYTE_LEN,
            |offset| {
                let slot = reference_bytes_mut(
                    self.bytes,
                    self.start,
                    offset,
                    SharedHeapReference::BYTE_LEN,
                )?;

                (self.visit)(RootSlot::SharedHeapBytes(slot))
            },
        )
    }

    fn tag(&mut self, offset: usize, width: u8) -> HeapResult<Option<u64>> {
        Ok(reference_tag_from_bytes(
            self.bytes, self.start, offset, width,
        ))
    }
}

/// Return one native-width reference payload from mapped memory.
unsafe fn read_reference_bits(address: usize) -> usize {
    // heap layouts guarantee pointer-width reference fields
    debug_assert_eq!(address % std::mem::align_of::<usize>(), 0);

    // SAFETY: callers provide mapped payload addresses for pointer-width fields
    unsafe { (address as *const usize).read() }
}

/// Return one unsigned reference-map tag from mapped memory.
unsafe fn read_reference_tag(address: usize, width: u8) -> u64 {
    let mut raw = [0u8; std::mem::size_of::<u64>()];

    for (index, byte) in raw.iter_mut().take(width as usize).enumerate() {
        // SAFETY: callers provide mapped payload addresses for tag bytes
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
    let window = reference_bytes(bytes, start, offset, std::mem::size_of::<usize>())?;

    // decode the native-width reference payload
    let mut raw = [0u8; std::mem::size_of::<usize>()];
    raw.copy_from_slice(window);

    Ok(usize::from_le_bytes(raw))
}

/// Return one immutable reference byte window.
fn reference_bytes(bytes: &[u8], start: usize, offset: usize, width: usize) -> HeapResult<&[u8]> {
    let Some(local_start) = offset.checked_sub(start) else {
        return Err(HeapError::representation(
            HeapRepresentationError::TruncatedReferenceBytes {
                start: offset,
                width,
            },
        ));
    };
    let local_end = local_start + width;
    let Some(window) = bytes.get(local_start..local_end) else {
        return Err(HeapError::representation(
            HeapRepresentationError::TruncatedReferenceBytes {
                start: local_start,
                width,
            },
        ));
    };

    Ok(window)
}

/// Return one mutable reference byte window.
fn reference_bytes_mut(
    bytes: &mut [u8],
    start: usize,
    offset: usize,
    width: usize,
) -> HeapResult<&mut [u8]> {
    let Some(local_start) = offset.checked_sub(start) else {
        return Err(HeapError::representation(
            HeapRepresentationError::TruncatedReferenceBytes {
                start: offset,
                width,
            },
        ));
    };
    let local_end = local_start + width;
    let Some(window) = bytes.get_mut(local_start..local_end) else {
        return Err(HeapError::representation(
            HeapRepresentationError::TruncatedReferenceBytes {
                start: local_start,
                width,
            },
        ));
    };

    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reject partial byte windows that overlap one encoded shared reference.
    #[test]
    fn test_rejects_truncated_shared_reference_window() {
        let trace_map = TraceMap::Fixed {
            local_offsets: Vec::new().into_boxed_slice(),
            shared_offsets: vec![0].into_boxed_slice(),
        };
        let mut references = Vec::new();

        let error = scan_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::bytes(4, &[0]),
            ReferenceRange::bytes(4, 1),
            &mut references,
        )
        .expect_err("partial reference windows should fail loudly");

        assert_eq!(
            error,
            HeapError::representation(HeapRepresentationError::TruncatedReferenceBytes {
                start: 0,
                width: SharedHeapReference::BYTE_LEN,
            })
        );
    }
}
