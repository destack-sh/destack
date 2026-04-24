use destack_mir as mir;
use serde::{Deserialize, Serialize};

use super::meta::ReferenceMeta;
use super::tag::ValueTag;
use crate::{
    FramePointer, HeapReference, RawPointer, SharedHeapReference, SharedRawPointer, StackPointer,
    StaticPointer,
};

const POINTER_BASE_MASK: u64 = 0xFFFF_FFFF;
const POINTER_SLOT_SHIFT: u64 = 32;
const STACK_INDEX_MASK: u64 = 0xFFFF;
const STACK_SLOT_SHIFT: u64 = 16;
const REF_META_SHIFT: u64 = 16;
const REF_META_MASK: u64 = 0xFF << REF_META_SHIFT;

/// Pack one native pointer-width payload into the value data lane.
const fn pack_pointer_bits(bits: usize) -> u64 {
    bits as u64
}

/// Unpack one native pointer-width payload from the value data lane.
const fn unpack_pointer_bits(bits: u64) -> usize {
    bits as usize
}

/// A runtime value in the VM.
///
/// Compact 16-byte representation using a packed data/meta layout.
/// The data field stores the payload, meta stores the tag and auxiliary bits.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "ValueEncoding", into = "ValueEncoding")]
#[repr(C)]
pub struct Value {
    /// The packed value payload.
    data: u64,
    /// The packed value metadata.
    meta: u64,
}

/// Serialized form of one packed VM value.
#[derive(Serialize, Deserialize)]
struct ValueEncoding {
    /// The packed value payload.
    data: u64,
    /// The packed value metadata.
    meta: u64,
}

impl From<Value> for ValueEncoding {
    fn from(value: Value) -> Self {
        Self {
            data: value.data,
            meta: value.meta,
        }
    }
}

impl TryFrom<ValueEncoding> for Value {
    type Error = String;

    fn try_from(encoded: ValueEncoding) -> Result<Self, Self::Error> {
        let value = Self {
            data: encoded.data,
            meta: encoded.meta,
        };

        if value.checked_tag().is_none() {
            return Err(format!("invalid value tag {}", value.tag_byte()));
        }

        Ok(value)
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::VOID
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.meta == other.meta
    }
}

impl Eq for Value {}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some(tag) = self.checked_tag() else {
            return write!(f, "InvalidValueTag({})", self.tag_byte());
        };

        match tag {
            ValueTag::Void => write!(f, "Void"),
            ValueTag::Bool => write!(f, "Bool({})", self.data != 0),
            ValueTag::Int => write!(
                f,
                "Int {{ value: {}, width: {} }}",
                self.data as i64,
                self.width()
            ),
            ValueTag::UInt => write!(
                f,
                "UInt {{ value: {}, width: {} }}",
                self.data,
                self.width()
            ),
            ValueTag::Float32 => write!(f, "Float32({})", f32::from_bits(self.data as u32)),
            ValueTag::Float64 => write!(f, "Float64({})", f64::from_bits(self.data)),
            ValueTag::Char => {
                let char_val = char::from_u32(self.data as u32).unwrap_or('\u{FFFD}');
                write!(f, "Char('{char_val}')")
            }
            ValueTag::HeapReference => {
                let reference = HeapReference::from_bits(unpack_pointer_bits(self.data));
                write!(f, "HeapReference(0x{:X})", reference.address())
            }
            ValueTag::SharedHeapReference => {
                let reference = SharedHeapReference::from_bits(unpack_pointer_bits(self.data));
                write!(f, "SharedHeapReference(0x{:X})", reference.address())
            }
            ValueTag::RawPointer => {
                let pointer = RawPointer::from_bits(unpack_pointer_bits(self.data));
                write!(f, "RawPointer(0x{:X})", pointer.address())
            }
            ValueTag::SharedRawPointer => {
                let pointer = SharedRawPointer::from_bits(unpack_pointer_bits(self.data));
                write!(f, "SharedRawPointer(0x{:X})", pointer.address())
            }
            ValueTag::StackPointer => {
                let pointer = self.stack_pointer_parts();
                if pointer.byte_offset == 0 {
                    write!(f, "StackPointer({}, {})", pointer.frame_idx, pointer.slot)
                } else {
                    write!(
                        f,
                        "StackPointer({}, {}, offset: {})",
                        pointer.frame_idx, pointer.slot, pointer.byte_offset
                    )
                }
            }
            ValueTag::FramePointer => {
                let pointer = self.frame_pointer_parts();
                if pointer.byte_offset == 0 {
                    write!(f, "FramePointer({}, {})", pointer.frame_idx, pointer.slot)
                } else {
                    write!(
                        f,
                        "FramePointer({}, {}, offset: {})",
                        pointer.frame_idx, pointer.slot, pointer.byte_offset
                    )
                }
            }
            ValueTag::StaticPointer => {
                let pointer = self.static_pointer_parts();
                if pointer.byte_offset == 0 {
                    write!(f, "StaticPointer({})", pointer.id.id)
                } else {
                    write!(
                        f,
                        "StaticPointer({}, offset: {})",
                        pointer.id.id, pointer.byte_offset
                    )
                }
            }
            ValueTag::FunctionPointer => write!(f, "FunctionPointer({})", self.data as u32),
        }
    }
}

impl From<&mir::Constant> for Value {
    fn from(constant: &mir::Constant) -> Self {
        match constant {
            mir::Constant::Null => Value::raw_pointer(RawPointer::NULL),
            mir::Constant::Boolean { value } => Value::bool(*value),
            mir::Constant::Int {
                value,
                width,
                is_signed: true,
            } => Value::int(*value, *width),
            mir::Constant::Int {
                value,
                width,
                is_signed: false,
            } => Value::uint(*value as u64, *width),
            mir::Constant::UInt { value, width } => Value::uint(*value, *width),
            mir::Constant::Float { bits, width: 32 } => {
                Value::float32(f32::from_bits(*bits as u32))
            }
            mir::Constant::Float { bits, width: _ } => Value::float64(f64::from_bits(*bits)),
            mir::Constant::Char { value } => Value::char(*value),
        }
    }
}

impl Value {
    /// Constant void value.
    pub const VOID: Self = Self { data: 0, meta: 0 };
    /// The packed byte width of one value.
    pub const BYTE_LEN: usize = std::mem::size_of::<Self>();

    /// Create metadata from tag and width.
    #[inline(always)]
    const fn make_meta(tag: ValueTag, width: u8) -> u64 {
        (tag as u64) | ((width as u64) << 8)
    }

    /// Attach reference metadata to this value.
    #[inline]
    pub fn with_reference_meta(mut self, meta: ReferenceMeta) -> Self {
        let bits = (meta.bits() as u64) << REF_META_SHIFT;
        self.meta = (self.meta & !REF_META_MASK) | bits;
        self
    }

    /// Get reference metadata from this value.
    #[inline]
    pub fn reference_meta(&self) -> ReferenceMeta {
        let bits = ((self.meta & REF_META_MASK) >> REF_META_SHIFT) as u8;
        ReferenceMeta::from_bits(bits)
    }

    /// Get the value tag.
    #[inline(always)]
    pub fn tag(&self) -> ValueTag {
        match self.checked_tag() {
            Some(tag) => tag,
            None => ValueTag::Void,
        }
    }

    /// Get the value tag when the packed tag byte is valid.
    #[inline(always)]
    pub fn checked_tag(&self) -> Option<ValueTag> {
        ValueTag::from_byte(self.tag_byte())
    }

    /// Return the raw packed tag byte.
    #[inline(always)]
    pub(crate) fn tag_byte(&self) -> u8 {
        (self.meta & 0xFF) as u8
    }

    /// Pack one stack pointer index field.
    #[inline]
    fn packed_stack_index(index: usize) -> Option<u64> {
        if index > STACK_INDEX_MASK as usize {
            return None;
        }

        Some(index as u64)
    }

    /// Pack one pointer offset field.
    #[inline]
    fn packed_pointer_offset(offset: usize) -> Option<u64> {
        if offset > POINTER_BASE_MASK as usize {
            return None;
        }

        Some(offset as u64)
    }

    /// Get the width (for Int/UInt).
    #[inline(always)]
    pub fn width(&self) -> u8 {
        ((self.meta >> 8) & 0xFF) as u8
    }

    /// Create a boolean value.
    #[inline]
    pub const fn bool(value: bool) -> Self {
        Self {
            data: value as u64,
            meta: Self::make_meta(ValueTag::Bool, 0),
        }
    }

    /// Create a signed integer value.
    #[inline]
    pub const fn int(value: i64, width: u8) -> Self {
        Self {
            data: value as u64,
            meta: Self::make_meta(ValueTag::Int, width),
        }
    }

    /// Create an int8 value.
    #[inline]
    pub const fn int8(value: i8) -> Self {
        Self::int(value as i64, 8)
    }

    /// Create an int16 value.
    #[inline]
    pub const fn int16(value: i16) -> Self {
        Self::int(value as i64, 16)
    }

    /// Create an int32 value.
    #[inline]
    pub const fn int32(value: i32) -> Self {
        Self::int(value as i64, 32)
    }

    /// Create an int64 value.
    #[inline]
    pub const fn int64(value: i64) -> Self {
        Self::int(value, 64)
    }

    /// Create an unsigned integer value.
    #[inline]
    pub const fn uint(value: u64, width: u8) -> Self {
        Self {
            data: value,
            meta: Self::make_meta(ValueTag::UInt, width),
        }
    }

    /// Create a uint8 value.
    #[inline]
    pub const fn uint8(value: u8) -> Self {
        Self::uint(value as u64, 8)
    }

    /// Create a uint16 value.
    #[inline]
    pub const fn uint16(value: u16) -> Self {
        Self::uint(value as u64, 16)
    }

    /// Create a uint32 value.
    #[inline]
    pub const fn uint32(value: u32) -> Self {
        Self::uint(value as u64, 32)
    }

    /// Create a uint64 value.
    #[inline]
    pub const fn uint64(value: u64) -> Self {
        Self::uint(value, 64)
    }

    /// Create a float32 value.
    #[inline]
    pub const fn float32(value: f32) -> Self {
        Self {
            data: value.to_bits() as u64,
            meta: Self::make_meta(ValueTag::Float32, 32),
        }
    }

    /// Create a float64 value.
    #[inline]
    pub const fn float64(value: f64) -> Self {
        Self {
            data: value.to_bits(),
            meta: Self::make_meta(ValueTag::Float64, 64),
        }
    }

    /// Create a char value.
    #[inline]
    pub const fn char(value: char) -> Self {
        Self {
            data: value as u64,
            meta: Self::make_meta(ValueTag::Char, 0),
        }
    }

    /// Create a heap reference value.
    #[inline]
    pub const fn heap_reference(reference: HeapReference) -> Self {
        Self {
            data: pack_pointer_bits(reference.bits()),
            meta: Self::make_meta(ValueTag::HeapReference, 0),
        }
    }

    /// Create a heap reference value with explicit metadata.
    #[inline]
    pub fn heap_reference_with_meta(reference: HeapReference, meta: ReferenceMeta) -> Self {
        Self::heap_reference(reference).with_reference_meta(meta)
    }

    /// Create a shared heap reference value.
    #[inline]
    pub const fn shared_heap_reference(reference: SharedHeapReference) -> Self {
        Self {
            data: pack_pointer_bits(reference.bits()),
            meta: Self::make_meta(ValueTag::SharedHeapReference, 0),
        }
    }

    /// Create a shared heap reference value with explicit metadata.
    #[inline]
    pub fn shared_heap_reference_with_meta(
        reference: SharedHeapReference,
        meta: ReferenceMeta,
    ) -> Self {
        Self::shared_heap_reference(reference).with_reference_meta(meta)
    }

    /// Create a raw pointer value.
    #[inline]
    pub const fn raw_pointer(ptr: RawPointer) -> Self {
        Self {
            data: pack_pointer_bits(ptr.bits()),
            meta: Self::make_meta(ValueTag::RawPointer, 0),
        }
    }

    /// Create a shared raw pointer value.
    #[inline]
    pub const fn shared_raw_pointer(ptr: SharedRawPointer) -> Self {
        Self {
            data: pack_pointer_bits(ptr.bits()),
            meta: Self::make_meta(ValueTag::SharedRawPointer, 0),
        }
    }

    /// Create a raw pointer value with explicit metadata.
    #[inline]
    pub fn raw_pointer_with_meta(ptr: RawPointer, meta: ReferenceMeta) -> Self {
        Self::raw_pointer(ptr).with_reference_meta(meta)
    }

    /// Create a shared raw pointer value with explicit metadata.
    #[inline]
    pub fn shared_raw_pointer_with_meta(ptr: SharedRawPointer, meta: ReferenceMeta) -> Self {
        Self::shared_raw_pointer(ptr).with_reference_meta(meta)
    }

    /// Create a stack pointer value.
    #[inline]
    pub fn stack_pointer(ptr: StackPointer) -> Option<Self> {
        let base = Self::packed_stack_index(ptr.frame_idx)?;
        let slot = Self::packed_stack_index(ptr.slot)?;
        let offset = Self::packed_pointer_offset(ptr.byte_offset)?;
        let packed = base | (slot << STACK_SLOT_SHIFT) | (offset << POINTER_SLOT_SHIFT);

        Some(Self {
            data: packed,
            meta: Self::make_meta(ValueTag::StackPointer, 0),
        })
    }

    /// Create a stack pointer value with explicit metadata.
    #[inline]
    pub fn stack_pointer_with_meta(ptr: StackPointer, meta: ReferenceMeta) -> Option<Self> {
        Some(Self::stack_pointer(ptr)?.with_reference_meta(meta))
    }

    /// Create a frame pointer value.
    #[inline]
    pub fn frame_pointer(ptr: FramePointer) -> Option<Self> {
        let base = Self::packed_stack_index(ptr.frame_idx)?;
        let slot = Self::packed_stack_index(ptr.slot)?;
        let offset = Self::packed_pointer_offset(ptr.byte_offset)?;
        let packed = base | (slot << STACK_SLOT_SHIFT) | (offset << POINTER_SLOT_SHIFT);

        Some(Self {
            data: packed,
            meta: Self::make_meta(ValueTag::FramePointer, 0),
        })
    }

    /// Create a frame pointer value with explicit metadata.
    #[inline]
    pub fn frame_pointer_with_meta(ptr: FramePointer, meta: ReferenceMeta) -> Option<Self> {
        Some(Self::frame_pointer(ptr)?.with_reference_meta(meta))
    }

    /// Create a static pointer value.
    #[inline]
    pub fn static_pointer(id: mir::LocalNodeId<mir::Global>) -> Option<Self> {
        Self::static_pointer_with_offset(id, 0)
    }

    /// Create a static pointer value with an explicit byte offset.
    #[inline]
    pub fn static_pointer_with_offset(
        id: mir::LocalNodeId<mir::Global>,
        byte_offset: usize,
    ) -> Option<Self> {
        let base = id.id as u64;
        let offset = Self::packed_pointer_offset(byte_offset)? << POINTER_SLOT_SHIFT;

        Some(Self {
            data: base | offset,
            meta: Self::make_meta(ValueTag::StaticPointer, 0),
        })
    }

    /// Create a static pointer value with explicit metadata.
    #[inline]
    pub fn static_pointer_with_meta(
        id: mir::LocalNodeId<mir::Global>,
        byte_offset: usize,
        meta: ReferenceMeta,
    ) -> Option<Self> {
        Some(Self::static_pointer_with_offset(id, byte_offset)?.with_reference_meta(meta))
    }

    /// Create a function pointer value.
    #[inline]
    pub fn function_pointer(id: mir::LocalNodeId<mir::Function>) -> Self {
        Self {
            data: id.id as u64,
            meta: Self::make_meta(ValueTag::FunctionPointer, 0),
        }
    }

    /// Check if value is truthy.
    #[inline]
    pub fn is_truthy(&self) -> bool {
        match self.tag() {
            ValueTag::Void => false,
            ValueTag::Bool => self.data != 0,
            ValueTag::Int => (self.data as i64) != 0,
            ValueTag::UInt => self.data != 0,
            ValueTag::Float32 => f32::from_bits(self.data as u32) != 0.0,
            ValueTag::Float64 => f64::from_bits(self.data) != 0.0,
            ValueTag::Char => true,
            ValueTag::HeapReference => self.data != 0,
            ValueTag::SharedHeapReference => self.data != 0,
            ValueTag::RawPointer => self.data != 0,
            ValueTag::SharedRawPointer => self.data != 0,
            ValueTag::StackPointer => true,
            ValueTag::FramePointer => true,
            ValueTag::StaticPointer => true,
            ValueTag::FunctionPointer => true,
        }
    }

    /// Try to cast to bool.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        if self.tag() == ValueTag::Bool {
            Some(self.data != 0)
        } else {
            None
        }
    }

    /// Try to cast to int (i64) if this is Int or UInt.
    #[inline]
    pub fn as_int(&self) -> Option<i64> {
        match self.tag() {
            ValueTag::Int => Some(self.data as i64),
            ValueTag::UInt => Some(self.data as i64),
            _ => None,
        }
    }

    /// Try to cast to uint (u64) if this is Int or UInt.
    #[inline]
    pub fn as_uint(&self) -> Option<u64> {
        match self.tag() {
            ValueTag::UInt => Some(self.data),
            ValueTag::Int => Some(self.data),
            _ => None,
        }
    }

    /// Try to cast to char.
    #[inline]
    pub fn as_char(&self) -> Option<char> {
        if self.tag() == ValueTag::Char {
            Some(self.data as u32).and_then(char::from_u32)
        } else {
            None
        }
    }

    /// Check if value is void.
    #[inline]
    pub fn is_void(&self) -> bool {
        self.tag() == ValueTag::Void
    }

    /// Check if value is a heap reference.
    #[inline]
    pub fn is_heap_reference(&self) -> bool {
        self.tag() == ValueTag::HeapReference
    }

    /// Check if value is a shared heap reference.
    #[inline]
    pub fn is_shared_heap_reference(&self) -> bool {
        self.tag() == ValueTag::SharedHeapReference
    }

    /// Try to get this value as a heap reference.
    #[inline]
    pub fn as_heap_reference(&self) -> Option<HeapReference> {
        match self.tag() {
            ValueTag::HeapReference => {
                Some(HeapReference::from_bits(unpack_pointer_bits(self.data)))
            }
            _ => None,
        }
    }

    /// Try to get this value as a shared heap reference.
    #[inline]
    pub fn as_shared_heap_reference(&self) -> Option<SharedHeapReference> {
        match self.tag() {
            ValueTag::SharedHeapReference => Some(SharedHeapReference::from_bits(
                unpack_pointer_bits(self.data),
            )),
            _ => None,
        }
    }

    /// Try to get this value as a raw pointer.
    #[inline]
    pub fn as_raw_pointer(&self) -> Option<RawPointer> {
        if self.tag() == ValueTag::RawPointer {
            Some(RawPointer::from_bits(unpack_pointer_bits(self.data)))
        } else {
            None
        }
    }

    /// Try to get this value as a shared raw pointer.
    #[inline]
    pub fn as_shared_raw_pointer(&self) -> Option<SharedRawPointer> {
        if self.tag() == ValueTag::SharedRawPointer {
            Some(SharedRawPointer::from_bits(unpack_pointer_bits(self.data)))
        } else {
            None
        }
    }

    /// Try to get this value as a stack pointer.
    #[inline]
    pub fn as_stack_pointer(&self) -> Option<StackPointer> {
        if self.tag() == ValueTag::StackPointer {
            return Some(self.stack_pointer_parts());
        }
        None
    }

    /// Try to get this value as a frame pointer.
    #[inline]
    pub fn as_frame_pointer(&self) -> Option<FramePointer> {
        if self.tag() == ValueTag::FramePointer {
            return Some(self.frame_pointer_parts());
        }
        None
    }

    /// Try to get this value as a static pointer.
    #[inline]
    pub fn as_static_pointer(&self) -> Option<StaticPointer> {
        if self.tag() == ValueTag::StaticPointer {
            return Some(self.static_pointer_parts());
        }
        None
    }

    /// Try to get this value as a function pointer.
    #[inline]
    pub fn as_function_pointer(&self) -> Option<mir::LocalNodeId<mir::Function>> {
        if self.tag() == ValueTag::FunctionPointer {
            return Some(mir::LocalNodeId::new(self.data as u32));
        }
        None
    }

    /// Try to get this value as an int with explicit width.
    #[inline]
    pub fn as_int_with_width(&self) -> Option<(i64, u8)> {
        if self.tag() == ValueTag::Int {
            return Some((self.data as i64, self.width()));
        }
        None
    }

    /// Try to get this value as a uint with explicit width.
    #[inline]
    pub fn as_uint_with_width(&self) -> Option<(u64, u8)> {
        if self.tag() == ValueTag::UInt {
            return Some((self.data, self.width()));
        }
        None
    }

    /// Try to get this value as float64.
    #[inline]
    pub fn as_float64(&self) -> Option<f64> {
        if self.tag() == ValueTag::Float64 {
            return Some(f64::from_bits(self.data));
        }
        None
    }

    /// Try to get this value as float32.
    #[inline]
    pub fn as_float32(&self) -> Option<f32> {
        if self.tag() == ValueTag::Float32 {
            return Some(f32::from_bits(self.data as u32));
        }
        None
    }

    /// Get raw data (for internal use).
    #[inline]
    pub fn raw_data(&self) -> u64 {
        self.data
    }

    /// Return the packed bytes for this value.
    #[inline]
    pub fn to_byte_array(self) -> [u8; Self::BYTE_LEN] {
        let mut bytes = [0u8; Self::BYTE_LEN];
        bytes[..8].copy_from_slice(&self.data.to_le_bytes());
        bytes[8..].copy_from_slice(&self.meta.to_le_bytes());
        bytes
    }

    /// Restore one value from one packed byte slice.
    #[inline]
    pub fn from_byte_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != Self::BYTE_LEN {
            return None;
        }

        let mut data = [0u8; 8];
        let mut meta = [0u8; 8];
        data.copy_from_slice(&bytes[..8]);
        meta.copy_from_slice(&bytes[8..]);

        let value = Self {
            data: u64::from_le_bytes(data),
            meta: u64::from_le_bytes(meta),
        };

        value.checked_tag()?;

        Some(value)
    }

    /// Decode one stack pointer from the packed value payload.
    #[inline]
    fn stack_pointer_parts(&self) -> StackPointer {
        let frame_idx = (self.data & STACK_INDEX_MASK) as usize;
        let slot = ((self.data >> STACK_SLOT_SHIFT) & STACK_INDEX_MASK) as usize;
        let byte_offset = ((self.data >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;

        StackPointer {
            frame_idx,
            slot,
            byte_offset,
        }
    }

    /// Decode one frame pointer from the packed value payload.
    #[inline]
    fn frame_pointer_parts(&self) -> FramePointer {
        let frame_idx = (self.data & STACK_INDEX_MASK) as usize;
        let slot = ((self.data >> STACK_SLOT_SHIFT) & STACK_INDEX_MASK) as usize;
        let byte_offset = ((self.data >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;

        FramePointer {
            frame_idx,
            slot,
            byte_offset,
        }
    }

    /// Decode one static pointer from the packed value payload.
    #[inline]
    fn static_pointer_parts(&self) -> StaticPointer {
        let base = (self.data & POINTER_BASE_MASK) as u32;
        let byte_offset = ((self.data >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;
        let id = mir::LocalNodeId::new(base);

        StaticPointer { id, byte_offset }
    }
}
