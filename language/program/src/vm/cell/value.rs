use crate::{CellLayout, GlobalAddress};
use destack_heap::{HeapReference, SharedHeapReference};

use super::{Cell, FramePointer, FunctionPointer, StackPointer};

impl CellLayout {
    /// Decode raw memory bits into one VM cell.
    #[inline(always)]
    pub fn decode(self, raw: u64) -> Cell {
        match self {
            Self::Void => Cell::ZERO,
            Self::Boolean => Cell::bool(raw != 0),
            Self::Int { width } => Cell::int(raw as i64, width),
            Self::Uint { width } => Cell::uint(raw, width),
            Self::Float16 | Self::Bfloat16 => Cell::from_bits(raw),
            Self::Float32 => Cell::float32(f32::from_bits(raw as u32)),
            Self::Float64 => Cell::float64(f64::from_bits(raw)),
            Self::HeapReference => Cell::heap_reference(HeapReference::from_bits(raw as usize)),
            Self::SharedHeapReference => {
                Cell::shared_heap_reference(SharedHeapReference::from_bits(raw as usize))
            }
            Self::Address => Cell::address(raw as usize),
            Self::StackPointer => Cell::stack_pointer(StackPointer::from_offset(raw as usize)),
            Self::FramePointer => Cell::frame_pointer(FramePointer::from_offset(raw as usize)),
            Self::GlobalAddress => Cell::global_address(GlobalAddress::from_bits(raw)),
            Self::FunctionPointer => {
                Cell::function_pointer(FunctionPointer::from_bits(raw as usize))
            }
        }
    }

    /// Encode one VM cell into raw memory bits.
    #[inline(always)]
    pub fn encode(self, value: Cell) -> u64 {
        match self {
            Self::Void => 0,
            Self::Boolean => u64::from(value.as_bool()),
            Self::Int { .. } => value.bits(),
            Self::Uint { .. } => value.as_u64(),
            Self::Float16 | Self::Bfloat16 => value.bits(),
            Self::Float32 => value.as_f32().to_bits() as u64,
            Self::Float64 => value.as_f64().to_bits(),
            Self::HeapReference => value.as_heap_reference().bits() as u64,
            Self::SharedHeapReference => value.as_shared_heap_reference().bits() as u64,
            Self::Address => value.as_address() as u64,
            Self::StackPointer => value.as_stack_pointer().bits() as u64,
            Self::FramePointer => value.as_frame_pointer().bits() as u64,
            Self::GlobalAddress => value.as_global_address().bits(),
            Self::FunctionPointer => value.as_function_pointer().bits() as u64,
        }
    }
}

/// One cell encoded as raw bytes.
#[derive(Debug)]
pub struct CellEncoding {
    /// The byte buffer.
    bytes: [u8; Cell::BYTE_LEN],
    /// The number of initialized bytes.
    len: usize,
}

impl CellEncoding {
    /// Return the initialized bytes.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Return the initialized byte count.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return whether the encoding is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Encode one VM cell into raw bits for the given format.
pub fn encode_cell_bits(format: CellLayout, value: Cell, pointer_bytes: u8) -> (u64, usize) {
    let raw = format.encode(value);
    let byte_len = format.byte_len(pointer_bytes as usize);

    (raw, byte_len)
}

/// Encode one VM cell into scalar bytes.
pub fn encode_cell_bytes(format: CellLayout, value: Cell, pointer_bytes: u8) -> CellEncoding {
    let (raw, byte_len) = encode_cell_bits(format, value, pointer_bytes);

    CellEncoding {
        bytes: raw.to_le_bytes(),
        len: byte_len,
    }
}
