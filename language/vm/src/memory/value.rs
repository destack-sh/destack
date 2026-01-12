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

/// Address space class for reference metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceAddressSpace {
    /// Target default address space.
    Generic,
    /// Stack or function-local memory.
    Stack,
    /// Global or module-static memory.
    Global,
    /// Heap-allocated memory.
    Heap,
    /// Constant or read-only memory.
    Constant,
    /// Shared or workgroup memory.
    Shared,
    /// Local or thread-local memory.
    Local,
    /// Target-specific custom address space.
    Target,
}

impl ReferenceAddressSpace {
    /// Map a MIR address space into a VM reference address space.
    pub fn from_mir(address_space: mir::AddressSpace) -> Self {
        match address_space {
            mir::AddressSpace::Generic => ReferenceAddressSpace::Generic,
            mir::AddressSpace::Stack => ReferenceAddressSpace::Stack,
            mir::AddressSpace::Global => ReferenceAddressSpace::Global,
            mir::AddressSpace::Heap => ReferenceAddressSpace::Heap,
            mir::AddressSpace::Shared => ReferenceAddressSpace::Shared,
            mir::AddressSpace::Local => ReferenceAddressSpace::Local,
            mir::AddressSpace::Constant => ReferenceAddressSpace::Constant,
            mir::AddressSpace::Target(_) => ReferenceAddressSpace::Target,
        }
    }

    /// Decode a reference address space from packed bits.
    pub fn from_bits(bits: u8) -> Self {
        match bits {
            0 => ReferenceAddressSpace::Generic,
            1 => ReferenceAddressSpace::Stack,
            2 => ReferenceAddressSpace::Global,
            3 => ReferenceAddressSpace::Heap,
            4 => ReferenceAddressSpace::Constant,
            5 => ReferenceAddressSpace::Shared,
            6 => ReferenceAddressSpace::Local,
            _ => ReferenceAddressSpace::Target,
        }
    }

    /// Encode a reference address space as packed bits.
    pub fn to_bits(self) -> u8 {
        match self {
            ReferenceAddressSpace::Generic => 0,
            ReferenceAddressSpace::Stack => 1,
            ReferenceAddressSpace::Global => 2,
            ReferenceAddressSpace::Heap => 3,
            ReferenceAddressSpace::Constant => 4,
            ReferenceAddressSpace::Shared => 5,
            ReferenceAddressSpace::Local => 6,
            ReferenceAddressSpace::Target => 7,
        }
    }

    /// Check whether this address space is supported by the VM.
    pub fn is_supported_by_vm(self) -> bool {
        matches!(
            self,
            ReferenceAddressSpace::Generic
                | ReferenceAddressSpace::Stack
                | ReferenceAddressSpace::Global
                | ReferenceAddressSpace::Heap
                | ReferenceAddressSpace::Constant
        )
    }

    /// Return a human-readable label for diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            ReferenceAddressSpace::Generic => "generic",
            ReferenceAddressSpace::Stack => "stack",
            ReferenceAddressSpace::Global => "global",
            ReferenceAddressSpace::Heap => "heap",
            ReferenceAddressSpace::Constant => "constant",
            ReferenceAddressSpace::Shared => "shared",
            ReferenceAddressSpace::Local => "local",
            ReferenceAddressSpace::Target => "target",
        }
    }
}

/// Metadata for reference values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceMeta {
    bits: u8,
}

const REF_KIND_MASK: u8 = 0x7;
const REF_MUTABLE_BIT: u8 = 1 << 3;
const REF_NULLABLE_BIT: u8 = 1 << 4;
const REF_ADDRESS_SPACE_SHIFT: u8 = 5;
const REF_ADDRESS_SPACE_MASK: u8 = 0x7 << REF_ADDRESS_SPACE_SHIFT;

impl ReferenceMeta {
    /// Empty reference metadata.
    pub const NONE: Self = Self { bits: 0 };

    /// Create reference metadata.
    pub fn new(
        kind: mir::ReferenceKind,
        address_space: mir::AddressSpace,
        mutability: mir::Mutability,
        is_nullable: bool,
    ) -> Self {
        let kind_bits = match kind {
            mir::ReferenceKind::Managed => 1,
            mir::ReferenceKind::Owned => 2,
            mir::ReferenceKind::Borrowed => 3,
            mir::ReferenceKind::Raw => 4,
        };
        let address_space_bits = ReferenceAddressSpace::from_mir(address_space).to_bits();

        let mut bits = kind_bits | (address_space_bits << REF_ADDRESS_SPACE_SHIFT);
        if matches!(mutability, mir::Mutability::Mutable) {
            bits |= REF_MUTABLE_BIT;
        }
        if is_nullable {
            bits |= REF_NULLABLE_BIT;
        }

        Self { bits }
    }

    /// Get the reference kind when available.
    pub fn kind(self) -> Option<mir::ReferenceKind> {
        match self.bits & REF_KIND_MASK {
            0 => None,
            1 => Some(mir::ReferenceKind::Managed),
            2 => Some(mir::ReferenceKind::Owned),
            3 => Some(mir::ReferenceKind::Borrowed),
            4 => Some(mir::ReferenceKind::Raw),
            _ => None,
        }
    }

    /// Get the reference mutability when available.
    pub fn mutability(self) -> Option<mir::Mutability> {
        self.kind()?;

        if self.bits & REF_MUTABLE_BIT != 0 {
            Some(mir::Mutability::Mutable)
        } else {
            Some(mir::Mutability::Immutable)
        }
    }

    /// Check whether this reference is nullable.
    pub fn is_nullable(self) -> bool {
        self.bits & REF_NULLABLE_BIT != 0
    }

    /// Get the reference address space.
    pub fn address_space(self) -> ReferenceAddressSpace {
        let bits = (self.bits & REF_ADDRESS_SPACE_MASK) >> REF_ADDRESS_SPACE_SHIFT;
        ReferenceAddressSpace::from_bits(bits)
    }

    /// Return the raw metadata bits.
    pub fn bits(self) -> u8 {
        self.bits
    }
}

const POINTER_BASE_MASK: u64 = 0xFFFF_FFFF;
const POINTER_SLOT_SHIFT: u64 = 32;
const STACK_INDEX_MASK: u64 = 0xFFFF;
const STACK_SLOT_SHIFT: u64 = 16;
const REF_META_SHIFT: u64 = 16;
const REF_META_MASK: u64 = 0xFF << REF_META_SHIFT;

/// A runtime value in the VM.
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
            ValueTag::ManagedReference => {
                let handle = self.as_heap_handle().unwrap();
                let slot_offset = handle.slot_index();
                if slot_offset == 0 {
                    write!(f, "ManagedReference(HeapHandle({}))", handle.id())
                } else {
                    write!(
                        f,
                        "ManagedReference(HeapHandle({}), slot {})",
                        handle.id(),
                        slot_offset
                    )
                }
            }
            ValueTag::RawPointer => {
                let pointer = self.as_raw_pointer().unwrap();
                let slot_offset = pointer.slot_index();
                if slot_offset == 0 {
                    write!(f, "RawPointer({})", pointer.id())
                } else {
                    write!(f, "RawPointer({}, slot {})", pointer.id(), slot_offset)
                }
            }
            ValueTag::StackPointer => {
                let pointer = self.as_stack_pointer().unwrap();
                if pointer.slot_offset == 0 {
                    write!(
                        f,
                        "StackPointer {{ frame: {}, slot: {} }}",
                        pointer.frame_idx, pointer.slot
                    )
                } else {
                    write!(
                        f,
                        "StackPointer {{ frame: {}, slot: {}, offset: {} }}",
                        pointer.frame_idx, pointer.slot, pointer.slot_offset
                    )
                }
            }
            ValueTag::GlobalPointer => {
                let pointer = self.as_global_pointer().unwrap();
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
            mir::Constant::String { .. } => {
                panic!("string constants must be allocated by the interpreter");
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
        ReferenceMeta { bits }
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

    /// Create a managed reference value with metadata.
    #[inline]
    pub fn managed_reference_with_meta(handle: HeapHandle, meta: ReferenceMeta) -> Self {
        Self::managed_reference(handle).with_reference_meta(meta)
    }

    /// Create a raw pointer value.
    #[inline]
    pub const fn raw_pointer(ptr: RawPointer) -> Self {
        Self {
            data: ptr.0,
            meta: Self::make_meta(ValueTag::RawPointer, 0),
        }
    }

    /// Create a raw pointer value with metadata.
    #[inline]
    pub fn raw_pointer_with_meta(ptr: RawPointer, meta: ReferenceMeta) -> Self {
        Self::raw_pointer(ptr).with_reference_meta(meta)
    }

    /// Create a stack pointer value.
    #[inline]
    pub const fn stack_pointer(ptr: StackPointer) -> Self {
        let frame = (ptr.frame_idx as u64) & STACK_INDEX_MASK;
        let slot = (ptr.slot as u64) & STACK_INDEX_MASK;
        let offset = (ptr.slot_offset as u64) & POINTER_BASE_MASK;
        let data = frame | (slot << STACK_SLOT_SHIFT) | (offset << POINTER_SLOT_SHIFT);
        Self {
            data,
            meta: Self::make_meta(ValueTag::StackPointer, 0),
        }
    }

    /// Create a stack pointer value with metadata.
    #[inline]
    pub fn stack_pointer_with_meta(ptr: StackPointer, meta: ReferenceMeta) -> Self {
        Self::stack_pointer(ptr).with_reference_meta(meta)
    }

    /// Create a global pointer value.
    #[inline]
    pub fn global_pointer(id: mir::LocalNodeId<mir::Global>) -> Self {
        Self::global_pointer_with_offset(id, 0)
    }

    /// Create a global pointer value with a slot offset.
    #[inline]
    pub fn global_pointer_with_offset(
        id: mir::LocalNodeId<mir::Global>,
        slot_offset: usize,
    ) -> Self {
        Self {
            data: (id.id as u64) | ((slot_offset as u64) << POINTER_SLOT_SHIFT),
            meta: Self::make_meta(ValueTag::GlobalPointer, 0),
        }
    }

    /// Create a global pointer value with metadata.
    #[inline]
    pub fn global_pointer_with_meta(
        id: mir::LocalNodeId<mir::Global>,
        slot_offset: usize,
        meta: ReferenceMeta,
    ) -> Self {
        Self::global_pointer_with_offset(id, slot_offset).with_reference_meta(meta)
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
            let frame_idx = (self.data & STACK_INDEX_MASK) as usize;
            let slot = ((self.data >> STACK_SLOT_SHIFT) & STACK_INDEX_MASK) as usize;
            let slot_offset = (self.data >> POINTER_SLOT_SHIFT) as usize;
            Some(StackPointer {
                frame_idx,
                slot,
                slot_offset,
            })
        } else {
            None
        }
    }

    /// Get this value as a global pointer.
    #[inline]
    pub fn as_global_pointer(&self) -> Option<GlobalPointer> {
        if self.tag() == ValueTag::GlobalPointer {
            let id = mir::LocalNodeId::new(self.data as u32);
            let slot_offset = (self.data >> POINTER_SLOT_SHIFT) as usize;
            Some(GlobalPointer { id, slot_offset })
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
        HeapHandle::with_slot(id, 0)
    }

    /// Check if this handle is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.id() == 0
    }

    /// Get the raw id of this handle.
    #[inline]
    pub fn id(&self) -> u64 {
        self.0 & POINTER_BASE_MASK
    }

    /// Get the slot offset stored in this handle.
    #[inline]
    pub fn slot_index(&self) -> usize {
        (self.0 >> POINTER_SLOT_SHIFT) as usize
    }

    /// Create a new handle with a slot offset.
    #[inline]
    pub fn with_slot(id: u64, slot_index: u32) -> Self {
        let base = id & POINTER_BASE_MASK;
        let slot = (slot_index as u64) << POINTER_SLOT_SHIFT;
        HeapHandle(base | slot)
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
        RawPointer::with_slot(id, 0)
    }

    /// Check if this pointer is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.id() == 0
    }

    /// Get the base id of this pointer.
    #[inline]
    pub fn id(&self) -> u64 {
        self.0 & POINTER_BASE_MASK
    }

    /// Get the slot offset stored in this pointer.
    #[inline]
    pub fn slot_index(&self) -> usize {
        (self.0 >> POINTER_SLOT_SHIFT) as usize
    }

    /// Create a new raw pointer with a slot offset.
    #[inline]
    pub fn with_slot(id: u64, slot_index: u32) -> Self {
        let base = id & POINTER_BASE_MASK;
        let slot = (slot_index as u64) << POINTER_SLOT_SHIFT;
        RawPointer(base | slot)
    }
}

/// Pointer to a stack-allocated object (frame-scoped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StackPointer {
    /// The frame depth (index into call stack).
    pub frame_idx: usize,
    /// The slot index within the frame's stack allocations.
    pub slot: usize,
    /// The slot offset within the stack allocation.
    pub slot_offset: usize,
}

impl StackPointer {
    /// Create a new stack pointer.
    #[inline]
    pub fn new(frame_idx: usize, slot: usize) -> Self {
        Self {
            frame_idx,
            slot,
            slot_offset: 0,
        }
    }

    /// Create a stack pointer with an offset into the slot.
    #[inline]
    pub fn with_offset(frame_idx: usize, slot: usize, slot_offset: usize) -> Self {
        Self {
            frame_idx,
            slot,
            slot_offset,
        }
    }
}

/// Pointer to a global value (with optional slot offset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalPointer {
    /// The global identifier.
    pub id: mir::LocalNodeId<mir::Global>,
    /// The slot offset within the global value.
    pub slot_offset: usize,
}
