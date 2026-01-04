use destack_mir as mir;

/// A runtime value in the machine.
///
/// Compact Copy-able representation. Strings and aggregates use heap handles
/// instead of Box to enable Copy semantics.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Value {
    /// No value (void/unit).
    #[default]
    Void,

    /// Boolean value.
    Bool(bool),

    /// Signed integer (up to 64-bit).
    Int { value: i64, width: u8 },

    /// Unsigned integer (up to 64-bit).
    UInt { value: u64, width: u8 },

    /// 32-bit floating point.
    Float32(f32),

    /// 64-bit floating point.
    Float64(f64),

    /// Character value (Unicode codepoint).
    Char(char),

    /// Managed reference (GC-tracked heap object).
    ManagedReference(HeapHandle),

    /// Raw pointer (manually managed heap object).
    RawPointer(RawPointer),

    /// Stack pointer (frame-scoped allocation).
    StackPointer(StackPointer),

    /// Global pointer (pointer to a global variable).
    GlobalPointer(mir::LocalNodeId<mir::Global>),

    /// Function pointer (for indirect calls).
    FunctionPointer(mir::LocalNodeId<mir::Function>),

    /// Heap-allocated aggregate (struct, tuple, array).
    Aggregate(HeapHandle),

    /// Heap-allocated string.
    String(HeapHandle),
}

impl From<&mir::Constant> for Value {
    fn from(constant: &mir::Constant) -> Self {
        match constant {
            mir::Constant::Boolean { value } => Value::Bool(*value),
            mir::Constant::Int {
                value,
                width,
                is_signed: true,
            } => Value::Int {
                value: *value,
                width: *width,
            },
            mir::Constant::Int {
                value,
                width,
                is_signed: false,
            } => Value::UInt {
                value: *value as u64,
                width: *width,
            },
            mir::Constant::UInt { value, width } => Value::UInt {
                value: *value,
                width: *width,
            },
            mir::Constant::Float { bits, width: 32 } => {
                Value::Float32(f32::from_bits(*bits as u32))
            }
            mir::Constant::Float { bits, width: _ } => Value::Float64(f64::from_bits(*bits)),
            // strings from constants need heap allocation
            mir::Constant::String { .. } => {
                // TODO: allocate string on heap
                Value::Void
            }
            mir::Constant::Char { value } => Value::Char(*value),
        }
    }
}

impl Value {
    /// Create a signed 32-bit integer value.
    #[inline]
    pub fn int32(value: i32) -> Self {
        Self::Int {
            value: value as i64,
            width: 32,
        }
    }

    /// Create a signed 64-bit integer value.
    #[inline]
    pub fn int64(value: i64) -> Self {
        Self::Int { value, width: 64 }
    }

    /// Create an unsigned 32-bit integer value.
    #[inline]
    pub fn uint32(value: u32) -> Self {
        Self::UInt {
            value: value as u64,
            width: 32,
        }
    }

    /// Create an unsigned 64-bit integer value.
    #[inline]
    pub fn uint64(value: u64) -> Self {
        Self::UInt { value, width: 64 }
    }

    /// Create a 64-bit float value.
    #[inline]
    pub fn float64(value: f64) -> Self {
        Self::Float64(value)
    }

    /// Create a 32-bit float value.
    #[inline]
    pub fn float32(value: f32) -> Self {
        Self::Float32(value)
    }

    /// Check if this value is truthy (for branch conditions).
    #[inline]
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Void => false,
            Value::Bool(b) => *b,
            Value::Int { value, .. } => *value != 0,
            Value::UInt { value, .. } => *value != 0,
            Value::Float32(f) => *f != 0.0,
            Value::Float64(f) => *f != 0.0,
            Value::Char(_) => true,
            Value::ManagedReference(h) => !h.is_null(),
            Value::RawPointer(p) => !p.is_null(),
            Value::StackPointer(_) => true,
            Value::GlobalPointer(_) => true,
            Value::FunctionPointer(_) => true,
            Value::Aggregate(h) => !h.is_null(),
            Value::String(h) => !h.is_null(),
        }
    }

    /// Get this value as a boolean, if applicable.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get this value as a signed integer, if applicable.
    #[inline]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int { value, .. } => Some(*value),
            Value::UInt { value, .. } => Some(*value as i64),
            _ => None,
        }
    }

    /// Get this value as an unsigned integer, if applicable.
    #[inline]
    pub fn as_uint(&self) -> Option<u64> {
        match self {
            Value::UInt { value, .. } => Some(*value),
            Value::Int { value, .. } => Some(*value as u64),
            _ => None,
        }
    }

    /// Get this value as a char, if applicable.
    #[inline]
    pub fn as_char(&self) -> Option<char> {
        match self {
            Value::Char(c) => Some(*c),
            _ => None,
        }
    }
}

/// Handle to a managed (GC-tracked) heap object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapHandle(u64);

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
pub struct RawPointer(u64);

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
