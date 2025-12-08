use destack_mir as mir;

/// A runtime value in the machine.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    /// No value (void/unit).
    #[default]
    Void,

    /// Boolean value.
    Boolean(bool),

    /// Signed integer (up to 64-bit).
    Int { value: i64, width: u16 },

    /// Unsigned integer (up to 64-bit).
    UInt { value: u64, width: u16 },

    /// 32-bit floating point.
    Float32(f32),

    /// 64-bit floating point.
    Float64(f64),

    /// Raw pointer (as integer address).
    RawPointer(u64),

    /// Managed reference (heap object handle).
    ManagedReference(HeapHandle),

    /// Aggregate value (struct, tuple, array).
    Aggregate(Vec<Value>),
}

impl From<&mir::Constant> for Value {
    fn from(constant: &mir::Constant) -> Self {
        match constant {
            mir::Constant::Boolean { value } => Value::Boolean(*value),
            mir::Constant::Int {
                value,
                width,
                is_signed: true,
            } => Value::Int {
                value: *value,
                width: *width as u16,
            },
            mir::Constant::Int {
                value,
                width,
                is_signed: false,
            } => Value::UInt {
                value: *value as u64,
                width: *width as u16,
            },
            mir::Constant::UInt { value, width } => Value::UInt {
                value: *value,
                width: *width as u16,
            },
            mir::Constant::Float { bits, width: 32 } => {
                Value::Float32(f32::from_bits(*bits as u32))
            }
            mir::Constant::Float { bits, width: _ } => Value::Float64(f64::from_bits(*bits)),
        }
    }
}

impl Value {
    /// Check if this value is truthy (for branch conditions).
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Void => false,
            Value::Boolean(b) => *b,
            Value::Int { value, .. } => *value != 0,
            Value::UInt { value, .. } => *value != 0,
            Value::Float32(f) => *f != 0.0,
            Value::Float64(f) => *f != 0.0,
            Value::RawPointer(p) => *p != 0,
            Value::ManagedReference(h) => !h.is_null(),
            Value::Aggregate(_) => true,
        }
    }

    /// Get this value as a boolean, if applicable.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Get this value as a signed integer, if applicable.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int { value, .. } => Some(*value),
            Value::UInt { value, .. } => Some(*value as i64),
            _ => None,
        }
    }

    /// Get this value as an unsigned integer, if applicable.
    pub fn as_uint(&self) -> Option<u64> {
        match self {
            Value::UInt { value, .. } => Some(*value),
            Value::Int { value, .. } => Some(*value as u64),
            _ => None,
        }
    }
}

/// Handle to a heap-allocated object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapHandle(u64);

impl HeapHandle {
    /// The null handle (represents null reference).
    pub const NULL: Self = HeapHandle(0);

    /// Create a new heap handle from a raw id.
    pub fn new(id: u64) -> Self {
        HeapHandle(id)
    }

    /// Check if this handle is null.
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Get the raw id of this handle.
    pub fn id(&self) -> u64 {
        self.0
    }
}

