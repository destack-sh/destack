use destack_mir as mir;
use serde::{Deserialize, Serialize};

use destack_engine::{StaticPointer, Value};

use crate::{FramePointer, FunctionPointer, HeapReference, SharedHeapReference, StackPointer};

/// Truncate one unsigned integer to a bit width.
const fn truncate_unsigned_bits(value: u64, width: u8) -> u64 {
    if width >= u64::BITS as u8 {
        return value;
    }

    value & ((1u64 << width) - 1)
}

/// Truncate one signed integer to a bit width.
const fn truncate_signed_bits(value: i64, width: u8) -> i64 {
    if width >= u64::BITS as u8 {
        return value;
    }

    let mask = (1u64 << width) - 1;
    let masked = (value as u64) & mask;
    let sign_bit = 1u64 << (width - 1);
    if masked & sign_bit != 0 {
        (masked | !mask) as i64
    } else {
        masked as i64
    }
}

/// One untyped VM word.
///
/// The type is supplied by MIR metadata, frame maps, and lowered instructions.
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Word(u64);

impl std::fmt::Debug for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Word(0x{:016X})", self.0)
    }
}

impl Word {
    /// The zero word.
    pub const VOID: Self = Self(0);
    /// The bit width of one VM word.
    pub const BIT_LEN: u8 = u64::BITS as u8;
    /// The byte width of one VM word.
    pub const BYTE_LEN: usize = std::mem::size_of::<Self>();

    /// Create one word from raw bits.
    #[inline(always)]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return this word as raw bits.
    #[inline(always)]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// View this value as a boolean.
    #[inline(always)]
    pub const fn as_bool(self) -> bool {
        self.0 != 0
    }

    /// View this value as a signed integer.
    #[inline(always)]
    pub const fn as_int(self) -> i64 {
        self.as_i64()
    }

    /// View this value as a signed integer.
    #[inline(always)]
    pub const fn as_i64(self) -> i64 {
        self.0 as i64
    }

    /// View this value as an unsigned integer.
    #[inline(always)]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// View this value as an unsigned integer.
    #[inline(always)]
    pub const fn as_uint(self) -> u64 {
        self.as_u64()
    }

    /// View this value as a float32.
    #[inline(always)]
    pub const fn as_f32(self) -> f32 {
        f32::from_bits(self.0 as u32)
    }

    /// View this value as a float32.
    #[inline(always)]
    pub const fn as_float32(self) -> f32 {
        self.as_f32()
    }

    /// View this value as a float64.
    #[inline(always)]
    pub const fn as_f64(self) -> f64 {
        f64::from_bits(self.0)
    }

    /// View this value as a float64.
    #[inline(always)]
    pub const fn as_float64(self) -> f64 {
        self.as_f64()
    }

    /// View this value as a character.
    #[inline(always)]
    pub fn as_char(self) -> Option<char> {
        char::from_u32(self.0 as u32)
    }

    /// View this value as a heap reference.
    #[inline(always)]
    pub const fn as_heap_reference(self) -> HeapReference {
        HeapReference::from_bits(self.0 as usize)
    }

    /// View this value as a shared heap reference.
    #[inline(always)]
    pub const fn as_shared_heap_reference(self) -> SharedHeapReference {
        SharedHeapReference::from_bits(self.0 as usize)
    }

    /// View this value as a native address.
    #[inline(always)]
    pub const fn as_address(self) -> usize {
        self.0 as usize
    }

    /// View this value as a stack pointer.
    #[inline(always)]
    pub const fn as_stack_pointer(self) -> StackPointer {
        StackPointer::from_address(self.0 as usize)
    }

    /// View this value as a frame pointer.
    #[inline(always)]
    pub const fn as_frame_pointer(self) -> FramePointer {
        FramePointer::from_address(self.0 as usize)
    }

    /// View this value as a static pointer.
    #[inline(always)]
    pub const fn as_static_pointer(self) -> StaticPointer {
        StaticPointer::from_bits(self.0 as usize)
    }

    /// View this value as a function pointer.
    #[inline(always)]
    pub const fn as_function_pointer(self) -> FunctionPointer {
        FunctionPointer::from_bits(self.0 as usize)
    }

    /// Create a boolean value.
    #[inline(always)]
    pub const fn bool(value: bool) -> Self {
        Self(value as u64)
    }

    /// Create a signed integer value.
    #[inline(always)]
    pub const fn int(value: i64, width: u8) -> Self {
        Self(truncate_signed_bits(value, width) as u64)
    }

    /// Create an int8 value.
    #[inline(always)]
    pub const fn int8(value: i8) -> Self {
        Self::int(value as i64, 8)
    }

    /// Create an int16 value.
    #[inline(always)]
    pub const fn int16(value: i16) -> Self {
        Self::int(value as i64, 16)
    }

    /// Create an int32 value.
    #[inline(always)]
    pub const fn int32(value: i32) -> Self {
        Self::int(value as i64, 32)
    }

    /// Create an int64 value.
    #[inline(always)]
    pub const fn int64(value: i64) -> Self {
        Self::int(value, 64)
    }

    /// Create an unsigned integer value.
    #[inline(always)]
    pub const fn uint(value: u64, width: u8) -> Self {
        Self(truncate_unsigned_bits(value, width))
    }

    /// Create a uint8 value.
    #[inline(always)]
    pub const fn uint8(value: u8) -> Self {
        Self::uint(value as u64, 8)
    }

    /// Create a uint16 value.
    #[inline(always)]
    pub const fn uint16(value: u16) -> Self {
        Self::uint(value as u64, 16)
    }

    /// Create a uint32 value.
    #[inline(always)]
    pub const fn uint32(value: u32) -> Self {
        Self::uint(value as u64, 32)
    }

    /// Create a uint64 value.
    #[inline(always)]
    pub const fn uint64(value: u64) -> Self {
        Self::uint(value, 64)
    }

    /// Create a float32 value.
    #[inline(always)]
    pub const fn float32(value: f32) -> Self {
        Self(value.to_bits() as u64)
    }

    /// Create a float64 value.
    #[inline(always)]
    pub const fn float64(value: f64) -> Self {
        Self(value.to_bits())
    }

    /// Create a char value.
    #[inline(always)]
    pub const fn char(value: char) -> Self {
        Self(value as u64)
    }

    /// Create a heap reference value.
    #[inline(always)]
    pub const fn heap_reference(reference: HeapReference) -> Self {
        Self(reference.bits() as u64)
    }

    /// Create a shared heap reference value.
    #[inline(always)]
    pub const fn shared_heap_reference(reference: SharedHeapReference) -> Self {
        Self(reference.bits() as u64)
    }

    /// Create a native address value.
    #[inline(always)]
    pub const fn address(address: usize) -> Self {
        Self(address as u64)
    }

    /// Create a stack pointer value.
    #[inline(always)]
    pub const fn stack_pointer(ptr: StackPointer) -> Self {
        Self(ptr.bits() as u64)
    }

    /// Create a frame pointer value.
    #[inline(always)]
    pub const fn frame_pointer(ptr: FramePointer) -> Self {
        Self(ptr.bits() as u64)
    }

    /// Create a static pointer value.
    #[inline(always)]
    pub const fn static_pointer(ptr: StaticPointer) -> Self {
        Self(ptr.bits() as u64)
    }

    /// Create a function pointer value.
    #[inline(always)]
    pub const fn function_pointer(pointer: FunctionPointer) -> Self {
        Self(pointer.bits() as u64)
    }

    /// Return the packed bytes for this value.
    #[inline(always)]
    pub const fn to_byte_array(self) -> [u8; Self::BYTE_LEN] {
        self.0.to_le_bytes()
    }

    /// Restore one word from one packed byte slice.
    #[inline]
    pub fn from_byte_slice(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; Self::BYTE_LEN] = bytes.try_into().ok()?;

        Some(Self(u64::from_le_bytes(bytes)))
    }
}

impl From<&mir::Constant> for Word {
    fn from(constant: &mir::Constant) -> Self {
        match constant {
            mir::Constant::Null => Word::VOID,
            mir::Constant::Boolean { value } => Word::bool(*value),
            mir::Constant::Int {
                value,
                width,
                is_signed: true,
            } => Word::int(*value as i64, *width as u8),
            mir::Constant::Int {
                value,
                width,
                is_signed: false,
            } => Word::uint(*value as u64, *width as u8),
            mir::Constant::UInt { value, width } => Word::uint(*value as u64, *width as u8),
            mir::Constant::Float { bits, format } => match format {
                mir::FloatType::Float16 | mir::FloatType::Bfloat16 => Word::from_bits(*bits),
                mir::FloatType::Float32 => Word::float32(f32::from_bits(*bits as u32)),
                mir::FloatType::Float64 => Word::float64(f64::from_bits(*bits)),
            },
            mir::Constant::Char { value } => Word::char(*value),
        }
    }
}

impl From<&Value> for Word {
    fn from(value: &Value) -> Self {
        match value {
            Value::Void => Word::VOID,
            Value::Bool(value) => Word::bool(*value),
            Value::Int { value, width } => Word::int(*value as i64, *width as u8),
            Value::UInt { value, width } => Word::uint(*value as u64, *width as u8),
            Value::Float16 { bits } => Word::from_bits(u64::from(*bits)),
            Value::Bfloat16 { bits } => Word::from_bits(u64::from(*bits)),
            Value::Float32 { bits } => Word::float32(f32::from_bits(*bits)),
            Value::Float64 { bits } => Word::float64(f64::from_bits(*bits)),
            Value::Char(value) => Word::char(*value),
            Value::HeapReference(reference) => Word::heap_reference(*reference),
            Value::SharedHeapReference(reference) => Word::shared_heap_reference(*reference),
            Value::Address(address) => Word::address(*address),
        }
    }
}
