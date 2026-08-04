use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Address, AtomicOperation, BooleanOperation, CastOperation, Comparison, FloatOperation,
    InstructionLayout, IntegerOperation, MemoryOperation, New, NewKind, Operand, Prefetch, Scalar,
    ScalarCheck, TensorOperation, Transfer, ValueType, VectorOperation,
};

/// One stable exact bytecode operation code.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Opcode(u16);

/// One reserved parameterized opcode range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OpcodeRange {
    /// The first reserved opcode.
    start: u16,
    /// The first opcode after this range.
    end: u16,
}

impl OpcodeRange {
    /// Create one half-open opcode range.
    const fn new(start: u16, end: u16) -> Self {
        Self { start, end }
    }

    /// Return the first reserved opcode.
    const fn start(self) -> u16 {
        self.start
    }

    /// Return whether this range reserves one exact opcode.
    const fn contains(self, opcode: u16) -> bool {
        opcode >= self.start && opcode < self.end
    }
}

macro_rules! opcodes {
    (
        $(
            $(#[$opcode_meta:meta])*
            $opcode:ident = $code:literal {
                text: $text:literal,
                signature: $signature:literal,
                operands: [$($operand:ident),* $(,)?],
            }
        )*
        ;
        $(
            $(#[$range_meta:meta])*
            $range:ident in $start:literal..$end:literal {
                layout: $layout:ident,
            }
        )*
    ) => {
        impl OpcodeRange {
            $(
                $(#[$range_meta])*
                const $range: Self = Self::new($start, $end);
            )*
        }

        impl Opcode {
            $(
                $(#[$opcode_meta])*
                #[doc = concat!("`", $text, "`")]
                #[doc = ""]
                #[doc = concat!("`", $signature, "`")]
                pub const $opcode: Self = Self($code);
            )*

            /// Return this directly named opcode's canonical text name.
            pub const fn name(self) -> Option<&'static str> {
                match self {
                    $(Self::$opcode => Some($text),)*
                    _ => None,
                }
            }

            /// Return this directly named opcode's exact operand layout.
            fn named_layout(self) -> Option<InstructionLayout> {
                let operands = match self {
                    $(Self::$opcode => &[$(Operand::$operand),*][..],)*
                    _ => return None,
                };

                Some(InstructionLayout::new(operands))
            }

            /// Return this parameterized opcode's exact operand layout.
            fn range_layout(self) -> Option<InstructionLayout> {
                let code = self.code();
                $(
                    if OpcodeRange::$range.contains(code) {
                        return Self::$layout(code);
                    }
                )*

                None
            }
        }
    };
}

opcodes! {
    // values
    MOVE = 0x0010 {
        text: "move",
        signature: "(source: word) => word",
        operands: [Result, Register],
    }
    MOVE_RANGE = 0x0011 {
        text: "move",
        signature: "(source: value) => value",
        operands: [ResultRange, RegisterSpan],
    }
    SELECT = 0x0012 {
        text: "select",
        signature: "(condition: boolean, then: word, else: word) => word",
        operands: [Result, Register, Register, Register],
    }
    SELECT_RANGE = 0x0013 {
        text: "select",
        signature: "(condition: boolean, then: value, else: value) => value",
        operands: [ResultRange, Register, RegisterSpan, RegisterSpan],
    }
    EQUAL = 0x0014 {
        text: "equal",
        signature: "(left: word, right: word) => boolean",
        operands: [Result, Register, Register],
    }
    CONSTANT_TYPE = 0x0015 {
        text: "constant",
        signature: "(type: TypeId) => typeId",
        operands: [Result, Type],
    }

    // aggregates
    AGGREGATE = 0x0016 {
        text: "aggregate",
        signature: "(placements: placement[]) => value",
        operands: [ResultRange, Aggregate],
    }
    EXTRACT = 0x0017 {
        text: "extract",
        signature: "(value: value, byteOffset: uint32, byteLength: uint32) => value",
        operands: [ResultRange, RegisterSpan, Unsigned32, Unsigned32],
    }
    INSERT = 0x0018 {
        text: "insert",
        signature: "(aggregate: value, byteOffset: uint32, byteLength: uint32, value: value) => value",
        operands: [ResultRange, RegisterSpan, Unsigned32, Unsigned32, RegisterSpan],
    }
    EQUAL_BYTES = 0x0019 {
        text: "equal.bytes",
        signature: "(left: value, right: value, byteLength: uint32) => boolean",
        operands: [Result, RegisterSpan, RegisterSpan, Unsigned32],
    }
    VARIANT_NEW = 0x001b {
        text: "variant.new",
        signature: "(layout: LayoutId, case: uint32, payload?: value) => value",
        operands: [ResultRange, Layout, Unsigned32, RegisterSpan],
    }
    VARIANT_TAG = 0x001c {
        text: "variant.tag",
        signature: "(variant: value, layout: LayoutId) => value",
        operands: [ResultRange, RegisterSpan, Layout],
    }
    VARIANT_TAG_LOAD = 0x001d {
        text: "variant.tag.load",
        signature: "(variant: reference, layout: LayoutId) => value",
        operands: [ResultRange, Register, Layout],
    }
    VARIANT_TAG_LOAD_CONSTANT = 0x001e {
        text: "variant.tag.load.constant",
        signature: "(variant: reference, layout: LayoutId) => value",
        operands: [ResultRange, Register, Layout],
    }
    VARIANT_TAG_LOAD_POINTER = 0x001f {
        text: "variant.tag.load.pointer",
        signature: "(variant: pointer, layout: LayoutId) => value",
        operands: [ResultRange, Register, Layout],
    }

    // constants
    CONSTANT_INT128 = 0x0021 {
        text: "constant",
        signature: "(bits: int128) => int128",
        operands: [ResultRange, Bits128],
    }
    CONSTANT_UINT128 = 0x0022 {
        text: "constant",
        signature: "(bits: uint128) => uint128",
        operands: [ResultRange, Bits128],
    }
    CONSTANT_NULL = 0x0023 {
        text: "constant",
        signature: "() => value",
        operands: [ResultRange],
    }
    CONSTANT_UNDEFINED = 0x0024 {
        text: "constant",
        signature: "() => value",
        operands: [ResultRange],
    }
    CONSTANT_ZEROED = 0x0026 {
        text: "constant",
        signature: "() => uninit",
        operands: [ResultRange],
    }

    // references and pointers
    FRAME_ADDRESS = 0x0030 {
        text: "frame.address",
        signature: "(value: value) => reference",
        operands: [Result, RegisterSpan],
    }
    GLOBAL_ADDRESS_CONSTANT = 0x0031 {
        text: "global.address.constant",
        signature: "(global: GlobalId) => reference",
        operands: [Result, Global],
    }
    GLOBAL_ADDRESS_LOCAL = 0x0032 {
        text: "global.address.local",
        signature: "(global: GlobalId) => reference",
        operands: [Result, Global],
    }
    GLOBAL_ADDRESS_SHARED = 0x0033 {
        text: "global.address.shared",
        signature: "(global: GlobalId) => reference",
        operands: [Result, Global],
    }
    REFERENCE_ADD_IMMEDIATE = 0x0034 {
        text: "reference.add",
        signature: "(base: reference, byteOffset: int32) => reference",
        operands: [Result, Register, Signed32],
    }
    REFERENCE_ADD = 0x0035 {
        text: "reference.add",
        signature: "(base: reference, byteOffset: int64) => reference",
        operands: [Result, Register, Register],
    }
    REFERENCE_ADD_SCALED = 0x0036 {
        text: "reference.add",
        signature: "(base: reference, offset: int64, scale: uint32) => reference",
        operands: [Result, Register, Register, Unsigned32],
    }
    REFERENCE_DIFF = 0x0037 {
        text: "reference.diff",
        signature: "(reference: reference, origin: reference) => int64",
        operands: [Result, Register, Register],
    }
    POINTER_ADD_IMMEDIATE = 0x0038 {
        text: "pointer.add",
        signature: "(base: pointer, byteOffset: int32) => pointer",
        operands: [Result, Register, Signed32],
    }
    POINTER_ADD = 0x0039 {
        text: "pointer.add",
        signature: "(base: pointer, byteOffset: int64) => pointer",
        operands: [Result, Register, Register],
    }
    POINTER_ADD_SCALED = 0x003a {
        text: "pointer.add",
        signature: "(base: pointer, offset: int64, scale: uint32) => pointer",
        operands: [Result, Register, Register, Unsigned32],
    }
    POINTER_DIFF = 0x003b {
        text: "pointer.diff",
        signature: "(pointer: pointer, origin: pointer) => int64",
        operands: [Result, Register, Register],
    }

    // memory
    LOAD = 0x0050 {
        text: "load",
        signature: "(reference: reference, byteLength: uint32) => value",
        operands: [ResultRange, Register, Unsigned32],
    }
    STORE = 0x0051 {
        text: "store",
        signature: "(reference: reference, value: value, byteLength: uint32) => void",
        operands: [Register, RegisterSpan, Unsigned32],
    }
    LOAD_CONSTANT = 0x0052 {
        text: "load.constant",
        signature: "(reference: reference, byteLength: uint32) => value",
        operands: [ResultRange, Register, Unsigned32],
    }
    LOAD_POINTER = 0x0053 {
        text: "load.pointer",
        signature: "(pointer: pointer, byteLength: uint32) => value",
        operands: [ResultRange, Register, Unsigned32],
    }
    STORE_POINTER = 0x0054 {
        text: "store.pointer",
        signature: "(pointer: pointer, value: value, byteLength: uint32) => void",
        operands: [Register, RegisterSpan, Unsigned32],
    }
    LOAD_VOLATILE = 0x0055 {
        text: "load.volatile",
        signature: "(reference: reference, byteLength: uint32) => value",
        operands: [ResultRange, Register, Unsigned32],
    }
    STORE_VOLATILE = 0x0056 {
        text: "store.volatile",
        signature: "(reference: reference, value: value, byteLength: uint32) => void",
        operands: [Register, RegisterSpan, Unsigned32],
    }
    LOAD_VOLATILE_POINTER = 0x0057 {
        text: "load.volatile.pointer",
        signature: "(pointer: pointer, byteLength: uint32) => value",
        operands: [ResultRange, Register, Unsigned32],
    }
    STORE_VOLATILE_POINTER = 0x0058 {
        text: "store.volatile.pointer",
        signature: "(pointer: pointer, value: value, byteLength: uint32) => void",
        operands: [Register, RegisterSpan, Unsigned32],
    }

    // function values
    FUNCTION_ADDRESS = 0x0060 {
        text: "function.address",
        signature: "(function: FunctionId) => functionPointer",
        operands: [Result, Function],
    }
    FUNCTION_BIND = 0x0061 {
        text: "function.bind",
        signature: "(function: FunctionId, environment: ref) => function",
        operands: [ResultRange, Function, Register],
    }

    // execution contexts
    CONTEXT_CURRENT = 0x0068 {
        text: "context.current",
        signature: "() => context",
        operands: [Result],
    }
    CONTEXT_REPLACE = 0x0069 {
        text: "context.replace",
        signature: "(context: context) => context",
        operands: [Result, Register],
    }
    CONTEXT_BIND = 0x006a {
        text: "context.bind",
        signature: "(context: context, variable: ref, value: value, site: AllocationSiteId, valueOffset: uint32) => context",
        operands: [Result, Register, Register, RegisterSpan, Allocation, Unsigned32],
    }
    CONTEXT_GET = 0x006b {
        text: "context.get",
        signature: "(context: context, variable: ref, default: value, valueOffset: uint32) => value",
        operands: [ResultRange, Register, Register, RegisterSpan, Unsigned32],
    }

    // slices
    SLICE_VIEW = 0x0070 {
        text: "slice.view",
        signature: "(slice: slice, stride: uint32, start: uint64, length: uint64) => slice",
        operands: [ResultRange, RegisterSpan, Unsigned32, Register, Register],
    }

    // dynamic values
    DYNAMIC_BIND = 0x0078 {
        text: "dynamic.bind",
        signature: "(payload: ref, table: DynamicTableId) => dynamic",
        operands: [ResultRange, Register, DynamicTable],
    }
    DYNAMIC_TYPE = 0x007a {
        text: "dynamic.type",
        signature: "(value: dynamic) => typeId",
        operands: [Result, RegisterSpan],
    }

    // allocation and destruction
    FREE = 0x0081 {
        text: "free",
        signature: "(value: ref<unique>) => void",
        operands: [Register, Reference],
    }
    DROP = 0x0082 {
        text: "drop",
        signature: "(value: value, destructor: FunctionId) => void",
        operands: [RegisterSpan, Function],
    }
    PIN = 0x0088 {
        text: "pin",
        signature: "(value: ref<managed>) => void",
        operands: [Register, Reference],
    }
    UNPIN = 0x0089 {
        text: "unpin",
        signature: "(value: ref<managed>) => void",
        operands: [Register, Reference],
    }

    // collector protocol
    BARRIER = 0x0090 {
        text: "barrier",
        signature: "(object: ref<managed>, offset: uint64, byteLength: uint64) => void",
        operands: [Register, Reference, Register, Register],
    }

    // calls
    CALL = 0x00a0 {
        text: "call",
        signature: "(callee: FunctionId, arguments: value[]) => value[]",
        operands: [ResultRange, Function, RegisterSpan],
    }
    CALL_INDIRECT = 0x00a1 {
        text: "call.indirect",
        signature: "(callee: function | functionPointer, arguments: value[]) => value[]",
        operands: [ResultRange, RegisterSpan, RegisterSpan],
    }
    CALL_VIRTUAL = 0x00a2 {
        text: "call.virtual",
        signature: "(receiver: ref, dispatchOffset: uint32, slot: uint16, arguments: value[]) => value[]",
        operands: [ResultRange, Register, Reference, Unsigned32, Unsigned16, RegisterSpan],
    }
    CALL_DYNAMIC = 0x00a3 {
        text: "call.dynamic",
        signature: "(receiver: dynamic, slot: uint16, arguments: value[]) => value[]",
        operands: [ResultRange, RegisterSpan, Unsigned16, RegisterSpan],
    }
    INVOKE = 0x00a4 {
        text: "invoke",
        signature: "(callee: FunctionId, arguments: value[], normal: label, unwind: label) => value[]",
        operands: [ResultRange, Function, RegisterSpan, Branch, Branch],
    }
    INVOKE_INDIRECT = 0x00a5 {
        text: "invoke.indirect",
        signature: "(callee: function | functionPointer, arguments: value[], normal: label, unwind: label) => value[]",
        operands: [ResultRange, RegisterSpan, RegisterSpan, Branch, Branch],
    }
    INVOKE_VIRTUAL = 0x00a6 {
        text: "invoke.virtual",
        signature: "(receiver: ref, dispatchOffset: uint32, slot: uint16, arguments: value[], normal: label, unwind: label) => value[]",
        operands: [ResultRange, Register, Reference, Unsigned32, Unsigned16, RegisterSpan, Branch, Branch],
    }
    INVOKE_DYNAMIC = 0x00a7 {
        text: "invoke.dynamic",
        signature: "(receiver: dynamic, slot: uint16, arguments: value[], normal: label, unwind: label) => value[]",
        operands: [ResultRange, RegisterSpan, Unsigned16, RegisterSpan, Branch, Branch],
    }
    TAIL_CALL = 0x00a8 {
        text: "tail.call",
        signature: "(callee: FunctionId, arguments: value[]) => never",
        operands: [Function, RegisterSpan],
    }
    TAIL_CALL_INDIRECT = 0x00a9 {
        text: "tail.call.indirect",
        signature: "(callee: function | functionPointer, arguments: value[]) => never",
        operands: [RegisterSpan, RegisterSpan],
    }
    TAIL_CALL_VIRTUAL = 0x00aa {
        text: "tail.call.virtual",
        signature: "(receiver: ref, dispatchOffset: uint32, slot: uint16, arguments: value[]) => never",
        operands: [Register, Reference, Unsigned32, Unsigned16, RegisterSpan],
    }
    TAIL_CALL_DYNAMIC = 0x00ab {
        text: "tail.call.dynamic",
        signature: "(receiver: dynamic, slot: uint16, arguments: value[]) => never",
        operands: [RegisterSpan, Unsigned16, RegisterSpan],
    }

    // continuation values
    CONTINUATION_NEW = 0x00ac {
        text: "continuation.new",
        signature: "(function: FunctionId, captures: value[]) => continuation",
        operands: [Result, Function, RegisterSpan],
    }
    CONTINUATION_DESTROY = 0x00ad {
        text: "continuation.destroy",
        signature: "(continuation: continuation) => void",
        operands: [Register],
    }

    // waiters
    WAITER_QUEUE = 0x00ae {
        text: "waiter.queue",
        signature: "(waiter: waiter, value: T, type: TypeId) => boolean",
        operands: [Result, Register, Type, RegisterSpan],
    }
    WAITER_CANCEL = 0x00af {
        text: "waiter.cancel",
        signature: "(waiter: waiter) => boolean",
        operands: [Result, Register],
    }

    // control flow
    JUMP = 0x00b0 {
        text: "jump",
        signature: "(target: label) => never",
        operands: [Branch],
    }
    BRANCH = 0x00b1 {
        text: "branch",
        signature: "(condition: boolean, then: label, else: label) => never",
        operands: [Register, Branch, Branch],
    }
    SWITCH = 0x00b2 {
        text: "switch",
        signature: "(value: uint32, cases: (uint32, label)[], fallback: label) => never",
        operands: [Register, Switch, Branch],
    }
    AWAIT = 0x00b3 {
        text: "await",
        signature: "(park: function, awaitable: value, resume: label, cancel: label, unwind: label) => value",
        operands: [ResultRange, Function, RegisterSpan, Branch, Branch, Branch],
    }
    YIELD = 0x00b4 {
        text: "yield",
        signature: "(value: value, resume: label, complete: label, unwind: label) => resumeValue | completeValue",
        operands: [ResultRange, ResultRange, RegisterSpan, Branch, Branch, Branch],
    }
    CONTINUATION_RESUME = 0x00b5 {
        text: "continuation.resume",
        signature: "(continuation: continuation, value: value, yielded: label, returned: label, unwind: label) => (yield: value, continuation: continuation) | (return: value)",
        operands: [ResultRange, Result, ResultRange, Register, RegisterSpan, Branch, Branch, Branch],
    }
    CONTINUATION_COMPLETE = 0x00b6 {
        text: "continuation.complete",
        signature: "(continuation: continuation, value: value, yielded: label, returned: label, unwind: label) => (yield: value, continuation: continuation) | (return: value)",
        operands: [ResultRange, Result, ResultRange, Register, RegisterSpan, Branch, Branch, Branch],
    }
    RETURN = 0x00b7 {
        text: "return",
        signature: "(results: value[]) => never",
        operands: [RegisterSpan],
    }
    TRAP = 0x00b8 {
        text: "trap",
        signature: "(kind: TrapKind) => never",
        operands: [Unsigned16],
    }
    UNREACHABLE = 0x00b9 {
        text: "unreachable",
        signature: "() => never",
        operands: [],
    }
    BREAKPOINT = 0x00ba {
        text: "breakpoint",
        signature: "() => void",
        operands: [],
    }
    POLL = 0x00bb {
        text: "poll",
        signature: "() => void",
        operands: [],
    }

    // panic and unwind
    PANIC = 0x00c1 {
        text: "panic",
        signature: "() => never",
        operands: [],
    }
    PANIC_VALUE = 0x00c2 {
        text: "panic",
        signature: "(value: value) => never",
        operands: [Type, RegisterSpan],
    }
    UNWIND_RESUME = 0x00c3 {
        text: "unwind.resume",
        signature: "() => never",
        operands: [],
    }

    // atomic memory
    ATOMIC_FENCE = 0x00c8 {
        text: "atomic.fence",
        signature: "(order: AtomicOrder, scope: ExecutionScope, storage: StorageSet) => void",
        operands: [FenceAccess],
    }

    // runtime checks
    CHECK_NULLISH = 0x00d0 {
        text: "check.nullish",
        signature: "(value: value, failure: label) => void",
        operands: [Register, Branch],
    }
    CHECK_EXACT_TYPE = 0x00d1 {
        text: "check.type",
        signature: "(value: value, expected: TypeId, failure: label) => void",
        operands: [Register, Type, Branch],
    }
    CHECK_SUBTYPE = 0x00d2 {
        text: "check.subtype",
        signature: "(value: value, expected: TypeId, failure: label) => void",
        operands: [Register, Type, Branch],
    }

    // casts
    CAST_POINTER_TO_INT = 0x00d8 {
        text: "cast.pointerToInt",
        signature: "(value: pointer) => uint64",
        operands: [Result, Register],
    }
    CAST_INT_TO_POINTER = 0x00d9 {
        text: "cast.intToPointer",
        signature: "(value: uint64) => pointer",
        operands: [Result, Register],
    }

    // profile instrumentation
    PROFILE_INCREMENT = 0x00e0 {
        text: "profile.increment",
        signature: "(counter: CounterId) => void",
        operands: [Counter],
    }
    PROFILE_SAMPLE = 0x00e1 {
        text: "profile.sample",
        signature: "(sampler: SamplerId, value: value) => void",
        operands: [Sampler, Register],
    }

    // tasks
    TASK_RESOLVE = 0x00e8 {
        text: "task.resolve",
        signature: "(value: T, type: TypeId) => task<T>",
        operands: [Result, Type, RegisterSpan],
    }
    TASK_START = 0x00e9 {
        text: "task.start",
        signature: "(continuation: continuation<void, never, T>) => task<T>",
        operands: [Result, Register],
    }
    TASK_PARK = 0x00ea {
        text: "task.park",
        signature: "(task: task<T>, waiter: waiter<T>) => void",
        operands: [Register, Register],
    }
    TASK_CANCEL = 0x00eb {
        text: "task.cancel",
        signature: "(task: task<T>) => void",
        operands: [Register],
    }
    TASK_DETACH = 0x00ec {
        text: "task.detach",
        signature: "(task: task<T>) => void",
        operands: [Register],
    }

    ;

    /// Scalar constants.
    CONSTANT in 0x0100..0x0200 {
        layout: constant_layout,
    }
    /// Boolean operations.
    BOOLEAN in 0x0200..0x0300 {
        layout: boolean_layout,
    }
    /// Native-width integer operations.
    INTEGER in 0x0300..0x0500 {
        layout: integer_layout,
    }
    /// Wide integer operations.
    INTEGER128 in 0x0500..0x0600 {
        layout: integer128_layout,
    }
    /// Floating-point operations.
    FLOAT in 0x0600..0x0700 {
        layout: float_layout,
    }
    /// Integer representation conversions.
    CAST_INTEGER in 0x0700..0x0800 {
        layout: cast_layout,
    }
    /// Floating-point to integer conversions.
    CAST_FLOAT_TO_INT in 0x0800..0x0840 {
        layout: cast_layout,
    }
    /// Integer to floating-point conversions.
    CAST_INT_TO_FLOAT in 0x0840..0x0860 {
        layout: cast_layout,
    }
    /// Floating-point width conversions.
    CAST_FLOAT in 0x0860..0x0880 {
        layout: cast_layout,
    }
    /// Equal-width scalar bit casts.
    CAST_BIT in 0x0880..0x0a00 {
        layout: cast_layout,
    }
    /// Scalar memory operations.
    MEMORY in 0x0a00..0x0b00 {
        layout: memory_layout,
    }
    /// Scalar atomic operations.
    ATOMIC in 0x0b00..0x0c00 {
        layout: atomic_layout,
    }
    /// Scalar atomic operations through native pointers.
    ATOMIC_POINTER in 0x0c00..0x0d00 {
        layout: atomic_layout,
    }
    /// Allocation operations.
    NEW in 0x0d00..0x0d20 {
        layout: new_layout,
    }
    /// Scalar runtime checks.
    CHECK in 0x0d20..0x0da0 {
        layout: check_layout,
    }
    /// Fused scalar branches.
    SCALAR_BRANCH in 0x0da0..0x0e00 {
        layout: branch_layout,
    }
    /// Vector operations.
    VECTOR in 0x0e00..0x0f00 {
        layout: vector_layout,
    }
    /// Tensor operations.
    TENSOR in 0x0f00..0x0f40 {
        layout: tensor_layout,
    }
    /// Byte range transfers.
    TRANSFER in 0x0f40..0x0f68 {
        layout: transfer_layout,
    }
    /// Byte range fills.
    FILL in 0x0f68..0x0f70 {
        layout: fill_layout,
    }
    /// Byte range comparisons.
    COMPARE in 0x0f70..0x0f88 {
        layout: compare_layout,
    }
    /// Memory prefetch operations.
    PREFETCH in 0x0f88..0x0f90 {
        layout: prefetch_layout,
    }
}

impl Opcode {
    /// The greatest opcode representable by an instruction header.
    pub const MAX: Self = Self(0x0fff);

    /// The reserved invalid opcode.
    pub const INVALID: Self = Self(0);

    /// Return whether this opcode belongs to one parameterized operation range.
    pub const fn is_parameterized(self) -> bool {
        self.0 >= OpcodeRange::CONSTANT.start()
    }

    /// Create one exact scalar constant opcode.
    pub const fn constant(scalar: Scalar) -> Self {
        Self(OpcodeRange::CONSTANT.start() + scalar.code() as u16)
    }

    /// Create one exact boolean opcode.
    pub const fn boolean(operation: BooleanOperation) -> Self {
        Self(OpcodeRange::BOOLEAN.start() + operation as u16)
    }

    /// Create one exact scalar integer opcode.
    pub const fn integer(operation: IntegerOperation, scalar: Scalar) -> Option<Self> {
        match scalar.integer_index() {
            Some(scalar) => Some(Self(
                OpcodeRange::INTEGER.start()
                    + operation as u16 * Scalar::INTEGER_OPCODE_STRIDE
                    + scalar,
            )),
            None => None,
        }
    }

    /// Create one exact 128-bit integer opcode.
    pub const fn integer128(operation: IntegerOperation, is_signed: bool) -> Self {
        Self(OpcodeRange::INTEGER128.start() + operation as u16 * 2 + is_signed as u16)
    }

    /// Create one exact scalar floating-point opcode.
    pub const fn float(operation: FloatOperation, scalar: Scalar) -> Option<Self> {
        match scalar.float_index() {
            Some(scalar) => Some(Self(
                OpcodeRange::FLOAT.start()
                    + operation as u16 * Scalar::FLOAT_OPCODE_STRIDE
                    + scalar,
            )),
            None => None,
        }
    }

    /// Create one exact value conversion opcode.
    pub const fn cast(
        operation: CastOperation,
        source: ValueType,
        target: ValueType,
    ) -> Option<Self> {
        let source_scalar = source.scalar_type();
        let target_scalar = target.scalar_type();
        let source_integer = match source_scalar {
            Some(source) => source.integer_index(),
            None => None,
        };
        let target_integer = match target_scalar {
            Some(target) => target.integer_index(),
            None => None,
        };
        let source_float = match source_scalar {
            Some(source) => source.float_index(),
            None => None,
        };
        let target_float = match target_scalar {
            Some(target) => target.float_index(),
            None => None,
        };

        match operation {
            CastOperation::Truncate
            | CastOperation::Saturate
            | CastOperation::SignExtend
            | CastOperation::ZeroExtend => match (source_integer, target_integer) {
                (Some(source), Some(target)) if operation.supports(source, target) => {
                    let operation = operation as u16;
                    let conversion = source * Scalar::INTEGER_OPCODE_STRIDE + target;

                    Some(Self(
                        OpcodeRange::CAST_INTEGER.start()
                            + operation
                                * Scalar::INTEGER_OPCODE_STRIDE
                                * Scalar::INTEGER_OPCODE_STRIDE
                            + conversion,
                    ))
                }
                _ => None,
            },
            CastOperation::FloatToInt | CastOperation::FloatToIntSaturating => {
                match (source_float, target_integer) {
                    (Some(source), Some(target)) => {
                        let operation = operation as u16 - CastOperation::FloatToInt as u16;
                        let conversion = source * Scalar::INTEGER_OPCODE_STRIDE + target;

                        Some(Self(
                            OpcodeRange::CAST_FLOAT_TO_INT.start()
                                + operation
                                    * Scalar::FLOAT_OPCODE_STRIDE
                                    * Scalar::INTEGER_OPCODE_STRIDE
                                + conversion,
                        ))
                    }
                    _ => None,
                }
            }
            CastOperation::IntToFloat => match (source_integer, target_float) {
                (Some(source), Some(target)) => Some(Self(
                    OpcodeRange::CAST_INT_TO_FLOAT.start()
                        + source * Scalar::FLOAT_OPCODE_STRIDE
                        + target,
                )),
                _ => None,
            },
            CastOperation::FloatConvert => match (source_float, target_float) {
                (Some(source), Some(target)) if source != target => Some(Self(
                    OpcodeRange::CAST_FLOAT.start() + source * Scalar::FLOAT_OPCODE_STRIDE + target,
                )),
                _ => None,
            },
            CastOperation::Bit => match (source_scalar, target_scalar) {
                (Some(source), Some(target))
                    if source.code() != target.code()
                        && source.bit_width() == target.bit_width() =>
                {
                    let conversion =
                        source.code() as u16 * Scalar::OPCODE_STRIDE + target.code() as u16;

                    Some(Self(OpcodeRange::CAST_BIT.start() + conversion))
                }
                _ => None,
            },
            CastOperation::PointerToInt => match target_scalar {
                Some(Scalar::Uint64) if source.is_pointer() => Some(Self::CAST_POINTER_TO_INT),
                _ => None,
            },
            CastOperation::IntToPointer => match source_scalar {
                Some(Scalar::Uint64) if target.is_pointer() => Some(Self::CAST_INT_TO_POINTER),
                _ => None,
            },
        }
    }

    /// Create one exact scalar memory opcode.
    pub const fn memory(
        operation: MemoryOperation,
        address: Address,
        scalar: Scalar,
        is_volatile: bool,
    ) -> Option<Self> {
        if !operation.supports(address, is_volatile) {
            return None;
        }

        let operation = operation as u16 * 2 + is_volatile as u16;
        let operation = operation * Address::COUNT + address as u16;

        Some(Self(
            OpcodeRange::MEMORY.start() + operation * Scalar::OPCODE_STRIDE + scalar.code() as u16,
        ))
    }

    /// Create one exact packed memory opcode.
    pub const fn memory_range(
        operation: MemoryOperation,
        address: Address,
        is_volatile: bool,
    ) -> Option<Self> {
        match (operation, address, is_volatile) {
            (MemoryOperation::Load, Address::Memory, false) => Some(Self::LOAD),
            (MemoryOperation::Load, Address::Constant, false) => Some(Self::LOAD_CONSTANT),
            (MemoryOperation::Load, Address::Pointer, false) => Some(Self::LOAD_POINTER),
            (MemoryOperation::Store, Address::Memory, false) => Some(Self::STORE),
            (MemoryOperation::Store, Address::Pointer, false) => Some(Self::STORE_POINTER),
            (MemoryOperation::Load, Address::Memory, true) => Some(Self::LOAD_VOLATILE),
            (MemoryOperation::Load, Address::Pointer, true) => Some(Self::LOAD_VOLATILE_POINTER),
            (MemoryOperation::Store, Address::Memory, true) => Some(Self::STORE_VOLATILE),
            (MemoryOperation::Store, Address::Pointer, true) => Some(Self::STORE_VOLATILE_POINTER),
            _ => None,
        }
    }

    /// Create one stored variant discriminant opcode.
    pub const fn variant_tag_load(address: Address) -> Self {
        match address {
            Address::Memory => Self::VARIANT_TAG_LOAD,
            Address::Constant => Self::VARIANT_TAG_LOAD_CONSTANT,
            Address::Pointer => Self::VARIANT_TAG_LOAD_POINTER,
        }
    }

    /// Return this stored variant discriminant opcode's address representation.
    pub const fn variant_tag_load_address(self) -> Option<Address> {
        match self {
            Self::VARIANT_TAG_LOAD => Some(Address::Memory),
            Self::VARIANT_TAG_LOAD_CONSTANT => Some(Address::Constant),
            Self::VARIANT_TAG_LOAD_POINTER => Some(Address::Pointer),
            _ => None,
        }
    }

    /// Create one exact byte range transfer opcode.
    pub const fn transfer(
        operation: Transfer,
        target: Address,
        source: Address,
        is_immediate: bool,
    ) -> Option<Self> {
        if !target.is_writable() {
            return None;
        }

        let operation = operation as u16 * Address::COUNT + target as u16;
        let operation = operation * Address::COUNT + source as u16;
        let operation = operation * 2 + is_immediate as u16;

        Some(Self(OpcodeRange::TRANSFER.start() + operation))
    }

    /// Create one exact byte range fill opcode.
    pub const fn fill(target: Address, is_immediate: bool) -> Option<Self> {
        if !target.is_writable() {
            return None;
        }

        let operation = target as u16 * 2 + is_immediate as u16;

        Some(Self(OpcodeRange::FILL.start() + operation))
    }

    /// Create one exact byte range comparison opcode.
    pub const fn compare(left: Address, right: Address, is_immediate: bool) -> Self {
        let operation = left as u16 * Address::COUNT + right as u16;
        let operation = operation * 2 + is_immediate as u16;

        Self(OpcodeRange::COMPARE.start() + operation)
    }

    /// Create one exact prefetch opcode.
    pub const fn prefetch(operation: Prefetch, address: Address) -> Option<Self> {
        if !operation.supports(address) {
            return None;
        }

        let operation = operation as u16 * Address::COUNT + address as u16;

        Some(Self(OpcodeRange::PREFETCH.start() + operation))
    }

    /// Create one exact scalar atomic opcode.
    pub const fn atomic(
        operation: AtomicOperation,
        address: Address,
        scalar: Scalar,
    ) -> Option<Self> {
        if !operation.supports(scalar) || !address.is_writable() {
            return None;
        }

        let range = match address {
            Address::Memory => OpcodeRange::ATOMIC,
            Address::Pointer => OpcodeRange::ATOMIC_POINTER,
            Address::Constant => return None,
        };

        Some(Self(
            range.start() + operation as u16 * Scalar::OPCODE_STRIDE + scalar.code() as u16,
        ))
    }

    /// Create one exact `new` opcode.
    pub const fn new(operation: New) -> Option<Self> {
        match operation.code() {
            Some(code) => Some(Self(OpcodeRange::NEW.start() + code)),
            None => None,
        }
    }

    /// Create one exact scalar runtime check opcode.
    pub const fn check(check: ScalarCheck, scalar: Scalar) -> Option<Self> {
        if check.supports(scalar) {
            Some(Self(
                OpcodeRange::CHECK.start()
                    + check as u16 * Scalar::OPCODE_STRIDE
                    + scalar.code() as u16,
            ))
        } else {
            None
        }
    }

    /// Create one exact fused scalar branch opcode.
    pub const fn branch(comparison: Comparison, scalar: Scalar) -> Option<Self> {
        if comparison.supports(scalar) {
            Some(Self(
                OpcodeRange::SCALAR_BRANCH.start()
                    + comparison as u16 * Scalar::OPCODE_STRIDE
                    + scalar.code() as u16,
            ))
        } else {
            None
        }
    }

    /// Create one vector opcode.
    pub const fn vector(operation: VectorOperation) -> Self {
        Self(OpcodeRange::VECTOR.start() + operation as u16)
    }

    /// Create one tensor opcode.
    pub const fn tensor(operation: TensorOperation) -> Self {
        Self(OpcodeRange::TENSOR.start() + operation as u16)
    }

    /// Create one opcode from its exact stable code.
    pub const fn from_code(code: u16) -> Self {
        Self(code)
    }

    /// Decode one scalar constant opcode.
    pub const fn constant_scalar(self) -> Option<Scalar> {
        if OpcodeRange::CONSTANT.contains(self.0) {
            Scalar::from_code((self.0 - OpcodeRange::CONSTANT.start()) as u8)
        } else {
            None
        }
    }

    /// Decode one scalar boolean opcode.
    pub const fn boolean_operation(self) -> Option<BooleanOperation> {
        if OpcodeRange::BOOLEAN.contains(self.0) {
            BooleanOperation::from_code((self.0 - OpcodeRange::BOOLEAN.start()) as u8)
        } else {
            None
        }
    }

    /// Decode one scalar integer opcode.
    pub const fn integer_operation(self) -> Option<(IntegerOperation, Scalar)> {
        if !OpcodeRange::INTEGER.contains(self.0) {
            return None;
        }
        let code = self.0 - OpcodeRange::INTEGER.start();
        let operation = IntegerOperation::from_code((code / Scalar::INTEGER_OPCODE_STRIDE) as u8);
        let scalar = Scalar::from_code((code % Scalar::INTEGER_OPCODE_STRIDE + 1) as u8);

        match (operation, scalar) {
            (Some(operation), Some(scalar)) => Some((operation, scalar)),
            _ => None,
        }
    }

    /// Decode one 128-bit integer opcode.
    pub const fn integer128_operation(self) -> Option<(IntegerOperation, bool)> {
        if !OpcodeRange::INTEGER128.contains(self.0) {
            return None;
        }
        let code = self.0 - OpcodeRange::INTEGER128.start();
        let operation = IntegerOperation::from_code((code / 2) as u8);

        match operation {
            Some(operation) => Some((operation, code % 2 == 1)),
            None => None,
        }
    }

    /// Decode one scalar floating-point opcode.
    pub const fn float_operation(self) -> Option<(FloatOperation, Scalar)> {
        if !OpcodeRange::FLOAT.contains(self.0) {
            return None;
        }
        let code = self.0 - OpcodeRange::FLOAT.start();
        let operation = FloatOperation::from_code((code / Scalar::FLOAT_OPCODE_STRIDE) as u8);
        let scalar = Scalar::from_code((code % Scalar::FLOAT_OPCODE_STRIDE + 9) as u8);

        match (operation, scalar) {
            (Some(operation), Some(scalar)) => Some((operation, scalar)),
            _ => None,
        }
    }

    /// Decode one value conversion opcode.
    pub const fn cast_operation(self) -> Option<(CastOperation, ValueType, ValueType)> {
        let code = self.0;

        if self.0 == Self::CAST_POINTER_TO_INT.0 {
            return Some((
                CastOperation::PointerToInt,
                ValueType::pointer(),
                ValueType::scalar(Scalar::Uint64),
            ));
        }
        if self.0 == Self::CAST_INT_TO_POINTER.0 {
            return Some((
                CastOperation::IntToPointer,
                ValueType::scalar(Scalar::Uint64),
                ValueType::pointer(),
            ));
        }

        if OpcodeRange::CAST_INTEGER.contains(code) {
            let code = code - OpcodeRange::CAST_INTEGER.start();
            let square = Scalar::INTEGER_OPCODE_STRIDE * Scalar::INTEGER_OPCODE_STRIDE;
            let operation = CastOperation::from_code((code / square) as u8);
            let conversion = code % square;
            let source = Scalar::from_code((conversion / Scalar::INTEGER_OPCODE_STRIDE + 1) as u8);
            let target = Scalar::from_code((conversion % Scalar::INTEGER_OPCODE_STRIDE + 1) as u8);

            return match (operation, source, target) {
                (Some(operation), Some(source), Some(target)) => Some((
                    operation,
                    ValueType::scalar(source),
                    ValueType::scalar(target),
                )),
                _ => None,
            };
        }

        if OpcodeRange::CAST_FLOAT_TO_INT.contains(code) {
            let code = code - OpcodeRange::CAST_FLOAT_TO_INT.start();
            let square = Scalar::FLOAT_OPCODE_STRIDE * Scalar::INTEGER_OPCODE_STRIDE;
            let operation =
                CastOperation::from_code((code / square) as u8 + CastOperation::FloatToInt as u8);
            let conversion = code % square;
            let source = Scalar::from_code((conversion / Scalar::INTEGER_OPCODE_STRIDE + 9) as u8);
            let target = Scalar::from_code((conversion % Scalar::INTEGER_OPCODE_STRIDE + 1) as u8);

            return match (operation, source, target) {
                (Some(operation), Some(source), Some(target)) => Some((
                    operation,
                    ValueType::scalar(source),
                    ValueType::scalar(target),
                )),
                _ => None,
            };
        }

        if OpcodeRange::CAST_INT_TO_FLOAT.contains(code) {
            let conversion = code - OpcodeRange::CAST_INT_TO_FLOAT.start();
            let source = Scalar::from_code((conversion / Scalar::FLOAT_OPCODE_STRIDE + 1) as u8);
            let target = Scalar::from_code((conversion % Scalar::FLOAT_OPCODE_STRIDE + 9) as u8);

            return match (source, target) {
                (Some(source), Some(target)) => Some((
                    CastOperation::IntToFloat,
                    ValueType::scalar(source),
                    ValueType::scalar(target),
                )),
                _ => None,
            };
        }

        if OpcodeRange::CAST_FLOAT.contains(code) {
            let conversion = code - OpcodeRange::CAST_FLOAT.start();
            let source = Scalar::from_code((conversion / Scalar::FLOAT_OPCODE_STRIDE + 9) as u8);
            let target = Scalar::from_code((conversion % Scalar::FLOAT_OPCODE_STRIDE + 9) as u8);

            return match (source, target) {
                (Some(source), Some(target)) if source.code() != target.code() => Some((
                    CastOperation::FloatConvert,
                    ValueType::scalar(source),
                    ValueType::scalar(target),
                )),
                _ => None,
            };
        }

        if OpcodeRange::CAST_BIT.contains(code) {
            let conversion = code - OpcodeRange::CAST_BIT.start();
            let source = Scalar::from_code((conversion / Scalar::OPCODE_STRIDE) as u8);
            let target = Scalar::from_code((conversion % Scalar::OPCODE_STRIDE) as u8);

            return match (source, target) {
                (Some(source), Some(target))
                    if source.code() != target.code()
                        && source.bit_width() == target.bit_width() =>
                {
                    Some((
                        CastOperation::Bit,
                        ValueType::scalar(source),
                        ValueType::scalar(target),
                    ))
                }
                _ => None,
            };
        }

        None
    }

    /// Decode one scalar memory opcode.
    pub const fn memory_operation(self) -> Option<(MemoryOperation, Address, Scalar, bool)> {
        if !OpcodeRange::MEMORY.contains(self.0) {
            return None;
        }
        let code = self.0 - OpcodeRange::MEMORY.start();
        let operation = code / Scalar::OPCODE_STRIDE;
        let address = Address::from_code((operation % Address::COUNT) as u8);
        let operation = operation / Address::COUNT;
        let is_volatile = !operation.is_multiple_of(2);
        let operation = MemoryOperation::from_code((operation / 2) as u8);
        let scalar = Scalar::from_code((code % Scalar::OPCODE_STRIDE) as u8);

        match (operation, address, scalar) {
            (Some(operation), Some(address), Some(scalar))
                if operation.supports(address, is_volatile) =>
            {
                Some((operation, address, scalar, is_volatile))
            }
            _ => None,
        }
    }

    /// Decode one packed memory opcode.
    pub const fn memory_range_operation(self) -> Option<(MemoryOperation, Address, bool)> {
        match self {
            Self::LOAD => Some((MemoryOperation::Load, Address::Memory, false)),
            Self::LOAD_CONSTANT => Some((MemoryOperation::Load, Address::Constant, false)),
            Self::LOAD_POINTER => Some((MemoryOperation::Load, Address::Pointer, false)),
            Self::STORE => Some((MemoryOperation::Store, Address::Memory, false)),
            Self::STORE_POINTER => Some((MemoryOperation::Store, Address::Pointer, false)),
            Self::LOAD_VOLATILE => Some((MemoryOperation::Load, Address::Memory, true)),
            Self::LOAD_VOLATILE_POINTER => Some((MemoryOperation::Load, Address::Pointer, true)),
            Self::STORE_VOLATILE => Some((MemoryOperation::Store, Address::Memory, true)),
            Self::STORE_VOLATILE_POINTER => Some((MemoryOperation::Store, Address::Pointer, true)),
            _ => None,
        }
    }

    /// Decode one byte range transfer opcode.
    pub const fn transfer_operation(self) -> Option<(Transfer, Address, Address, bool)> {
        if !OpcodeRange::TRANSFER.contains(self.0) {
            return None;
        }

        let code = self.0 - OpcodeRange::TRANSFER.start();
        let is_immediate = !code.is_multiple_of(2);
        let code = code / 2;
        let source = Address::from_code((code % Address::COUNT) as u8);
        let code = code / Address::COUNT;
        let target = Address::from_code((code % Address::COUNT) as u8);
        let operation = Transfer::from_code((code / Address::COUNT) as u8);

        match (operation, target, source) {
            (Some(operation), Some(target), Some(source)) if target.is_writable() => {
                Some((operation, target, source, is_immediate))
            }
            _ => None,
        }
    }

    /// Decode one byte range fill opcode.
    pub const fn fill_operation(self) -> Option<(Address, bool)> {
        if !OpcodeRange::FILL.contains(self.0) {
            return None;
        }

        let code = self.0 - OpcodeRange::FILL.start();
        let is_immediate = !code.is_multiple_of(2);
        let target = Address::from_code((code / 2) as u8);

        match target {
            Some(target) if target.is_writable() => Some((target, is_immediate)),
            _ => None,
        }
    }

    /// Decode one byte range comparison opcode.
    pub const fn compare_operation(self) -> Option<(Address, Address, bool)> {
        if !OpcodeRange::COMPARE.contains(self.0) {
            return None;
        }

        let code = self.0 - OpcodeRange::COMPARE.start();
        let is_immediate = !code.is_multiple_of(2);
        let code = code / 2;
        let right = Address::from_code((code % Address::COUNT) as u8);
        let left = Address::from_code((code / Address::COUNT) as u8);

        match (left, right) {
            (Some(left), Some(right)) => Some((left, right, is_immediate)),
            _ => None,
        }
    }

    /// Decode one prefetch opcode.
    pub const fn prefetch_operation(self) -> Option<(Prefetch, Address)> {
        if !OpcodeRange::PREFETCH.contains(self.0) {
            return None;
        }

        let code = self.0 - OpcodeRange::PREFETCH.start();
        let address = Address::from_code((code % Address::COUNT) as u8);
        let operation = Prefetch::from_code((code / Address::COUNT) as u8);

        match (operation, address) {
            (Some(operation), Some(address)) if operation.supports(address) => {
                Some((operation, address))
            }
            _ => None,
        }
    }

    /// Decode one scalar atomic opcode.
    pub const fn atomic_operation(self) -> Option<(AtomicOperation, Address, Scalar)> {
        let (range, address) = if OpcodeRange::ATOMIC.contains(self.0) {
            (OpcodeRange::ATOMIC, Address::Memory)
        } else if OpcodeRange::ATOMIC_POINTER.contains(self.0) {
            (OpcodeRange::ATOMIC_POINTER, Address::Pointer)
        } else {
            return None;
        };
        let code = self.0 - range.start();
        let operation = AtomicOperation::from_code((code / Scalar::OPCODE_STRIDE) as u8);
        let scalar = Scalar::from_code((code % Scalar::OPCODE_STRIDE) as u8);

        match (operation, scalar) {
            (Some(operation), Some(scalar)) if operation.supports(scalar) => {
                Some((operation, address, scalar))
            }
            _ => None,
        }
    }

    /// Decode one `new` opcode.
    pub const fn new_operation(self) -> Option<New> {
        if !OpcodeRange::NEW.contains(self.0) {
            return None;
        }

        New::from_code(self.0 - OpcodeRange::NEW.start())
    }

    /// Decode one scalar runtime check opcode.
    pub const fn scalar_check(self) -> Option<(ScalarCheck, Scalar)> {
        if !OpcodeRange::CHECK.contains(self.0) {
            return None;
        }
        let code = self.0 - OpcodeRange::CHECK.start();
        let check = ScalarCheck::from_code((code / Scalar::OPCODE_STRIDE) as u8);
        let scalar = Scalar::from_code((code % Scalar::OPCODE_STRIDE) as u8);

        match (check, scalar) {
            (Some(check), Some(scalar)) if check.supports(scalar) => Some((check, scalar)),
            _ => None,
        }
    }

    /// Decode one fused scalar branch opcode.
    pub const fn comparison(self) -> Option<(Comparison, Scalar)> {
        if !OpcodeRange::SCALAR_BRANCH.contains(self.0) {
            return None;
        }
        let code = self.0 - OpcodeRange::SCALAR_BRANCH.start();
        let comparison = Comparison::from_code((code / Scalar::OPCODE_STRIDE) as u8);
        let scalar = Scalar::from_code((code % Scalar::OPCODE_STRIDE) as u8);

        match (comparison, scalar) {
            (Some(comparison), Some(scalar)) if comparison.supports(scalar) => {
                Some((comparison, scalar))
            }
            _ => None,
        }
    }

    /// Decode one vector opcode.
    pub const fn vector_operation(self) -> Option<VectorOperation> {
        if OpcodeRange::VECTOR.contains(self.0) {
            VectorOperation::from_code((self.0 - OpcodeRange::VECTOR.start()) as u8)
        } else {
            None
        }
    }

    /// Decode one tensor opcode.
    pub const fn tensor_operation(self) -> Option<TensorOperation> {
        if OpcodeRange::TENSOR.contains(self.0) {
            TensorOperation::from_code((self.0 - OpcodeRange::TENSOR.start()) as u8)
        } else {
            None
        }
    }

    /// Return this opcode's exact binary operand layout.
    pub fn layout(self) -> Option<InstructionLayout> {
        self.named_layout().or_else(|| self.range_layout())
    }

    /// Return whether this opcode is assigned by the bytecode ISA.
    pub fn is_defined(self) -> bool {
        self.layout().is_some()
    }

    /// Return whether this instruction ends control flow without a successor.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::TAIL_CALL
                | Self::TAIL_CALL_INDIRECT
                | Self::TAIL_CALL_VIRTUAL
                | Self::TAIL_CALL_DYNAMIC
                | Self::RETURN
                | Self::TRAP
                | Self::UNREACHABLE
                | Self::PANIC
                | Self::PANIC_VALUE
                | Self::UNWIND_RESUME
        )
    }

    /// Return whether execution may continue at the next encoded instruction.
    pub fn falls_through(self) -> bool {
        if self.is_terminal() {
            return false;
        }

        if matches!(
            self,
            Self::JUMP
                | Self::BRANCH
                | Self::SWITCH
                | Self::AWAIT
                | Self::YIELD
                | Self::CONTINUATION_RESUME
                | Self::CONTINUATION_COMPLETE
                | Self::INVOKE
                | Self::INVOKE_INDIRECT
                | Self::INVOKE_VIRTUAL
                | Self::INVOKE_DYNAMIC
        ) {
            return false;
        }

        if self.comparison().is_some() {
            return false;
        }

        !self
            .new_operation()
            .is_some_and(|operation| operation.is_fallible)
    }

    /// Return the branch that receives one result operand.
    pub fn result_branch(self, result: usize) -> Option<usize> {
        if matches!(
            self,
            Self::CONTINUATION_RESUME | Self::CONTINUATION_COMPLETE
        ) {
            return match result {
                0 | 1 => Some(0),
                2 => Some(1),
                _ => None,
            };
        }

        if self == Self::YIELD {
            return match result {
                0 => Some(0),
                1 => Some(1),
                _ => None,
            };
        }

        if result == 0
            && (matches!(
                self,
                Self::AWAIT
                    | Self::INVOKE
                    | Self::INVOKE_INDIRECT
                    | Self::INVOKE_VIRTUAL
                    | Self::INVOKE_DYNAMIC
            ) || self
                .new_operation()
                .is_some_and(|operation| operation.is_fallible))
        {
            Some(0)
        } else {
            None
        }
    }

    /// Return this opcode's exact stable code.
    pub const fn code(self) -> u16 {
        self.0
    }

    /// Return the operand layout for one scalar constant opcode.
    fn constant_layout(code: u16) -> Option<InstructionLayout> {
        let scalar = code - OpcodeRange::CONSTANT.start();
        Scalar::from_code(scalar as u8)?;

        Some(InstructionLayout::new(&[Operand::Result, Operand::Bits64]))
    }

    /// Return the operand layout for one scalar cast opcode.
    fn cast_layout(code: u16) -> Option<InstructionLayout> {
        let opcode = Self(code);
        opcode.cast_operation()?;

        Some(InstructionLayout::new(&[
            Operand::Result,
            Operand::Register,
        ]))
    }

    /// Return the operand layout for one fused scalar branch opcode.
    fn branch_layout(code: u16) -> Option<InstructionLayout> {
        let opcode = Self(code);
        opcode.comparison()?;

        Some(InstructionLayout::new(&[
            Operand::Register,
            Operand::Register,
            Operand::Branch,
            Operand::Branch,
        ]))
    }

    /// Return the operand layout for one boolean opcode.
    fn boolean_layout(code: u16) -> Option<InstructionLayout> {
        let operation = BooleanOperation::from_code((code - OpcodeRange::BOOLEAN.start()) as u8)?;
        if operation.input_count() == 1 {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::Register,
            ]))
        }
    }

    /// Return the operand layout for one scalar integer opcode.
    fn integer_layout(code: u16) -> Option<InstructionLayout> {
        let operation = IntegerOperation::from_code(
            ((code - OpcodeRange::INTEGER.start()) / Scalar::INTEGER_OPCODE_STRIDE) as u8,
        )?;
        if operation.input_count() == 1 {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
            ]))
        } else if operation.is_overflowing() {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Result,
                Operand::Register,
                Operand::Register,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::Register,
            ]))
        }
    }

    /// Return the operand layout for one 128-bit integer opcode.
    fn integer128_layout(code: u16) -> Option<InstructionLayout> {
        let operation =
            IntegerOperation::from_code(((code - OpcodeRange::INTEGER128.start()) / 2) as u8)?;
        if operation.is_count() {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::RegisterSpan,
            ]))
        } else if operation.is_comparison() {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::RegisterSpan,
                Operand::RegisterSpan,
            ]))
        } else if operation.is_overflowing() {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::Result,
                Operand::RegisterSpan,
                Operand::RegisterSpan,
            ]))
        } else if operation.input_count() == 1 {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::RegisterSpan,
            ]))
        } else if operation.uses_count() {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::RegisterSpan,
                Operand::Register,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::RegisterSpan,
                Operand::RegisterSpan,
            ]))
        }
    }

    /// Return the operand layout for one scalar floating-point opcode.
    fn float_layout(code: u16) -> Option<InstructionLayout> {
        let operation = FloatOperation::from_code(
            ((code - OpcodeRange::FLOAT.start()) / Scalar::FLOAT_OPCODE_STRIDE) as u8,
        )?;
        if operation.input_count() == 1 {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
            ]))
        } else if operation.input_count() == 3 {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Register,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::Register,
            ]))
        }
    }

    /// Return the operand layout for one scalar memory opcode.
    fn memory_layout(code: u16) -> Option<InstructionLayout> {
        let operation = Self(code).memory_operation()?.0;
        if operation == MemoryOperation::Load {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
            ]))
        }
    }

    /// Return the operand layout for one byte range transfer opcode.
    fn transfer_layout(code: u16) -> Option<InstructionLayout> {
        let is_immediate = Self(code).transfer_operation()?.3;
        if is_immediate {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::Unsigned32,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::Register,
            ]))
        }
    }

    /// Return the operand layout for one byte range fill opcode.
    fn fill_layout(code: u16) -> Option<InstructionLayout> {
        let is_immediate = Self(code).fill_operation()?.1;
        if is_immediate {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::Unsigned32,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::Register,
            ]))
        }
    }

    /// Return the operand layout for one byte range comparison opcode.
    fn compare_layout(code: u16) -> Option<InstructionLayout> {
        let is_immediate = Self(code).compare_operation()?.2;
        if is_immediate {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Unsigned32,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Register,
            ]))
        }
    }

    /// Return the operand layout for one prefetch opcode.
    fn prefetch_layout(code: u16) -> Option<InstructionLayout> {
        Self(code).prefetch_operation()?;

        Some(InstructionLayout::new(&[Operand::Register]))
    }

    /// Return the operand layout for one atomic opcode.
    fn atomic_layout(code: u16) -> Option<InstructionLayout> {
        let operation = Self(code).atomic_operation()?.0;
        if operation == AtomicOperation::Load {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::AtomicAccess,
            ]))
        } else if operation == AtomicOperation::Store {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::AtomicAccess,
            ]))
        } else if operation == AtomicOperation::CompareExchange
            || operation == AtomicOperation::CompareExchangeWeak
        {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Register,
                Operand::CompareExchangeAccess,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::AtomicAccess,
            ]))
        }
    }

    /// Return the operand layout for one `new` opcode.
    fn new_layout(code: u16) -> Option<InstructionLayout> {
        let operation = New::from_code(code - OpcodeRange::NEW.start())?;
        let is_slice = operation.kind == NewKind::Slice;
        let is_fallible = operation.is_fallible;

        if is_slice && is_fallible {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::Allocation,
                Operand::Register,
                Operand::Branch,
                Operand::Branch,
            ]))
        } else if is_slice {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::Allocation,
                Operand::Register,
            ]))
        } else if is_fallible {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Allocation,
                Operand::Branch,
                Operand::Branch,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Allocation,
            ]))
        }
    }

    /// Return the operand layout for one runtime check opcode.
    fn check_layout(code: u16) -> Option<InstructionLayout> {
        let operation = (code - OpcodeRange::CHECK.start()) / Scalar::OPCODE_STRIDE;
        let operation = ScalarCheck::from_code(operation as u8)?;
        if operation == ScalarCheck::Nonzero {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Branch,
            ]))
        } else if operation == ScalarCheck::Shift {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Unsigned16,
                Operand::Branch,
            ]))
        } else if operation == ScalarCheck::Narrow {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Scalar,
                Operand::Branch,
            ]))
        } else if operation == ScalarCheck::Bounds {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::Branch,
            ]))
        } else if operation == ScalarCheck::Range {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::Register,
                Operand::Branch,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::Branch,
            ]))
        }
    }

    /// Return the operand layout for one vector opcode.
    fn vector_layout(code: u16) -> Option<InstructionLayout> {
        let operation = code - OpcodeRange::VECTOR.start();
        let layout = match operation {
            operation if operation == VectorOperation::Splat as u16 => {
                &[Operand::ResultRange, Operand::Register, Operand::VectorType][..]
            }
            operation if operation == VectorOperation::Insert as u16 => &[
                Operand::ResultRange,
                Operand::RegisterSpan,
                Operand::Register,
                Operand::Register,
                Operand::VectorType,
            ],
            operation if operation == VectorOperation::Extract as u16 => &[
                Operand::Result,
                Operand::RegisterSpan,
                Operand::Register,
                Operand::VectorType,
            ],
            operation if operation == VectorOperation::Shuffle as u16 => &[
                Operand::ResultRange,
                Operand::RegisterList,
                Operand::Unsigned16List,
                Operand::VectorType,
            ],
            operation
                if operation == VectorOperation::Element as u16
                    || operation == VectorOperation::Compare as u16 =>
            {
                &[
                    Operand::ResultRange,
                    Operand::RegisterList,
                    Operand::VectorType,
                    Operand::Operator,
                ]
            }
            operation if operation == VectorOperation::Select as u16 => &[
                Operand::ResultRange,
                Operand::RegisterList,
                Operand::VectorType,
            ],
            operation if operation == VectorOperation::Reduce as u16 => &[
                Operand::Result,
                Operand::RegisterSpan,
                Operand::VectorType,
                Operand::Operator,
            ],
            operation if operation == VectorOperation::Convert as u16 => &[
                Operand::ResultRange,
                Operand::RegisterSpan,
                Operand::VectorType,
                Operand::VectorType,
                Operand::Operator,
            ],
            operation if operation == VectorOperation::Load as u16 => {
                &[Operand::ResultRange, Operand::Register, Operand::VectorType]
            }
            operation if operation == VectorOperation::Store as u16 => &[
                Operand::Register,
                Operand::RegisterSpan,
                Operand::VectorType,
            ],
            _ => return None,
        };

        Some(InstructionLayout::new(layout))
    }

    /// Return the operand layout for one tensor opcode.
    fn tensor_layout(code: u16) -> Option<InstructionLayout> {
        let operation = code - OpcodeRange::TENSOR.start();
        let layout = match operation {
            operation if operation == TensorOperation::Element as u16 => &[
                Operand::Result,
                Operand::TensorList,
                Operand::Operator,
                Operand::Allocation,
            ][..],
            operation if operation == TensorOperation::Compare as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::Tensor,
                Operand::Operator,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::Select as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::Tensor,
                Operand::Tensor,
                Operand::Allocation,
            ],
            operation
                if operation == TensorOperation::Transpose as u16
                    || operation == TensorOperation::Broadcast as u16 =>
            {
                &[
                    Operand::Result,
                    Operand::Tensor,
                    Operand::Unsigned16List,
                    Operand::Allocation,
                ]
            }
            operation if operation == TensorOperation::Reshape as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::RegisterList,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::Slice as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::Pad as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::Register,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::Concat as u16 => &[
                Operand::Result,
                Operand::TensorList,
                Operand::Unsigned16,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::Splat as u16 => {
                &[Operand::Result, Operand::Register, Operand::Allocation]
            }
            operation if operation == TensorOperation::Convert as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::Operator,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::Bitcast as u16 => {
                &[Operand::Result, Operand::Tensor, Operand::Allocation]
            }
            operation if operation == TensorOperation::Reduce as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::Register,
                Operand::Operator,
                Operand::Unsigned16List,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::IndexReduce as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::Operator,
                Operand::Unsigned16,
                Operand::Unsigned16,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::Contract as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::Tensor,
                Operand::ContractionAxes,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::Gather as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::Tensor,
                Operand::GatherAxes,
                Operand::Bits64List,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::Scatter as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::Tensor,
                Operand::Tensor,
                Operand::ScatterAxes,
                Operand::Operator,
                Operand::Allocation,
            ],
            operation if operation == TensorOperation::Load as u16 => {
                &[Operand::Result, Operand::Tensor, Operand::RegisterList]
            }
            operation if operation == TensorOperation::Extract as u16 => {
                &[Operand::Result, Operand::Tensor, Operand::RegisterList]
            }
            operation if operation == TensorOperation::Store as u16 => {
                &[Operand::Tensor, Operand::RegisterList, Operand::Register]
            }
            operation if operation == TensorOperation::Fill as u16 => {
                &[Operand::Tensor, Operand::Register]
            }
            operation if operation == TensorOperation::Copy as u16 => {
                &[Operand::Tensor, Operand::Tensor]
            }
            operation if operation == TensorOperation::View as u16 => &[
                Operand::ResultRange,
                Operand::Tensor,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::Layout,
            ],
            operation if operation == TensorOperation::Convolution as u16 => &[
                Operand::Result,
                Operand::Tensor,
                Operand::Tensor,
                Operand::ConvolutionAxes,
                Operand::Window,
                Operand::ConvolutionGroups,
                Operand::Allocation,
            ],
            _ => return None,
        };

        Some(InstructionLayout::new(layout))
    }
}
