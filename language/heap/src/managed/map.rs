use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::value::{ManagedReference, Value};

/// Reference-scanning metadata for one managed allocation payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceMap {
    /// Allocation contains no managed references.
    None,
    /// Allocation stores direct managed-reference words at fixed byte offsets.
    ReferenceOffsets {
        /// Byte offsets of encoded managed references.
        offsets: Box<[u32]>,
    },
    /// Allocation stores full VM values at fixed byte offsets.
    ValueOffsets {
        /// Byte offsets of encoded VM values.
        offsets: Box<[u32]>,
    },
    /// Allocation stores repeated elements with managed-reference words at fixed element offsets.
    RepeatedReferenceOffsets {
        /// The number of elements in the allocation.
        count: u32,
        /// The element byte stride.
        element_size: u32,
        /// Managed-reference byte offsets within each element.
        offsets: Box<[u32]>,
    },
}

/// Tracing failure for one managed reference map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceMapError {
    /// The configured managed-reference width is unsupported.
    UnsupportedManagedReferenceWidth { bytes: u8 },
    /// One decoded managed-reference window has an unsupported width.
    UnsupportedReferenceWindowWidth { bytes: usize },
    /// One traced field width overflowed its byte offset.
    OffsetOverflow { start: usize, width: usize },
    /// One traced field extended past the provided payload bytes.
    TruncatedPayload {
        start: usize,
        width: usize,
        len: usize,
    },
    /// One traced field could not be read from one random-access reader.
    TruncatedReaderWindow { start: usize, width: usize },
    /// One traced value payload was invalid.
    InvalidValuePayload { start: usize },
}

impl Display for ReferenceMapError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedManagedReferenceWidth { bytes } => {
                write!(
                    formatter,
                    "unsupported managed reference width for tracing: {bytes}"
                )
            }
            Self::UnsupportedReferenceWindowWidth { bytes } => {
                write!(
                    formatter,
                    "unsupported managed reference width for tracing window: {bytes}"
                )
            }
            Self::OffsetOverflow { start, width } => {
                write!(
                    formatter,
                    "managed reference offset overflow while tracing: start={start}, width={width}"
                )
            }
            Self::TruncatedPayload { start, width, len } => {
                write!(
                    formatter,
                    "truncated managed reference payload while tracing: start={start}, width={width}, len={len}"
                )
            }
            Self::TruncatedReaderWindow { start, width } => {
                write!(
                    formatter,
                    "truncated managed reference payload while tracing: start={start}, width={width}"
                )
            }
            Self::InvalidValuePayload { start } => {
                write!(
                    formatter,
                    "invalid value payload while tracing managed references: start={start}"
                )
            }
        }
    }
}

impl Error for ReferenceMapError {}

impl ReferenceMap {
    /// Return an empty reference map.
    pub const fn empty() -> Self {
        Self::None
    }

    /// Report whether this reference map can reach managed references.
    pub fn has_managed_edges(&self) -> bool {
        // check the encoded shape directly
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
    ) -> Result<bool, ReferenceMapError> {
        // empty writes cannot overlap anything
        if len == 0 {
            return Ok(false);
        }

        // treat overflow as an overlapping write
        let Some(end) = start.checked_add(len) else {
            return Ok(true);
        };

        // validate the traced reference width once
        let managed_reference_bytes = Self::managed_reference_width(managed_reference_bytes)?;

        // check overlap against the encoded shape
        let is_overlapping = match self {
            Self::None => false,
            Self::ReferenceOffsets { offsets } => {
                Self::touches_reference_offsets(offsets, start, end, managed_reference_bytes)
            }
            Self::ValueOffsets { offsets } => Self::touches_value_offsets(offsets, start, end),
            Self::RepeatedReferenceOffsets {
                count,
                element_size,
                offsets,
            } => Self::touches_repeated_reference_offsets(
                *count,
                *element_size,
                offsets,
                start,
                end,
                managed_reference_bytes,
            ),
        };

        Ok(is_overlapping)
    }

    /// Visit each managed reference stored in the given payload bytes.
    pub fn trace_references(
        &self,
        bytes: &[u8],
        managed_reference_bytes: u8,
        visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        // validate the traced reference width once
        let managed_reference_bytes = Self::managed_reference_width(managed_reference_bytes)?;

        // walk the encoded shape directly
        match self {
            Self::None => {}
            Self::ReferenceOffsets { offsets } => {
                Self::trace_reference_offsets(offsets, bytes, managed_reference_bytes, visit)?
            }
            Self::ValueOffsets { offsets } => Self::trace_value_offsets(offsets, bytes, visit)?,
            Self::RepeatedReferenceOffsets {
                count,
                element_size,
                offsets,
            } => Self::trace_repeated_reference_offsets(
                *count,
                *element_size,
                offsets,
                bytes,
                managed_reference_bytes,
                visit,
            )?,
        }

        Ok(())
    }

    /// Visit each managed reference using one random-access byte reader.
    pub fn trace_references_in_reader(
        &self,
        managed_reference_bytes: u8,
        fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
        visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        // validate the traced reference width once
        let managed_reference_bytes = Self::managed_reference_width(managed_reference_bytes)?;

        // walk the encoded shape through the reader
        match self {
            Self::None => {}
            Self::ReferenceOffsets { offsets } => Self::trace_reference_offsets_in_reader(
                offsets,
                managed_reference_bytes,
                fill_bytes,
                visit,
            )?,
            Self::ValueOffsets { offsets } => {
                Self::trace_value_offsets_in_reader(offsets, fill_bytes, visit)?
            }
            Self::RepeatedReferenceOffsets {
                count,
                element_size,
                offsets,
            } => Self::trace_repeated_reference_offsets_in_reader(
                *count,
                *element_size,
                offsets,
                managed_reference_bytes,
                fill_bytes,
                visit,
            )?,
        }

        Ok(())
    }

    /// Visit each managed reference whose encoded bytes overlap the given byte range.
    pub fn trace_references_in_reader_range(
        &self,
        start: usize,
        len: usize,
        managed_reference_bytes: u8,
        fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
        visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        // empty writes cannot overlap anything
        if len == 0 {
            return Ok(());
        }

        // drop overflowed windows rather than tracing nonsense
        let Some(end) = start.checked_add(len) else {
            return Ok(());
        };

        // validate the traced reference width once
        let managed_reference_bytes = Self::managed_reference_width(managed_reference_bytes)?;

        // walk only overlapping fields in the encoded shape
        match self {
            Self::None => {}
            Self::ReferenceOffsets { offsets } => Self::trace_reference_offsets_in_reader_range(
                offsets,
                start,
                end,
                managed_reference_bytes,
                fill_bytes,
                visit,
            )?,
            Self::ValueOffsets { offsets } => {
                Self::trace_value_offsets_in_reader_range(offsets, start, end, fill_bytes, visit)?
            }
            Self::RepeatedReferenceOffsets {
                count,
                element_size,
                offsets,
            } => Self::trace_repeated_reference_offsets_in_reader_range(
                *count,
                *element_size,
                offsets,
                start,
                end,
                managed_reference_bytes,
                fill_bytes,
                visit,
            )?,
        }

        Ok(())
    }

    /// Report whether one direct managed-reference table overlaps the given byte range.
    fn touches_reference_offsets(
        offsets: &[u32],
        start: usize,
        end: usize,
        managed_reference_bytes: usize,
    ) -> bool {
        // scan each encoded managed-reference field
        offsets.iter().copied().any(|offset| {
            Self::ranges_overlap(start, end, offset as usize, managed_reference_bytes)
        })
    }

    /// Report whether one value table overlaps the given byte range.
    fn touches_value_offsets(offsets: &[u32], start: usize, end: usize) -> bool {
        // scan each full value slot
        offsets
            .iter()
            .copied()
            .any(|offset| Self::ranges_overlap(start, end, offset as usize, Value::BYTE_LEN))
    }

    /// Report whether one repeated managed-reference table overlaps the given byte range.
    fn touches_repeated_reference_offsets(
        count: u32,
        element_size: u32,
        offsets: &[u32],
        start: usize,
        end: usize,
        managed_reference_bytes: usize,
    ) -> bool {
        // scan each repeated field definition
        offsets.iter().copied().any(|offset| {
            Self::repeated_ranges_overlap(
                start,
                end,
                count as usize,
                element_size as usize,
                offset as usize,
                managed_reference_bytes,
            )
        })
    }

    /// Visit each direct managed reference stored in one payload.
    fn trace_reference_offsets(
        offsets: &[u32],
        bytes: &[u8],
        managed_reference_bytes: usize,
        mut visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        // decode each direct managed-reference field
        for &offset in offsets {
            let start = offset as usize;
            let reference = Self::decode_managed_reference(bytes, start, managed_reference_bytes)?;

            visit(reference);
        }

        Ok(())
    }

    /// Visit each managed reference stored inside one full value payload.
    fn trace_value_offsets(
        offsets: &[u32],
        bytes: &[u8],
        mut visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        // decode each full value slot
        for &offset in offsets {
            let start = offset as usize;
            let reference = Self::decode_value_reference(bytes, start)?;

            // visit only managed-reference values
            if let Some(reference) = reference {
                visit(reference);
            }
        }

        Ok(())
    }

    /// Visit each repeated managed reference stored in one payload.
    fn trace_repeated_reference_offsets(
        count: u32,
        element_size: u32,
        offsets: &[u32],
        bytes: &[u8],
        managed_reference_bytes: usize,
        mut visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        let element_size = element_size as usize;

        // walk each repeated element
        for index in 0..(count as usize) {
            let base = index.saturating_mul(element_size);

            // decode each managed-reference field within that element
            for &offset in offsets {
                let start = base.saturating_add(offset as usize);
                let reference =
                    Self::decode_managed_reference(bytes, start, managed_reference_bytes)?;

                visit(reference);
            }
        }

        Ok(())
    }

    /// Visit each direct managed reference through one random-access reader.
    fn trace_reference_offsets_in_reader(
        offsets: &[u32],
        managed_reference_bytes: usize,
        mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
        mut visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        let mut raw = [0u8; 8];

        // read and decode each direct managed-reference field
        for &offset in offsets {
            let start = offset as usize;
            let buffer = &mut raw[..managed_reference_bytes];

            // fail loudly on truncated reader windows
            if !fill_bytes(start, buffer) {
                return Err(ReferenceMapError::TruncatedReaderWindow {
                    start,
                    width: managed_reference_bytes,
                });
            }

            let reference = Self::decode_managed_reference_window(buffer)?;
            visit(reference);
        }

        Ok(())
    }

    /// Visit each managed reference stored inside one value slot through one reader.
    fn trace_value_offsets_in_reader(
        offsets: &[u32],
        mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
        mut visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        let mut raw = [0u8; Value::BYTE_LEN];

        // read and decode each full value slot
        for &offset in offsets {
            let start = offset as usize;
            let buffer = &mut raw[..Value::BYTE_LEN];

            // fail loudly on truncated reader windows
            if !fill_bytes(start, buffer) {
                return Err(ReferenceMapError::TruncatedReaderWindow {
                    start,
                    width: Value::BYTE_LEN,
                });
            }

            let reference = Self::decode_value_reference_window(buffer, start)?;

            // visit only managed-reference values
            if let Some(reference) = reference {
                visit(reference);
            }
        }

        Ok(())
    }

    /// Visit each repeated managed reference through one random-access reader.
    fn trace_repeated_reference_offsets_in_reader(
        count: u32,
        element_size: u32,
        offsets: &[u32],
        managed_reference_bytes: usize,
        mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
        mut visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        let element_size = element_size as usize;
        let mut raw = [0u8; 8];

        // walk each repeated element
        for index in 0..(count as usize) {
            let base = index.saturating_mul(element_size);

            // read and decode each managed-reference field within that element
            for &offset in offsets {
                let start = base.saturating_add(offset as usize);
                let buffer = &mut raw[..managed_reference_bytes];

                // fail loudly on truncated reader windows
                if !fill_bytes(start, buffer) {
                    return Err(ReferenceMapError::TruncatedReaderWindow {
                        start,
                        width: managed_reference_bytes,
                    });
                }

                let reference = Self::decode_managed_reference_window(buffer)?;
                visit(reference);
            }
        }

        Ok(())
    }

    /// Visit each overlapping direct managed reference through one reader.
    fn trace_reference_offsets_in_reader_range(
        offsets: &[u32],
        start: usize,
        end: usize,
        managed_reference_bytes: usize,
        mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
        mut visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        let mut raw = [0u8; 8];

        // walk each direct managed-reference field
        for &offset in offsets {
            let field_start = offset as usize;

            // skip non-overlapping fields
            if !Self::ranges_overlap(start, end, field_start, managed_reference_bytes) {
                continue;
            }

            let buffer = &mut raw[..managed_reference_bytes];

            // fail loudly on truncated reader windows
            if !fill_bytes(field_start, buffer) {
                return Err(ReferenceMapError::TruncatedReaderWindow {
                    start: field_start,
                    width: managed_reference_bytes,
                });
            }

            let reference = Self::decode_managed_reference_window(buffer)?;
            visit(reference);
        }

        Ok(())
    }

    /// Visit each overlapping managed reference stored inside one value slot through one reader.
    fn trace_value_offsets_in_reader_range(
        offsets: &[u32],
        start: usize,
        end: usize,
        mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
        mut visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        let mut raw = [0u8; Value::BYTE_LEN];

        // walk each full value slot
        for &offset in offsets {
            let field_start = offset as usize;

            // skip non-overlapping fields
            if !Self::ranges_overlap(start, end, field_start, Value::BYTE_LEN) {
                continue;
            }

            let buffer = &mut raw[..Value::BYTE_LEN];

            // fail loudly on truncated reader windows
            if !fill_bytes(field_start, buffer) {
                return Err(ReferenceMapError::TruncatedReaderWindow {
                    start: field_start,
                    width: Value::BYTE_LEN,
                });
            }

            let reference = Self::decode_value_reference_window(buffer, field_start)?;

            // visit only managed-reference values
            if let Some(reference) = reference {
                visit(reference);
            }
        }

        Ok(())
    }

    /// Visit each overlapping repeated managed reference through one reader.
    fn trace_repeated_reference_offsets_in_reader_range(
        count: u32,
        element_size: u32,
        offsets: &[u32],
        start: usize,
        end: usize,
        managed_reference_bytes: usize,
        mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
        mut visit: impl FnMut(ManagedReference),
    ) -> Result<(), ReferenceMapError> {
        let element_size = element_size as usize;
        let mut raw = [0u8; 8];

        // walk each repeated field definition
        for &offset in offsets {
            let Some((first_index, last_index)) = Self::overlapping_repeated_index_range(
                start,
                end,
                count as usize,
                element_size,
                offset as usize,
                managed_reference_bytes,
            ) else {
                continue;
            };

            // walk the overlapping repeated elements for that field
            for index in first_index..=last_index {
                let field_start = index
                    .saturating_mul(element_size)
                    .saturating_add(offset as usize);
                let buffer = &mut raw[..managed_reference_bytes];

                // fail loudly on truncated reader windows
                if !fill_bytes(field_start, buffer) {
                    return Err(ReferenceMapError::TruncatedReaderWindow {
                        start: field_start,
                        width: managed_reference_bytes,
                    });
                }

                let reference = Self::decode_managed_reference_window(buffer)?;
                visit(reference);
            }
        }

        Ok(())
    }

    /// Return the validated byte width for one traced managed reference.
    fn managed_reference_width(managed_reference_bytes: u8) -> Result<usize, ReferenceMapError> {
        // accept only the currently supported encodings
        match managed_reference_bytes {
            4 | 8 => Ok(managed_reference_bytes as usize),
            _ => Err(ReferenceMapError::UnsupportedManagedReferenceWidth {
                bytes: managed_reference_bytes,
            }),
        }
    }

    /// Decode one managed reference from one traced byte window.
    fn decode_managed_reference(
        bytes: &[u8],
        start: usize,
        managed_reference_bytes: usize,
    ) -> Result<ManagedReference, ReferenceMapError> {
        // resolve the traced byte window
        let end = start.checked_add(managed_reference_bytes).ok_or(
            ReferenceMapError::OffsetOverflow {
                start,
                width: managed_reference_bytes,
            },
        )?;
        let window = bytes
            .get(start..end)
            .ok_or(ReferenceMapError::TruncatedPayload {
                start,
                width: managed_reference_bytes,
                len: bytes.len(),
            })?;

        // decode the resolved window
        Self::decode_managed_reference_window(window)
    }

    /// Decode one managed reference from one traced byte window.
    fn decode_managed_reference_window(
        window: &[u8],
    ) -> Result<ManagedReference, ReferenceMapError> {
        // decode according to the encoded width
        match window.len() {
            4 => {
                let mut raw = [0u8; 4];
                raw.copy_from_slice(window);

                Ok(ManagedReference::from_bits(u64::from(u32::from_le_bytes(
                    raw,
                ))))
            }
            8 => {
                let mut raw = [0u8; 8];
                raw.copy_from_slice(window);

                Ok(ManagedReference::from_bits(u64::from_le_bytes(raw)))
            }
            bytes => Err(ReferenceMapError::UnsupportedReferenceWindowWidth { bytes }),
        }
    }

    /// Decode one managed reference stored inside one full value payload.
    fn decode_value_reference(
        bytes: &[u8],
        start: usize,
    ) -> Result<Option<ManagedReference>, ReferenceMapError> {
        // resolve the traced byte window
        let end = start
            .checked_add(Value::BYTE_LEN)
            .ok_or(ReferenceMapError::OffsetOverflow {
                start,
                width: Value::BYTE_LEN,
            })?;
        let window = bytes
            .get(start..end)
            .ok_or(ReferenceMapError::TruncatedPayload {
                start,
                width: Value::BYTE_LEN,
                len: bytes.len(),
            })?;

        // decode the resolved value window
        Self::decode_value_reference_window(window, start)
    }

    /// Decode one managed reference stored inside one full value window.
    fn decode_value_reference_window(
        window: &[u8],
        start: usize,
    ) -> Result<Option<ManagedReference>, ReferenceMapError> {
        // decode the value payload first
        let value = Value::from_byte_slice(window)
            .ok_or(ReferenceMapError::InvalidValuePayload { start })?;

        // then select only managed references
        Ok(value.as_managed_reference())
    }

    /// Report whether one byte range overlaps one fixed-width field range.
    fn ranges_overlap(
        left_start: usize,
        left_end: usize,
        right_start: usize,
        right_len: usize,
    ) -> bool {
        // treat overflow on the right range as overlapping
        let Some(right_end) = right_start.checked_add(right_len) else {
            return true;
        };

        // then compare the half-open ranges
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
        Self::overlapping_repeated_index_range(start, end, count, element_size, offset, width)
            .is_some()
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
        // reject empty repeated fields first
        if count == 0 || width == 0 {
            return None;
        }

        // treat zero-stride repeated fields as one shared field
        if element_size == 0 {
            return Self::ranges_overlap(start, end, offset, width).then_some((0, count - 1));
        }

        // convert into signed math so we can reason about prefix overlap cleanly
        let start = start as i128;
        let end = end as i128;
        let count = count as i128;
        let element_size = element_size as i128;
        let offset = offset as i128;
        let width = width as i128;
        let low_numerator = start - offset - width + 1;
        let high_numerator = end - offset - 1;
        let low = Self::div_ceil_i128(low_numerator, element_size).max(0);
        let high = Self::div_floor_i128(high_numerator, element_size).min(count - 1);

        // keep only non-empty index ranges
        (low <= high).then_some((low as usize, high as usize))
    }

    /// Return floor(lhs / rhs) for signed integers with positive rhs.
    fn div_floor_i128(lhs: i128, rhs: i128) -> i128 {
        debug_assert!(rhs > 0);

        // compute the truncated division first
        let quotient = lhs / rhs;
        let remainder = lhs % rhs;

        // then correct negative partial quotients downward
        if remainder < 0 {
            quotient - 1
        } else {
            quotient
        }
    }

    /// Return ceil(lhs / rhs) for signed integers with positive rhs.
    fn div_ceil_i128(lhs: i128, rhs: i128) -> i128 {
        debug_assert!(rhs > 0);

        // compute the truncated division first
        let quotient = lhs / rhs;
        let remainder = lhs % rhs;

        // then correct positive partial quotients upward
        if remainder > 0 {
            quotient + 1
        } else {
            quotient
        }
    }
}
