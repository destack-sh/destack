use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use super::super::value::{ManagedReference, Value};
use crate::alloc::{projected_vec_capacity, vec_capacity_bytes, vec_capacity_bytes_delta};

/// Reference-scanning metadata for one managed allocation payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceMap {
    /// Allocation contains no managed references.
    None,
    /// Allocation stores direct managed-reference words at fixed byte offsets.
    ReferenceOffsets {
        /// Byte offsets of encoded managed references.
        offsets: Vec<u32>,
    },
    /// Allocation stores full VM values at fixed byte offsets.
    ValueOffsets {
        /// Byte offsets of encoded VM values.
        offsets: Vec<u32>,
    },
    /// Allocation stores repeated elements with managed-reference words at fixed element offsets.
    RepeatedReferenceOffsets {
        /// The number of elements in the allocation.
        count: u32,
        /// The element byte stride.
        element_size: u32,
        /// Managed-reference byte offsets within each element.
        offsets: Vec<u32>,
    },
}

impl ReferenceMap {
    /// Return an empty reference map.
    pub const fn empty() -> Self {
        Self::None
    }

    /// Report whether this reference map can reach managed references.
    pub fn has_managed_edges(&self) -> bool {
        match self {
            Self::None => false,
            Self::ReferenceOffsets { offsets } => !offsets.is_empty(),
            Self::ValueOffsets { offsets } => !offsets.is_empty(),
            Self::RepeatedReferenceOffsets { count, offsets, .. } => {
                *count > 0 && !offsets.is_empty()
            }
        }
    }

    /// Report whether one write range may overlap any managed-reference bytes.
    pub fn touches_managed_range(
        &self,
        start: usize,
        len: usize,
        managed_reference_bytes: u8,
    ) -> bool {
        if len == 0 {
            return false;
        }

        let Some(end) = start.checked_add(len) else {
            return true;
        };
        let managed_reference_bytes = managed_reference_width(managed_reference_bytes);

        match self {
            Self::None => false,
            Self::ReferenceOffsets { offsets } => offsets
                .iter()
                .copied()
                .any(|offset| ranges_overlap(start, end, offset as usize, managed_reference_bytes)),
            Self::ValueOffsets { offsets } => offsets
                .iter()
                .copied()
                .any(|offset| ranges_overlap(start, end, offset as usize, Value::BYTE_LEN)),
            Self::RepeatedReferenceOffsets {
                count,
                element_size,
                offsets,
            } => offsets.iter().copied().any(|offset| {
                repeated_ranges_overlap(
                    start,
                    end,
                    *count as usize,
                    *element_size as usize,
                    offset as usize,
                    managed_reference_bytes,
                )
            }),
        }
    }

    /// Return the retained bytes owned by this reference map.
    pub(crate) fn retained_bytes(&self) -> usize {
        match self {
            Self::None => 0,
            Self::ReferenceOffsets { offsets } => offsets.capacity() * std::mem::size_of::<u32>(),
            Self::ValueOffsets { offsets } => offsets.capacity() * std::mem::size_of::<u32>(),
            Self::RepeatedReferenceOffsets { offsets, .. } => {
                offsets.capacity() * std::mem::size_of::<u32>()
            }
        }
    }

    /// Visit each managed reference stored in the given payload bytes.
    pub fn for_each_reference(
        &self,
        bytes: &[u8],
        managed_reference_bytes: u8,
        mut visit: impl FnMut(ManagedReference),
    ) {
        let managed_reference_bytes = managed_reference_width(managed_reference_bytes);

        match self {
            Self::None => {}
            Self::ReferenceOffsets { offsets } => {
                for &offset in offsets {
                    let start = offset as usize;
                    let reference = decode_managed_reference(bytes, start, managed_reference_bytes);

                    visit(reference);
                }
            }
            Self::ValueOffsets { offsets } => {
                for &offset in offsets {
                    let start = offset as usize;
                    let end = start.saturating_add(Value::BYTE_LEN);
                    let window = bytes.get(start..end).unwrap_or_else(|| {
                        panic!(
                            "truncated value payload while tracing managed references: start={start}, width={}",
                            Value::BYTE_LEN,
                        )
                    });

                    let value = Value::from_byte_slice(window).unwrap_or_else(|| {
                        panic!(
                            "invalid value payload while tracing managed references: start={start}"
                        )
                    });

                    if let Some(handle) = value.as_managed_reference() {
                        visit(handle);
                    }
                }
            }
            Self::RepeatedReferenceOffsets {
                count,
                element_size,
                offsets,
            } => {
                let element_size = *element_size as usize;

                for index in 0..(*count as usize) {
                    let base = index.saturating_mul(element_size);

                    for &offset in offsets {
                        let start = base.saturating_add(offset as usize);
                        let reference =
                            decode_managed_reference(bytes, start, managed_reference_bytes);

                        visit(reference);
                    }
                }
            }
        }
    }

    /// Visit each managed reference using one random-access byte reader.
    pub fn for_each_reference_in_reader(
        &self,
        managed_reference_bytes: u8,
        mut read_window: impl FnMut(usize, &mut [u8]) -> bool,
        mut visit: impl FnMut(ManagedReference),
    ) {
        let managed_reference_bytes = managed_reference_width(managed_reference_bytes);

        match self {
            Self::None => {}
            Self::ReferenceOffsets { offsets } => {
                let mut raw = [0u8; 8];

                for &offset in offsets {
                    let start = offset as usize;
                    let window = &mut raw[..managed_reference_bytes];

                    if !read_window(start, window) {
                        panic!(
                            "truncated managed reference payload while tracing: start={start}, width={managed_reference_bytes}",
                        );
                    }

                    let reference = decode_managed_reference_from_window(window);
                    visit(reference);
                }
            }
            Self::ValueOffsets { offsets } => {
                let mut raw = [0u8; Value::BYTE_LEN];

                for &offset in offsets {
                    let start = offset as usize;
                    let window = &mut raw[..Value::BYTE_LEN];

                    if !read_window(start, window) {
                        panic!(
                            "truncated value payload while tracing managed references: start={start}, width={}",
                            Value::BYTE_LEN,
                        );
                    }

                    let value = Value::from_byte_slice(window).unwrap_or_else(|| {
                        panic!(
                            "invalid value payload while tracing managed references: start={start}"
                        )
                    });

                    if let Some(handle) = value.as_managed_reference() {
                        visit(handle);
                    }
                }
            }
            Self::RepeatedReferenceOffsets {
                count,
                element_size,
                offsets,
            } => {
                let element_size = *element_size as usize;
                let mut raw = [0u8; 8];

                for index in 0..(*count as usize) {
                    let base = index.saturating_mul(element_size);

                    for &offset in offsets {
                        let start = base.saturating_add(offset as usize);
                        let window = &mut raw[..managed_reference_bytes];

                        if !read_window(start, window) {
                            panic!(
                                "truncated managed reference payload while tracing: start={start}, width={managed_reference_bytes}",
                            );
                        }

                        let reference = decode_managed_reference_from_window(window);
                        visit(reference);
                    }
                }
            }
        }
    }

    /// Visit each managed reference whose encoded bytes overlap the given byte range.
    pub fn for_each_reference_in_reader_range(
        &self,
        start: usize,
        len: usize,
        managed_reference_bytes: u8,
        mut read_window: impl FnMut(usize, &mut [u8]) -> bool,
        mut visit: impl FnMut(ManagedReference),
    ) {
        if len == 0 {
            return;
        }

        let Some(end) = start.checked_add(len) else {
            return;
        };
        let managed_reference_bytes = managed_reference_width(managed_reference_bytes);

        match self {
            Self::None => {}
            Self::ReferenceOffsets { offsets } => {
                let mut raw = [0u8; 8];

                for &offset in offsets {
                    let field_start = offset as usize;
                    if !ranges_overlap(start, end, field_start, managed_reference_bytes) {
                        continue;
                    }

                    let window = &mut raw[..managed_reference_bytes];
                    if !read_window(field_start, window) {
                        panic!(
                            "truncated managed reference payload while tracing: start={field_start}, width={managed_reference_bytes}",
                        );
                    }

                    let reference = decode_managed_reference_from_window(window);
                    visit(reference);
                }
            }
            Self::ValueOffsets { offsets } => {
                let mut raw = [0u8; Value::BYTE_LEN];

                for &offset in offsets {
                    let field_start = offset as usize;
                    if !ranges_overlap(start, end, field_start, Value::BYTE_LEN) {
                        continue;
                    }

                    let window = &mut raw[..Value::BYTE_LEN];
                    if !read_window(field_start, window) {
                        panic!(
                            "truncated value payload while tracing managed references: start={field_start}, width={}",
                            Value::BYTE_LEN,
                        );
                    }

                    let value = Value::from_byte_slice(window).unwrap_or_else(|| {
                        panic!(
                            "invalid value payload while tracing managed references: start={field_start}"
                        )
                    });

                    if let Some(handle) = value.as_managed_reference() {
                        visit(handle);
                    }
                }
            }
            Self::RepeatedReferenceOffsets {
                count,
                element_size,
                offsets,
            } => {
                let element_size = *element_size as usize;
                let mut raw = [0u8; 8];

                for &offset in offsets {
                    let Some((first_index, last_index)) = overlapping_repeated_index_range(
                        start,
                        end,
                        *count as usize,
                        element_size,
                        offset as usize,
                        managed_reference_bytes,
                    ) else {
                        continue;
                    };

                    for index in first_index..=last_index {
                        let field_start = index
                            .saturating_mul(element_size)
                            .saturating_add(offset as usize);
                        let window = &mut raw[..managed_reference_bytes];

                        if !read_window(field_start, window) {
                            panic!(
                                "truncated managed reference payload while tracing: start={field_start}, width={managed_reference_bytes}",
                            );
                        }

                        let reference = decode_managed_reference_from_window(window);
                        visit(reference);
                    }
                }
            }
        }
    }
}

/// Return the validated byte width for one traced managed reference.
fn managed_reference_width(managed_reference_bytes: u8) -> usize {
    match managed_reference_bytes {
        4 | 8 => managed_reference_bytes as usize,
        _ => panic!("unsupported managed reference width for tracing: {managed_reference_bytes}"),
    }
}

/// Decode one managed reference from one traced byte window.
fn decode_managed_reference(
    bytes: &[u8],
    start: usize,
    managed_reference_bytes: usize,
) -> ManagedReference {
    match managed_reference_bytes {
        4 => {
            let end = start.checked_add(4).unwrap_or_else(|| {
                panic!("managed reference offset overflow while tracing: start={start}, width=4")
            });
            let window = bytes.get(start..end).unwrap_or_else(|| {
                panic!(
                    "truncated managed reference payload while tracing: start={start}, width=4, len={}",
                    bytes.len(),
                )
            });
            let mut raw = [0u8; 4];
            raw.copy_from_slice(window);

            ManagedReference::from_bits(u64::from(u32::from_le_bytes(raw)))
        }
        8 => {
            let end = start.checked_add(8).unwrap_or_else(|| {
                panic!("managed reference offset overflow while tracing: start={start}, width=8")
            });
            let window = bytes.get(start..end).unwrap_or_else(|| {
                panic!(
                    "truncated managed reference payload while tracing: start={start}, width=8, len={}",
                    bytes.len(),
                )
            });
            let mut raw = [0u8; 8];
            raw.copy_from_slice(window);

            ManagedReference::from_bits(u64::from_le_bytes(raw))
        }
        _ => panic!("unsupported managed reference width for tracing: {managed_reference_bytes}"),
    }
}

/// Decode one managed reference from one traced byte window.
fn decode_managed_reference_from_window(window: &[u8]) -> ManagedReference {
    match window.len() {
        4 => {
            let mut raw = [0u8; 4];
            raw.copy_from_slice(window);

            ManagedReference::from_bits(u64::from(u32::from_le_bytes(raw)))
        }
        8 => {
            let mut raw = [0u8; 8];
            raw.copy_from_slice(window);

            ManagedReference::from_bits(u64::from_le_bytes(raw))
        }
        _ => panic!(
            "unsupported managed reference width for tracing window: {}",
            window.len()
        ),
    }
}

/// Report whether one byte range overlaps one fixed-width field range.
fn ranges_overlap(
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

/// Report whether one repeated field range can overlap the given byte window.
fn repeated_ranges_overlap(
    start: usize,
    end: usize,
    count: usize,
    element_size: usize,
    offset: usize,
    width: usize,
) -> bool {
    overlapping_repeated_index_range(start, end, count, element_size, offset, width).is_some()
}

/// Return the overlapping repeated-element index range for the given byte window.
fn overlapping_repeated_index_range(
    start: usize,
    end: usize,
    count: usize,
    element_size: usize,
    offset: usize,
    width: usize,
) -> Option<(usize, usize)> {
    if count == 0 || width == 0 {
        return None;
    }

    if element_size == 0 {
        return ranges_overlap(start, end, offset, width).then_some((0, count - 1));
    }

    let start = start as i128;
    let end = end as i128;
    let count = count as i128;
    let element_size = element_size as i128;
    let offset = offset as i128;
    let width = width as i128;
    let low_numerator = start - offset - width + 1;
    let high_numerator = end - offset - 1;
    let low = div_ceil_i128(low_numerator, element_size).max(0);
    let high = div_floor_i128(high_numerator, element_size).min(count - 1);

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

/// One interned managed reference-map identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReferenceMapId(u32);

impl ReferenceMapId {
    /// Create one managed reference-map identifier.
    pub(crate) const fn new(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the reference-map table index for this identifier.
    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Interned managed reference maps for one heap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReferenceMapTable {
    /// Interned reference maps.
    maps: Vec<ReferenceMap>,
    /// Open-addressed reverse lookup keyed by reference-map id.
    map_slots: Vec<u32>,
    /// The exact retained bytes for this table.
    retained_bytes: usize,
}

/// The sentinel for one empty reverse-lookup slot.
const EMPTY_REFERENCE_MAP_SLOT: u32 = u32::MAX;

impl Default for ReferenceMapTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferenceMapTable {
    /// Create one managed reference-map table with the empty map pre-interned.
    pub(crate) fn new() -> Self {
        Self::from_maps(vec![ReferenceMap::empty()])
    }

    /// Restore one managed reference-map table from one flattened map list.
    pub(crate) fn from_maps(maps: Vec<ReferenceMap>) -> Self {
        let maps = if maps.is_empty() {
            vec![ReferenceMap::empty()]
        } else {
            maps
        };
        let mut table = Self {
            maps,
            map_slots: Vec::new(),
            retained_bytes: 0,
        };
        table.rebuild_map_slots();
        table.recompute_retained_bytes();
        table
    }

    /// Return one borrowed reference map for the given identifier.
    pub(crate) fn get(&self, map_id: ReferenceMapId) -> Option<&ReferenceMap> {
        self.maps.get(map_id.index())
    }

    /// Return the stable identifier this map would have after interning.
    pub(crate) fn projected_id(&self, map: &ReferenceMap) -> ReferenceMapId {
        if self.map_slots.is_empty() {
            return ReferenceMapId::new(self.maps.len());
        }

        match self.lookup(map) {
            Ok(map_id) => map_id,
            Err(_) => ReferenceMapId::new(self.maps.len()),
        }
    }

    /// Intern one reference map and return its stable identifier.
    pub(crate) fn intern(&mut self, map: ReferenceMap) -> (ReferenceMapId, i64) {
        if self.map_slots.is_empty() {
            self.rebuild_map_slots();
        }

        if let Ok(map_id) = self.lookup(&map) {
            return (map_id, 0);
        }

        let retained_before = self.retained_bytes as i64;

        if self.needs_rehash() {
            self.rebuild_map_slots_for_count(self.maps.len().saturating_add(1));
        }

        let map_id = ReferenceMapId::new(self.maps.len());
        self.maps.push(map);
        let Err(slot_index) = self.lookup(
            self.maps
                .last()
                .expect("newly interned reference map must stay addressable"),
        ) else {
            unreachable!("newly interned reference map must not already exist");
        };
        self.map_slots[slot_index] = map_id.index() as u32;

        self.recompute_retained_bytes();
        (map_id, self.retained_bytes as i64 - retained_before)
    }

    /// Return the retained-byte delta for interning one reference map.
    pub(crate) fn intern_delta(&self, map: &ReferenceMap) -> i64 {
        if self.map_slots.is_empty() {
            if self.maps.iter().any(|existing| existing == map) {
                return self.rebuild_retained_delta();
            }

            return self
                .rebuild_retained_delta()
                .saturating_add(self.intern_delta_after_rebuild(map));
        }

        self.intern_delta_after_rebuild(map)
    }

    /// Intern one borrowed reference map and return its stable identifier.
    pub(crate) fn intern_borrowed(&mut self, map: &ReferenceMap) -> (ReferenceMapId, i64) {
        if self.map_slots.is_empty() {
            self.rebuild_map_slots();
        }

        if let Ok(map_id) = self.lookup(map) {
            return (map_id, 0);
        }

        self.intern(map.clone())
    }

    /// Return the retained-byte delta for interning one borrowed reference map.
    pub(crate) fn intern_borrowed_delta(&self, map: &ReferenceMap) -> i64 {
        self.intern_delta(map)
    }

    /// Intern one borrowed repeated-offset map and return its stable identifier.
    pub(crate) fn intern_repeated_reference_offsets(
        &mut self,
        count: u32,
        element_size: u32,
        offsets: &[u32],
    ) -> (ReferenceMapId, i64) {
        if self.map_slots.is_empty() {
            self.rebuild_map_slots();
        }

        if let Ok(map_id) = self.lookup_repeated_reference_offsets(count, element_size, offsets) {
            return (map_id, 0);
        }

        self.intern(ReferenceMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets: offsets.to_vec(),
        })
    }

    /// Return the retained-byte delta for interning one borrowed repeated-offset map.
    pub(crate) fn intern_repeated_reference_offsets_delta(
        &self,
        count: u32,
        element_size: u32,
        offsets: &[u32],
    ) -> i64 {
        if self.map_slots.is_empty() {
            let already_interned = self.maps.iter().any(|map| {
                matches!(
                    map,
                    ReferenceMap::RepeatedReferenceOffsets {
                        count: existing_count,
                        element_size: existing_element_size,
                        offsets: existing_offsets,
                    } if *existing_count == count
                        && *existing_element_size == element_size
                        && existing_offsets.as_slice() == offsets
                )
            });

            if already_interned {
                return self.rebuild_retained_delta();
            }

            return self.rebuild_retained_delta().saturating_add(
                self.intern_repeated_delta_after_rebuild(count, element_size, offsets),
            );
        }

        self.intern_repeated_delta_after_rebuild(count, element_size, offsets)
    }

    /// Return the flattened reference maps.
    pub(crate) fn snapshot(&self) -> Vec<ReferenceMap> {
        self.maps.clone()
    }

    /// Return the retained bytes owned by this table.
    pub(crate) fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    /// Return whether the reverse lookup should grow before one more insert.
    fn needs_rehash(&self) -> bool {
        self.map_slots.is_empty()
            || self.maps.len().saturating_add(1) * 4 > self.map_slots.len() * 3
    }

    /// Rebuild the reverse lookup for the current trace set.
    fn rebuild_map_slots(&mut self) {
        self.rebuild_map_slots_for_count(self.maps.len());
    }

    /// Rebuild the reverse lookup for the given target trace count.
    fn rebuild_map_slots_for_count(&mut self, count: usize) {
        let slot_count = count.saturating_mul(2).max(8).next_power_of_two();
        self.map_slots = vec![EMPTY_REFERENCE_MAP_SLOT; slot_count];

        for (index, map) in self.maps.iter().enumerate() {
            let Err(slot_index) = self.lookup_in_slots(map, &self.map_slots) else {
                unreachable!("reference-map table rebuild must not duplicate entries");
            };
            self.map_slots[slot_index] = index as u32;
        }
    }

    /// Return one interned identifier or one insertion slot for this reference map.
    fn lookup(&self, map: &ReferenceMap) -> Result<ReferenceMapId, usize> {
        self.lookup_in_slots(map, &self.map_slots)
            .map(|map_id| ReferenceMapId::new(map_id as usize))
    }

    /// Return one interned identifier for a borrowed repeated-offset map when present.
    fn lookup_repeated_reference_offsets(
        &self,
        count: u32,
        element_size: u32,
        offsets: &[u32],
    ) -> Result<ReferenceMapId, usize> {
        self.lookup_repeated_reference_offsets_in_slots(
            count,
            element_size,
            offsets,
            &self.map_slots,
        )
        .map(|map_id| ReferenceMapId::new(map_id as usize))
    }

    /// Probe one reverse-lookup table for this reference map.
    fn lookup_in_slots(&self, map: &ReferenceMap, slots: &[u32]) -> Result<u32, usize> {
        let mask = slots.len() - 1;
        let mut slot_index = (Self::hash_reference_map(map) as usize) & mask;

        loop {
            let map_index = slots[slot_index];

            if map_index == EMPTY_REFERENCE_MAP_SLOT {
                return Err(slot_index);
            }

            if self.maps[map_index as usize] == *map {
                return Ok(map_index);
            }

            slot_index = (slot_index + 1) & mask;
        }
    }

    /// Probe one reverse-lookup table for one borrowed repeated-offset map.
    fn lookup_repeated_reference_offsets_in_slots(
        &self,
        count: u32,
        element_size: u32,
        offsets: &[u32],
        slots: &[u32],
    ) -> Result<u32, usize> {
        let mask = slots.len() - 1;
        let mut slot_index =
            (Self::hash_repeated_reference_offsets(count, element_size, offsets) as usize) & mask;

        loop {
            let map_index = slots[slot_index];

            if map_index == EMPTY_REFERENCE_MAP_SLOT {
                return Err(slot_index);
            }

            let is_match = matches!(
                &self.maps[map_index as usize],
                ReferenceMap::RepeatedReferenceOffsets {
                    count: existing_count,
                    element_size: existing_element_size,
                    offsets: existing_offsets,
                } if *existing_count == count
                    && *existing_element_size == element_size
                    && existing_offsets.as_slice() == offsets
            );
            if is_match {
                return Ok(map_index);
            }

            slot_index = (slot_index + 1) & mask;
        }
    }

    /// Hash one managed reference map for the reverse lookup.
    fn hash_reference_map(map: &ReferenceMap) -> u64 {
        let mut hasher = DefaultHasher::new();
        map.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash one borrowed repeated-offset map for the reverse lookup.
    fn hash_repeated_reference_offsets(count: u32, element_size: u32, offsets: &[u32]) -> u64 {
        let mut hasher = DefaultHasher::new();
        std::mem::discriminant(&ReferenceMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets: Vec::new(),
        })
        .hash(&mut hasher);
        count.hash(&mut hasher);
        element_size.hash(&mut hasher);
        offsets.hash(&mut hasher);
        hasher.finish()
    }

    /// Recompute the exact retained bytes for this table.
    fn recompute_retained_bytes(&mut self) {
        let mut retained_bytes = self.maps.capacity() * std::mem::size_of::<ReferenceMap>();

        for map in &self.maps {
            retained_bytes += map.retained_bytes();
        }

        retained_bytes += self.map_slots.capacity() * std::mem::size_of::<u32>();
        self.retained_bytes = retained_bytes;
    }

    /// Return the retained-byte delta after one potential slot-table rebuild.
    fn rebuild_retained_delta(&self) -> i64 {
        let slot_count = self.maps.len().saturating_mul(2).max(8).next_power_of_two();

        vec_capacity_bytes::<u32>(slot_count) as i64
            - vec_capacity_bytes::<u32>(self.map_slots.capacity()) as i64
    }

    /// Return the retained-byte delta for one new reference map after the slots are ready.
    fn intern_delta_after_rebuild(&self, map: &ReferenceMap) -> i64 {
        if self.lookup(map).is_ok() {
            return 0;
        }

        let needs_rehash = self.needs_rehash();
        let new_map_capacity =
            projected_vec_capacity::<ReferenceMap>(self.maps.len(), self.maps.capacity(), 1);
        let map_capacity_delta =
            vec_capacity_bytes_delta::<ReferenceMap>(self.maps.capacity(), new_map_capacity);
        let slot_capacity_delta = if needs_rehash {
            let slot_count = self
                .maps
                .len()
                .saturating_add(1)
                .saturating_mul(2)
                .max(8)
                .next_power_of_two();
            vec_capacity_bytes::<u32>(slot_count) as i64
                - vec_capacity_bytes::<u32>(self.map_slots.capacity()) as i64
        } else {
            0
        };

        map.retained_bytes() as i64 + map_capacity_delta + slot_capacity_delta
    }

    /// Return the retained-byte delta for one new repeated-offset map after the slots are ready.
    fn intern_repeated_delta_after_rebuild(
        &self,
        count: u32,
        element_size: u32,
        offsets: &[u32],
    ) -> i64 {
        if self
            .lookup_repeated_reference_offsets(count, element_size, offsets)
            .is_ok()
        {
            return 0;
        }

        self.intern_delta_after_rebuild(&ReferenceMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets: offsets.to_vec(),
        })
    }
}
