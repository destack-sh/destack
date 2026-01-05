use destack_mir as mir;

/// Type tag for packed values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ValueTag {
    /// No value.
    Void = 0,
    /// Boolean value.
    Bool = 1,
    /// Signed integer with width.
    Int = 2,
    /// Unsigned integer with width.
    UInt = 3,
    /// 32-bit float.
    Float32 = 4,
    /// 64-bit float.
    Float64 = 5,
    /// Unicode character.
    Char = 6,
    /// GC-tracked heap reference.
    ManagedReference = 7,
    /// Manually managed heap pointer.
    RawPointer = 8,
    /// Frame-scoped stack pointer.
    StackPointer = 9,
    /// Global variable pointer.
    GlobalPointer = 10,
    /// Function pointer.
    FunctionPointer = 11,
    /// Heap-allocated aggregate.
    Aggregate = 12,
    /// Heap-allocated string.
    String = 13,
}

/// A runtime value in the machine.
///
/// Compact 16-byte representation using a packed data/meta layout.
/// The data field stores the actual value, meta stores the tag and width.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Value {
    /// The value data (i64, u64, f64 bits, handle id, etc.).
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
                let c = char::from_u32(self.data as u32).unwrap_or('\0');
                write!(f, "Char({c:?})")
            }
            ValueTag::ManagedReference => write!(f, "ManagedReference(HeapHandle({}))", self.data),
            ValueTag::RawPointer => write!(f, "RawPointer({})", self.data),
            ValueTag::StackPointer => {
                let frame = self.data as u32;
                let slot = (self.data >> 32) as u32;
                write!(f, "StackPointer {{ frame: {frame}, slot: {slot} }}")
            }
            ValueTag::GlobalPointer => write!(f, "GlobalPointer({})", self.data as u32),
            ValueTag::FunctionPointer => write!(f, "FunctionPointer({})", self.data as u32),
            ValueTag::Aggregate => write!(f, "Aggregate(HeapHandle({}))", self.data),
            ValueTag::String => write!(f, "String(HeapHandle({}))", self.data),
        }
    }
}

impl From<&mir::Constant> for Value {
    fn from(constant: &mir::Constant) -> Self {
        match constant {
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
            // strings from constants need heap allocation
            mir::Constant::String { .. } => {
                // TODO #Incomplete: allocate string on heap
                Value::VOID
            }
            mir::Constant::Char { value } => Value::char(*value),
        }
    }
}

impl Value {
    /// Constant void value.
    pub const VOID: Self = Self { data: 0, meta: 0 };

    /// Create metadata from tag and width.
    #[inline(always)]
    const fn make_meta(tag: ValueTag, width: u8) -> u64 {
        (tag as u64) | ((width as u64) << 8)
    }

    /// Get the type tag.
    #[inline(always)]
    pub fn tag(&self) -> ValueTag {
        // SAFETY: we only construct valid tags
        unsafe { std::mem::transmute((self.meta & 0xFF) as u8) }
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

    /// Create a signed 32-bit integer value.
    #[inline]
    pub const fn int32(value: i32) -> Self {
        Self::int(value as i64, 32)
    }

    /// Create a signed 64-bit integer value.
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

    /// Create an unsigned 32-bit integer value.
    #[inline]
    pub const fn uint32(value: u32) -> Self {
        Self::uint(value as u64, 32)
    }

    /// Create an unsigned 64-bit integer value.
    #[inline]
    pub const fn uint64(value: u64) -> Self {
        Self::uint(value, 64)
    }

    /// Create a 32-bit float value.
    #[inline]
    pub const fn float32(value: f32) -> Self {
        Self {
            data: value.to_bits() as u64,
            meta: Self::make_meta(ValueTag::Float32, 32),
        }
    }

    /// Create a 64-bit float value.
    #[inline]
    pub const fn float64(value: f64) -> Self {
        Self {
            data: value.to_bits(),
            meta: Self::make_meta(ValueTag::Float64, 64),
        }
    }

    /// Create a character value.
    #[inline]
    pub const fn char(value: char) -> Self {
        Self {
            data: value as u64,
            meta: Self::make_meta(ValueTag::Char, 0),
        }
    }

    /// Create a managed reference value.
    #[inline]
    pub const fn managed_reference(handle: HeapHandle) -> Self {
        Self {
            data: handle.0,
            meta: Self::make_meta(ValueTag::ManagedReference, 0),
        }
    }

    /// Create a raw pointer value.
    #[inline]
    pub const fn raw_pointer(ptr: RawPointer) -> Self {
        Self {
            data: ptr.0,
            meta: Self::make_meta(ValueTag::RawPointer, 0),
        }
    }

    /// Create a stack pointer value.
    #[inline]
    pub const fn stack_pointer(ptr: StackPointer) -> Self {
        let data = (ptr.frame_idx as u64) | ((ptr.slot as u64) << 32);
        Self {
            data,
            meta: Self::make_meta(ValueTag::StackPointer, 0),
        }
    }

    /// Create a global pointer value.
    #[inline]
    pub fn global_pointer(id: mir::LocalNodeId<mir::Global>) -> Self {
        Self {
            data: id.id as u64,
            meta: Self::make_meta(ValueTag::GlobalPointer, 0),
        }
    }

    /// Create a function pointer value.
    #[inline]
    pub fn function_pointer(id: mir::LocalNodeId<mir::Function>) -> Self {
        Self {
            data: id.id as u64,
            meta: Self::make_meta(ValueTag::FunctionPointer, 0),
        }
    }

    /// Create an aggregate value.
    #[inline]
    pub const fn aggregate(handle: HeapHandle) -> Self {
        Self {
            data: handle.0,
            meta: Self::make_meta(ValueTag::Aggregate, 0),
        }
    }

    /// Create a string value.
    #[inline]
    pub const fn string(handle: HeapHandle) -> Self {
        Self {
            data: handle.0,
            meta: Self::make_meta(ValueTag::String, 0),
        }
    }

    /// Check if this value is truthy (for branch conditions).
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
            ValueTag::RawPointer => self.data != 0,
            ValueTag::StackPointer => true,
            ValueTag::GlobalPointer => true,
            ValueTag::FunctionPointer => true,
            ValueTag::Aggregate => self.data != 0,
            ValueTag::String => self.data != 0,
        }
    }

    /// Get this value as a boolean, if applicable.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        if self.tag() == ValueTag::Bool {
            Some(self.data != 0)
        } else {
            None
        }
    }

    /// Get this value as a signed integer, if applicable.
    #[inline]
    pub fn as_int(&self) -> Option<i64> {
        match self.tag() {
            ValueTag::Int => Some(self.data as i64),
            ValueTag::UInt => Some(self.data as i64),
            _ => None,
        }
    }

    /// Get this value as an unsigned integer, if applicable.
    #[inline]
    pub fn as_uint(&self) -> Option<u64> {
        match self.tag() {
            ValueTag::UInt => Some(self.data),
            ValueTag::Int => Some(self.data),
            _ => None,
        }
    }

    /// Get this value as a char, if applicable.
    #[inline]
    pub fn as_char(&self) -> Option<char> {
        if self.tag() == ValueTag::Char {
            char::from_u32(self.data as u32)
        } else {
            None
        }
    }

    /// Check if this is the void value.
    #[inline]
    pub fn is_void(&self) -> bool {
        self.tag() == ValueTag::Void
    }

    /// Check if this is a managed reference.
    #[inline]
    pub fn is_managed_reference(&self) -> bool {
        self.tag() == ValueTag::ManagedReference
    }

    /// Check if this is an aggregate.
    #[inline]
    pub fn is_aggregate(&self) -> bool {
        self.tag() == ValueTag::Aggregate
    }

    /// Get this value as a heap handle (for managed ref, aggregate, string).
    #[inline]
    pub fn as_heap_handle(&self) -> Option<HeapHandle> {
        match self.tag() {
            ValueTag::ManagedReference | ValueTag::Aggregate | ValueTag::String => {
                Some(HeapHandle(self.data))
            }
            _ => None,
        }
    }

    /// Get this value as a raw pointer.
    #[inline]
    pub fn as_raw_pointer(&self) -> Option<RawPointer> {
        if self.tag() == ValueTag::RawPointer {
            Some(RawPointer(self.data))
        } else {
            None
        }
    }

    /// Get this value as a stack pointer.
    #[inline]
    pub fn as_stack_pointer(&self) -> Option<StackPointer> {
        if self.tag() == ValueTag::StackPointer {
            Some(StackPointer {
                frame_idx: self.data as u32 as usize,
                slot: (self.data >> 32) as u32 as usize,
            })
        } else {
            None
        }
    }

    /// Get this value as a global pointer.
    #[inline]
    pub fn as_global_pointer(&self) -> Option<mir::LocalNodeId<mir::Global>> {
        if self.tag() == ValueTag::GlobalPointer {
            Some(mir::LocalNodeId::new(self.data as u32))
        } else {
            None
        }
    }

    /// Get this value as a function pointer.
    #[inline]
    pub fn as_function_pointer(&self) -> Option<mir::LocalNodeId<mir::Function>> {
        if self.tag() == ValueTag::FunctionPointer {
            Some(mir::LocalNodeId::new(self.data as u32))
        } else {
            None
        }
    }

    /// Get signed int value and width.
    #[inline]
    pub fn as_int_with_width(&self) -> Option<(i64, u8)> {
        if self.tag() == ValueTag::Int {
            Some((self.data as i64, self.width()))
        } else {
            None
        }
    }

    /// Get unsigned int value and width.
    #[inline]
    pub fn as_uint_with_width(&self) -> Option<(u64, u8)> {
        if self.tag() == ValueTag::UInt {
            Some((self.data, self.width()))
        } else {
            None
        }
    }

    /// Get float64 value.
    #[inline]
    pub fn as_float64(&self) -> Option<f64> {
        if self.tag() == ValueTag::Float64 {
            Some(f64::from_bits(self.data))
        } else {
            None
        }
    }

    /// Get float32 value.
    #[inline]
    pub fn as_float32(&self) -> Option<f32> {
        if self.tag() == ValueTag::Float32 {
            Some(f32::from_bits(self.data as u32))
        } else {
            None
        }
    }

    /// Get raw data (for internal use).
    #[inline]
    pub fn raw_data(&self) -> u64 {
        self.data
    }
}

/// Handle to a managed (GC-tracked) heap object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapHandle(pub(crate) u64);

impl HeapHandle {
    /// The null handle (represents null reference).
    pub const NULL: Self = HeapHandle(0);

    /// Create a new heap handle from a raw id.
    #[inline]
    pub fn new(id: u64) -> Self {
        HeapHandle(id)
    }

    /// Check if this handle is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Get the raw id of this handle.
    #[inline]
    pub fn id(&self) -> u64 {
        self.0
    }
}

/// Pointer to a raw (manually managed) heap object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawPointer(pub(crate) u64);

impl RawPointer {
    /// The null pointer.
    pub const NULL: Self = RawPointer(0);

    /// Create a new raw pointer from an id.
    #[inline]
    pub fn new(id: u64) -> Self {
        RawPointer(id)
    }

    /// Check if this pointer is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Get the raw id of this pointer.
    #[inline]
    pub fn id(&self) -> u64 {
        self.0
    }
}

/// Pointer to a stack-allocated object (frame-scoped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StackPointer {
    /// The frame depth (index into call stack).
    pub frame_idx: usize,
    /// The slot index within the frame's stack allocations.
    pub slot: usize,
}

impl StackPointer {
    /// Create a new stack pointer.
    #[inline]
    pub fn new(frame_idx: usize, slot: usize) -> Self {
        Self { frame_idx, slot }
    }
}
