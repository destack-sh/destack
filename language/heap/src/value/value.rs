use destack_mir as mir;
use serde::{Deserialize, Serialize};

use super::meta::ReferenceMeta;
use super::pointer::{
    GlobalPointer, LocalPointer, ManagedReference, POINTER_BASE_MASK, POINTER_SLOT_SHIFT,
    REF_META_MASK, REF_META_SHIFT, RawPointer, STACK_INDEX_MASK, STACK_SLOT_SHIFT,
    SharedManagedReference, SharedRawPointer, StackPointer,
};
use super::tag::ValueTag;

/// A runtime value in the VM.
///
/// Compact 16-byte representation using a packed data/meta layout.
/// The data field stores the actual value, meta stores the tag and width.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct Value {
    /// The value data (i64, u64, f64 bits, pointer id, etc.).
    data: u64,
    /// Metadata: tag in low byte, width in second byte.
    meta: u64,
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
        match self.tag() {
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
            ValueTag::ManagedReference => {
                let reference = ManagedReference::from_bits(self.data);
                if reference.slot_offset() == 0 {
                    write!(f, "ManagedReference({})", reference.id())
                } else {
                    write!(
                        f,
                        "ManagedReference({}, slot {})",
                        reference.id(),
                        reference.slot_offset()
                    )
                }
            }
            ValueTag::SharedManagedReference => {
                let reference = SharedManagedReference::from_bits(self.data);
                if reference.slot_offset() == 0 {
                    write!(f, "SharedManagedReference({})", reference.id())
                } else {
                    write!(
                        f,
                        "SharedManagedReference({}, slot {})",
                        reference.id(),
                        reference.slot_offset()
                    )
                }
            }
            ValueTag::RawPointer => {
                let pointer = RawPointer::from_bits(self.data);
                if pointer.slot_offset() == 0 {
                    write!(f, "RawPointer({})", pointer.id())
                } else {
                    write!(
                        f,
                        "RawPointer({}, offset: {})",
                        pointer.id(),
                        pointer.slot_offset()
                    )
                }
            }
            ValueTag::SharedRawPointer => {
                let pointer = SharedRawPointer::from_bits(self.data);
                if pointer.byte_offset() == 0 {
                    write!(f, "SharedRawPointer({})", pointer.id())
                } else {
                    write!(
                        f,
                        "SharedRawPointer({}, offset: {})",
                        pointer.id(),
                        pointer.byte_offset()
                    )
                }
            }
            ValueTag::StackPointer => {
                let pointer = self.stack_pointer_parts();
                if pointer.slot_offset == 0 {
                    write!(f, "StackPointer({}, {})", pointer.frame_idx, pointer.slot)
                } else {
                    write!(
                        f,
                        "StackPointer({}, {}, offset: {})",
                        pointer.frame_idx, pointer.slot, pointer.slot_offset
                    )
                }
            }
            ValueTag::LocalPointer => {
                let pointer = self.local_pointer_parts();
                if pointer.slot_offset == 0 {
                    write!(f, "LocalPointer({}, {})", pointer.frame_idx, pointer.local)
                } else {
                    write!(
                        f,
                        "LocalPointer({}, {}, offset: {})",
                        pointer.frame_idx, pointer.local, pointer.slot_offset
                    )
                }
            }
            ValueTag::GlobalPointer => {
                let pointer = self.global_pointer_parts();
                if pointer.slot_offset == 0 {
                    write!(f, "GlobalPointer({})", pointer.id.id)
                } else {
                    write!(
                        f,
                        "GlobalPointer({}, offset: {})",
                        pointer.id.id, pointer.slot_offset
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
    fn tag_byte(&self) -> u8 {
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

    /// Create a managed heap reference value.
    #[inline]
    pub const fn managed_reference(reference: ManagedReference) -> Self {
        Self {
            data: reference.0,
            meta: Self::make_meta(ValueTag::ManagedReference, 0),
        }
    }

    /// Create a managed heap reference value with explicit metadata.
    #[inline]
    pub fn managed_reference_with_meta(reference: ManagedReference, meta: ReferenceMeta) -> Self {
        Self::managed_reference(reference).with_reference_meta(meta)
    }

    /// Create a shared managed heap reference value.
    #[inline]
    pub const fn shared_managed_reference(reference: SharedManagedReference) -> Self {
        Self {
            data: reference.0,
            meta: Self::make_meta(ValueTag::SharedManagedReference, 0),
        }
    }

    /// Create a shared managed heap reference value with explicit metadata.
    #[inline]
    pub fn shared_managed_reference_with_meta(
        reference: SharedManagedReference,
        meta: ReferenceMeta,
    ) -> Self {
        Self::shared_managed_reference(reference).with_reference_meta(meta)
    }

    /// Create a raw pointer value.
    #[inline]
    pub const fn raw_pointer(ptr: RawPointer) -> Self {
        Self {
            data: ptr.0,
            meta: Self::make_meta(ValueTag::RawPointer, 0),
        }
    }

    /// Create a shared raw pointer value.
    #[inline]
    pub const fn shared_raw_pointer(ptr: SharedRawPointer) -> Self {
        Self {
            data: ptr.0,
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
        let offset = Self::packed_pointer_offset(ptr.slot_offset)?;
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

    /// Create a local pointer value.
    #[inline]
    pub fn local_pointer(ptr: LocalPointer) -> Option<Self> {
        let base = Self::packed_stack_index(ptr.frame_idx)?;
        let slot = Self::packed_stack_index(ptr.local)?;
        let offset = Self::packed_pointer_offset(ptr.slot_offset)?;
        let packed = base | (slot << STACK_SLOT_SHIFT) | (offset << POINTER_SLOT_SHIFT);

        Some(Self {
            data: packed,
            meta: Self::make_meta(ValueTag::LocalPointer, 0),
        })
    }

    /// Create a local pointer value with explicit metadata.
    #[inline]
    pub fn local_pointer_with_meta(ptr: LocalPointer, meta: ReferenceMeta) -> Option<Self> {
        Some(Self::local_pointer(ptr)?.with_reference_meta(meta))
    }

    /// Create a global pointer value.
    #[inline]
    pub fn global_pointer(id: mir::LocalNodeId<mir::Global>) -> Option<Self> {
        Self::global_pointer_with_offset(id, 0)
    }

    /// Create a global pointer value with an explicit slot offset.
    #[inline]
    pub fn global_pointer_with_offset(
        id: mir::LocalNodeId<mir::Global>,
        slot_offset: usize,
    ) -> Option<Self> {
        let base = id.id as u64;
        let slot = Self::packed_pointer_offset(slot_offset)? << POINTER_SLOT_SHIFT;

        Some(Self {
            data: base | slot,
            meta: Self::make_meta(ValueTag::GlobalPointer, 0),
        })
    }

    /// Create a global pointer value with explicit metadata.
    #[inline]
    pub fn global_pointer_with_meta(
        id: mir::LocalNodeId<mir::Global>,
        slot_offset: usize,
        meta: ReferenceMeta,
    ) -> Option<Self> {
        Some(Self::global_pointer_with_offset(id, slot_offset)?.with_reference_meta(meta))
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
            ValueTag::ManagedReference => self.data != 0,
            ValueTag::SharedManagedReference => self.data != 0,
            ValueTag::RawPointer => self.data != 0,
            ValueTag::SharedRawPointer => self.data != 0,
            ValueTag::StackPointer => true,
            ValueTag::LocalPointer => true,
            ValueTag::GlobalPointer => true,
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

    /// Check if value is a managed reference.
    #[inline]
    pub fn is_managed_reference(&self) -> bool {
        self.tag() == ValueTag::ManagedReference
    }

    /// Check if value is a shared managed reference.
    #[inline]
    pub fn is_shared_managed_reference(&self) -> bool {
        self.tag() == ValueTag::SharedManagedReference
    }

    /// Try to get this value as a managed reference.
    #[inline]
    pub fn as_managed_reference(&self) -> Option<ManagedReference> {
        match self.tag() {
            ValueTag::ManagedReference => Some(ManagedReference(self.data)),
            _ => None,
        }
    }

    /// Try to get this value as a shared managed reference.
    #[inline]
    pub fn as_shared_managed_reference(&self) -> Option<SharedManagedReference> {
        match self.tag() {
            ValueTag::SharedManagedReference => Some(SharedManagedReference(self.data)),
            _ => None,
        }
    }

    /// Try to get this value as a raw pointer.
    #[inline]
    pub fn as_raw_pointer(&self) -> Option<RawPointer> {
        if self.tag() == ValueTag::RawPointer {
            Some(RawPointer(self.data))
        } else {
            None
        }
    }

    /// Try to get this value as a shared raw pointer.
    #[inline]
    pub fn as_shared_raw_pointer(&self) -> Option<SharedRawPointer> {
        if self.tag() == ValueTag::SharedRawPointer {
            Some(SharedRawPointer(self.data))
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

    /// Try to get this value as a local pointer.
    #[inline]
    pub fn as_local_pointer(&self) -> Option<LocalPointer> {
        if self.tag() == ValueTag::LocalPointer {
            return Some(self.local_pointer_parts());
        }
        None
    }

    /// Try to get this value as a global pointer.
    #[inline]
    pub fn as_global_pointer(&self) -> Option<GlobalPointer> {
        if self.tag() == ValueTag::GlobalPointer {
            return Some(self.global_pointer_parts());
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
        let slot_offset = ((self.data >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;

        StackPointer {
            frame_idx,
            slot,
            slot_offset,
        }
    }

    /// Decode one local pointer from the packed value payload.
    #[inline]
    fn local_pointer_parts(&self) -> LocalPointer {
        let frame_idx = (self.data & STACK_INDEX_MASK) as usize;
        let local = ((self.data >> STACK_SLOT_SHIFT) & STACK_INDEX_MASK) as usize;
        let slot_offset = ((self.data >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;

        LocalPointer {
            frame_idx,
            local,
            slot_offset,
        }
    }

    /// Decode one global pointer from the packed value payload.
    #[inline]
    fn global_pointer_parts(&self) -> GlobalPointer {
        let base = (self.data & POINTER_BASE_MASK) as u32;
        let slot_offset = ((self.data >> POINTER_SLOT_SHIFT) & POINTER_BASE_MASK) as usize;
        let id = mir::LocalNodeId::new(base);

        GlobalPointer { id, slot_offset }
    }
}
