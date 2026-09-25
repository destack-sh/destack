use tspp_mir::{TraceId, TraceMap};

use crate::{
    Bitmap, HeapError, HeapReference, HeapRepresentationError, HeapResult, ReferenceOffsets,
    RootSlot, SharedHeapReference, TraceView, TraceVisitor,
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
    pub(crate) fn overlaps(self, offset: usize, width: usize) -> bool {
        match self {
            Self::All => true,
            Self::Bytes { start, end } => ranges_overlap(start, end, offset, width),
        }
    }

    /// Return the repeated element indexes whose strides overlap this range.
    pub(crate) fn element_window(
        self,
        base_offset: usize,
        stride: usize,
        count: u32,
    ) -> std::ops::Range<u32> {
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
    fn offsets(offsets: ReferenceOffsets<'_>) -> &[u32];

    /// Decode one reference from native bits.
    fn from_bits(bits: usize) -> Self;

    /// Return whether one trace map may contain this reference class.
    fn is_present(trace_map: &TraceMap) -> bool;

    /// Return whether one byte range may overlap this reference class.
    fn overlaps(trace_map: &TraceMap, range: ReferenceRange) -> bool;
}

/// Worker-local reference fields.
impl ReferenceClass for HeapReference {
    const BYTE_LEN: usize = HeapReference::BYTE_LEN;

    fn offsets(offsets: ReferenceOffsets<'_>) -> &[u32] {
        offsets.local
    }

    fn from_bits(bits: usize) -> Self {
        HeapReference::from_bits(bits)
    }

    fn is_present(trace_map: &TraceMap) -> bool {
        trace_map.has_local_reference()
    }

    fn overlaps(trace_map: &TraceMap, range: ReferenceRange) -> bool {
        overlaps_reference_range::<Self>(trace_map, range)
    }
}

/// Runtime-shared reference fields.
impl ReferenceClass for SharedHeapReference {
    const BYTE_LEN: usize = SharedHeapReference::BYTE_LEN;

    fn offsets(offsets: ReferenceOffsets<'_>) -> &[u32] {
        offsets.shared
    }

    fn from_bits(bits: usize) -> Self {
        SharedHeapReference::from_bits(bits)
    }

    fn is_present(trace_map: &TraceMap) -> bool {
        trace_map.has_shared_reference()
    }

    fn overlaps(trace_map: &TraceMap, range: ReferenceRange) -> bool {
        overlaps_reference_range::<Self>(trace_map, range)
    }
}

/// The class of one address word a frame value holds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameWord {
    /// An address inside a frame.
    Frame,
    /// A borrow whose target storage is classified by address.
    Borrow,
}

/// Heap reference edge encoded in object or frame bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeapEdge {
    /// Worker-local heap reference.
    Local(HeapReference),
    /// Runtime-shared heap reference.
    Shared(SharedHeapReference),
}

impl HeapEdge {
    /// Return the stable space-relative reference bits.
    pub const fn bits(self) -> usize {
        match self {
            Self::Local(reference) => reference.bits(),
            Self::Shared(reference) => reference.bits(),
        }
    }
}

/// Visit exact reference offsets for one payload-independent trace map.
pub(crate) fn visit_static_reference_offsets<R: ReferenceClass>(
    trace_map: &TraceMap,
    range: ReferenceRange,
    visit: &mut impl FnMut(usize),
) {
    debug_assert!(!trace_map.has_variant_reference());

    walk_reference_offset_union::<R>(trace_map, 0, range, &mut |offset| {
        visit(offset);

        true
    });
}

/// Walk the union of reference offsets across variants.
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
            frame_offsets,
            borrow_offsets,
        } => {
            let offsets = ReferenceOffsets {
                local: local_offsets,
                shared: shared_offsets,
                frame: frame_offsets,
                borrow: borrow_offsets,
            };
            for offset in R::offsets(offsets) {
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
        TraceMap::Variant { cases, .. } => {
            for case in cases {
                let variant_offset = base_offset + case.payload_offset as usize;

                if !walk_reference_offset_union::<R>(&case.map, variant_offset, range, visit) {
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
        frame_offsets: Vec::new().into_boxed_slice(),
        borrow_offsets: Vec::new().into_boxed_slice(),
    }
}

/// Return whether one byte range may overlap any reference field of one class.
fn overlaps_reference_range<R: ReferenceClass>(
    trace_map: &TraceMap,
    range: ReferenceRange,
) -> bool {
    let mut overlaps = false;

    walk_reference_offset_union::<R>(trace_map, 0, range, &mut |_| {
        overlaps = true;

        false
    });

    overlaps
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
    debug_assert!(!trace_map.has_variant_reference());

    // noscan layouts have no side bits
    if !trace_map.has_heap_reference() {
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
    debug_assert!(!trace_map.has_variant_reference());

    visit_static_reference_offsets::<R>(trace_map, ReferenceRange::All, &mut |offset| {
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

impl TraceView<'_> {
    /// Visit mutable frame address slots encoded in one value, borrows classified by the caller.
    pub fn visit_frame_address_slots(
        self,
        trace: TraceId,
        bytes: &mut [u8],
        visit: &mut dyn FnMut(&mut [u8], FrameWord) -> HeapResult<()>,
    ) -> HeapResult<()> {
        let mut walker = FrameAddressSlotWalker { bytes, visit };

        self.walk(trace, 0, ReferenceRange::All, &mut walker)
    }
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

/// Visit read-only references from one compact trace entry.
pub(crate) fn visit_trace_references<R: ReferenceClass>(
    trace_view: TraceView<'_>,
    trace_id: TraceId,
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

            trace_view.walk(trace_id, 0, range, &mut walker)
        }
        ReferenceInput::Bytes { start, bytes } => {
            let mut walker = ByteReferenceVisitWalker::<R> {
                start,
                bytes,
                visit,
            };

            trace_view.walk(trace_id, 0, range, &mut walker)
        }
    }
}

/// Walk references selected by one trace map.
fn walk_trace_map<W: TraceVisitor>(
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
            frame_offsets,
            borrow_offsets,
        } => {
            let offsets = ReferenceOffsets {
                local: local_offsets,
                shared: shared_offsets,
                frame: frame_offsets,
                borrow: borrow_offsets,
            };
            walker.fixed(offsets, base_offset, range)?;
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
        TraceMap::Variant { encoding, cases } => {
            let field = encoding.field();
            let offset = base_offset + field.offset as usize;
            let Some(scalar) = walker.scalar(offset, field.byte_len)? else {
                for case in cases {
                    let variant_offset = base_offset + case.payload_offset as usize;

                    walk_trace_map(&case.map, variant_offset, range, walker)?;
                }

                return Ok(());
            };
            let Some(variant) = trace_map.variant(scalar) else {
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

/// Read-only reference visitor over mapped block memory.
struct MemoryReferenceVisitWalker<'a, R: ReferenceClass> {
    /// The mapped block base address.
    base_address: usize,
    /// The reference visitor.
    visit: &'a mut dyn FnMut(R) -> HeapResult<()>,
}

impl<R: ReferenceClass> TraceVisitor for MemoryReferenceVisitWalker<'_, R> {
    fn fixed(
        &mut self,
        offsets: ReferenceOffsets<'_>,
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()> {
        walk_direct_offsets(
            R::offsets(offsets),
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

    fn scalar(&mut self, offset: usize, byte_len: u8) -> HeapResult<Option<u128>> {
        // SAFETY: mapped discriminants are inside live payload memory
        let scalar = unsafe { read_discriminant_scalar(self.base_address + offset, byte_len) };

        Ok(Some(scalar))
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

impl<R: ReferenceClass> TraceVisitor for ByteReferenceVisitWalker<'_, R> {
    fn fixed(
        &mut self,
        offsets: ReferenceOffsets<'_>,
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()> {
        walk_direct_offsets(
            R::offsets(offsets),
            base_offset,
            range,
            R::BYTE_LEN,
            |offset| {
                let bits = reference_bits_from_bytes(self.bytes, self.start, offset)?;

                (self.visit)(R::from_bits(bits))
            },
        )
    }

    fn scalar(&mut self, offset: usize, byte_len: u8) -> HeapResult<Option<u128>> {
        Ok(discriminant_scalar_from_bytes(
            self.bytes, self.start, offset, byte_len,
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

impl TraceVisitor for ByteEdgeWalker<'_> {
    fn fixed(
        &mut self,
        offsets: ReferenceOffsets<'_>,
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()> {
        walk_direct_offsets(
            offsets.local,
            base_offset,
            range,
            HeapReference::BYTE_LEN,
            |offset| {
                let bits = reference_bits_from_bytes(self.bytes, self.start, offset)?;

                (self.visit)(HeapEdge::Local(HeapReference::from_bits(bits)))
            },
        )?;

        walk_direct_offsets(
            offsets.shared,
            base_offset,
            range,
            SharedHeapReference::BYTE_LEN,
            |offset| {
                let bits = reference_bits_from_bytes(self.bytes, self.start, offset)?;

                (self.visit)(HeapEdge::Shared(SharedHeapReference::from_bits(bits)))
            },
        )
    }

    fn scalar(&mut self, offset: usize, byte_len: u8) -> HeapResult<Option<u128>> {
        Ok(discriminant_scalar_from_bytes(
            self.bytes, self.start, offset, byte_len,
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

/// Mutable frame address walker over caller-provided bytes.
struct FrameAddressSlotWalker<'a> {
    /// The byte window to mutate.
    bytes: &'a mut [u8],
    /// The frame address slot visitor.
    visit: &'a mut dyn FnMut(&mut [u8], FrameWord) -> HeapResult<()>,
}

impl TraceVisitor for FrameAddressSlotWalker<'_> {
    fn fixed(
        &mut self,
        offsets: ReferenceOffsets<'_>,
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()> {
        walk_direct_offsets(
            offsets.frame,
            base_offset,
            range,
            REFERENCE_BYTES,
            |offset| {
                let slot = reference_bytes_mut(self.bytes, 0, offset, REFERENCE_BYTES)?;

                (self.visit)(slot, FrameWord::Frame)
            },
        )?;

        walk_direct_offsets(
            offsets.borrow,
            base_offset,
            range,
            REFERENCE_BYTES,
            |offset| {
                let slot = reference_bytes_mut(self.bytes, 0, offset, REFERENCE_BYTES)?;

                (self.visit)(slot, FrameWord::Borrow)
            },
        )
    }

    fn scalar(&mut self, offset: usize, byte_len: u8) -> HeapResult<Option<u128>> {
        Ok(discriminant_scalar_from_bytes(
            self.bytes, 0, offset, byte_len,
        ))
    }
}

impl TraceVisitor for ByteSlotWalker<'_> {
    fn fixed(
        &mut self,
        offsets: ReferenceOffsets<'_>,
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()> {
        walk_direct_offsets(
            offsets.local,
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
            offsets.shared,
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
        )?;

        walk_direct_offsets(
            offsets.borrow,
            base_offset,
            range,
            REFERENCE_BYTES,
            |offset| {
                let slot = reference_bytes_mut(self.bytes, self.start, offset, REFERENCE_BYTES)?;

                (self.visit)(RootSlot::BorrowBytes(slot))
            },
        )
    }

    fn scalar(&mut self, offset: usize, byte_len: u8) -> HeapResult<Option<u128>> {
        Ok(discriminant_scalar_from_bytes(
            self.bytes, self.start, offset, byte_len,
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

/// Return one variant discriminant scalar from mapped memory.
unsafe fn read_discriminant_scalar(address: usize, byte_len: u8) -> u128 {
    let mut raw = [0u8; std::mem::size_of::<u128>()];

    for (index, byte) in raw.iter_mut().take(byte_len as usize).enumerate() {
        // SAFETY: callers provide mapped payload addresses for discriminant bytes
        *byte = unsafe { (address as *const u8).add(index).read() };
    }

    u128::from_le_bytes(raw)
}

/// Return one variant discriminant scalar from caller-provided bytes.
fn discriminant_scalar_from_bytes(
    bytes: &[u8],
    start: usize,
    offset: usize,
    byte_len: u8,
) -> Option<u128> {
    if offset < start {
        return None;
    }

    let local_start = offset - start;
    let local_end = local_start + byte_len as usize;
    let window = bytes.get(local_start..local_end)?;

    let mut raw = [0u8; std::mem::size_of::<u128>()];
    raw[..byte_len as usize].copy_from_slice(window);

    Some(u128::from_le_bytes(raw))
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
    use tspp_mir::{DiscriminantField, VariantEncoding, VariantTrace};

    use super::*;

    /// Select variant references through a direct discriminant field.
    #[test]
    fn test_scans_variant_reference_at_nonzero_discriminant_offset() {
        let trace_map = TraceMap::Variant {
            encoding: VariantEncoding::Direct {
                field: DiscriminantField::scalar(3, 1),
            },
            cases: vec![VariantTrace {
                discriminant: 7u128.into(),
                payload_offset: 8,
                map: TraceMap::Fixed {
                    local_offsets: vec![0].into_boxed_slice(),
                    shared_offsets: Vec::new().into_boxed_slice(),
                    frame_offsets: Vec::new().into_boxed_slice(),
                    borrow_offsets: Vec::new().into_boxed_slice(),
                },
            }]
            .into_boxed_slice(),
        };
        let expected = HeapReference::new(42);
        let mut bytes = vec![0; 8 + HeapReference::BYTE_LEN];
        bytes[3] = 7;
        expected
            .write_to_bytes(&mut bytes[8..])
            .expect("reference bytes should have the native width");

        let mut references = Vec::new();
        scan_references::<HeapReference>(
            &trace_map,
            ReferenceInput::bytes(0, &bytes),
            ReferenceRange::All,
            &mut references,
        )
        .expect("variant reference bytes should scan");

        assert_eq!(references, vec![expected]);
    }

    /// Select variant references through a payload niche.
    #[test]
    fn test_scans_niche_variant_reference() {
        let trace_map = TraceMap::Variant {
            encoding: VariantEncoding::Niche {
                field: DiscriminantField::scalar(0, 1),
                untagged_case: 0,
                niche_start: 0u128.into(),
            },
            cases: vec![
                VariantTrace {
                    discriminant: 1u128.into(),
                    payload_offset: 0,
                    map: TraceMap::Empty,
                },
                VariantTrace {
                    discriminant: 0u128.into(),
                    payload_offset: 8,
                    map: TraceMap::Fixed {
                        local_offsets: vec![0].into_boxed_slice(),
                        shared_offsets: Box::default(),
                        frame_offsets: Box::default(),
                        borrow_offsets: Box::default(),
                    },
                },
            ]
            .into_boxed_slice(),
        };
        let expected = HeapReference::new(42);
        let mut bytes = vec![1; 8 + HeapReference::BYTE_LEN];
        bytes[0] = 0;
        expected
            .write_to_bytes(&mut bytes[8..])
            .expect("reference bytes should have the native width");

        let mut references = Vec::new();
        scan_references::<HeapReference>(
            &trace_map,
            ReferenceInput::bytes(0, &bytes),
            ReferenceRange::All,
            &mut references,
        )
        .expect("niche trace should scan");

        assert_eq!(references, vec![expected]);
    }

    /// Reject partial byte windows that overlap one encoded shared reference.
    #[test]
    fn test_rejects_truncated_shared_reference_window() {
        let trace_map = TraceMap::Fixed {
            local_offsets: Vec::new().into_boxed_slice(),
            shared_offsets: vec![0].into_boxed_slice(),
            frame_offsets: Vec::new().into_boxed_slice(),
            borrow_offsets: Vec::new().into_boxed_slice(),
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
