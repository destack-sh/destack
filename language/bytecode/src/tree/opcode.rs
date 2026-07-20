use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    AllocationInitialization, AllocationOperation, AtomicOperation, BooleanOperation,
    CastOperation, Comparison, FloatOperation, InstructionLayout, IntegerOperation,
    MemoryOperation, Operand, ReferenceKind, Scalar, ScalarCheck, Space, TensorOperation,
    VectorOperation,
};

const OPCODE_MAX: u16 = 0x0fff;

const FIXED_END: u16 = Opcode::CONSTANT_UINT128.0;
const CONSTANT_BASE: u16 = 80;
const CONSTANT_END: u16 = CONSTANT_BASE + SCALAR_COUNT;
const BOOLEAN_BASE: u16 = 96;
const BOOLEAN_END: u16 = BOOLEAN_BASE + BOOLEAN_OPERATION_COUNT;
const INTEGER_BASE: u16 = 112;
const INTEGER_END: u16 = INTEGER_BASE + INTEGER_OPERATION_COUNT * INTEGER_COUNT;
const INTEGER128_BASE: u16 = 416;
const INTEGER128_END: u16 = INTEGER128_BASE + INTEGER_OPERATION_COUNT * 2;
const FLOAT_BASE: u16 = 496;
const FLOAT_END: u16 = FLOAT_BASE + FLOAT_OPERATION_COUNT * FLOAT_COUNT;
const CAST_INTEGER_BASE: u16 = 656;
const CAST_INTEGER_END: u16 =
    CAST_INTEGER_BASE + INTEGER_CAST_OPERATION_COUNT * INTEGER_COUNT * INTEGER_COUNT;
const CAST_FLOAT_TO_INT_BASE: u16 = CAST_INTEGER_END;
const CAST_FLOAT_TO_INT_END: u16 =
    CAST_FLOAT_TO_INT_BASE + FLOAT_TO_INT_OPERATION_COUNT * FLOAT_COUNT * INTEGER_COUNT;
const CAST_INT_TO_FLOAT_BASE: u16 = CAST_FLOAT_TO_INT_END;
const CAST_INT_TO_FLOAT_END: u16 = CAST_INT_TO_FLOAT_BASE + INTEGER_COUNT * FLOAT_COUNT;
const CAST_FLOAT_BASE: u16 = CAST_INT_TO_FLOAT_END;
const CAST_FLOAT_END: u16 = CAST_FLOAT_BASE + FLOAT_COUNT * FLOAT_COUNT;
const CAST_BIT_BASE: u16 = CAST_FLOAT_END;
const CAST_BIT_END: u16 = CAST_BIT_BASE + SCALAR_COUNT * SCALAR_COUNT;
const MEMORY_BASE: u16 = 1200;
const MEMORY_END: u16 = MEMORY_BASE + MEMORY_OPERATION_COUNT * SCALAR_COUNT;
const ATOMIC_BASE: u16 = 1296;
const ATOMIC_END: u16 = ATOMIC_BASE + ATOMIC_OPERATION_COUNT * SCALAR_COUNT;
const ALLOCATION_BASE: u16 = 1488;
const ALLOCATION_END: u16 = ALLOCATION_BASE + 32;
const CHECK_BASE: u16 = 1520;
const CHECK_END: u16 = CHECK_BASE + CHECK_COUNT * SCALAR_COUNT;
const BRANCH_BASE: u16 = 1632;
const BRANCH_END: u16 = BRANCH_BASE + COMPARISON_COUNT * SCALAR_COUNT;
const VECTOR_BASE: u16 = 1712;
const VECTOR_END: u16 = VECTOR_BASE + VECTOR_OPERATION_COUNT;
const TENSOR_BASE: u16 = 1728;
const TENSOR_END: u16 = TENSOR_BASE + TENSOR_OPERATION_COUNT;

const ALLOCATION_OWNERSHIP_SHIFT: u16 = 1;
const ALLOCATION_OPERATION_SHIFT: u16 = 2;
const ALLOCATION_INITIALIZATION_SHIFT: u16 = 3;
const ALLOCATION_FALLIBILITY_SHIFT: u16 = 4;

const SCALAR_COUNT: u16 = 13;
const INTEGER_COUNT: u16 = 8;
const FLOAT_COUNT: u16 = 4;
const BOOLEAN_OPERATION_COUNT: u16 = BooleanOperation::Not as u16 + 1;
const INTEGER_OPERATION_COUNT: u16 = IntegerOperation::SubtractSaturating as u16 + 1;
const FLOAT_OPERATION_COUNT: u16 = FloatOperation::RoundTiesEven as u16 + 1;
const INTEGER_CAST_OPERATION_COUNT: u16 = CastOperation::ZeroExtend as u16 + 1;
const FLOAT_TO_INT_OPERATION_COUNT: u16 =
    CastOperation::FloatToIntSaturating as u16 - CastOperation::FloatToInt as u16 + 1;
const MEMORY_OPERATION_COUNT: u16 = MemoryOperation::Store as u16 + 1;
const ATOMIC_OPERATION_COUNT: u16 = AtomicOperation::WaitTimed as u16 + 1;
const CHECK_COUNT: u16 = ScalarCheck::Range as u16 + 1;
const COMPARISON_COUNT: u16 = Comparison::GreaterEqual as u16 + 1;
const VECTOR_OPERATION_COUNT: u16 = VectorOperation::Store as u16 + 1;
const TENSOR_OPERATION_COUNT: u16 = TensorOperation::Convolution as u16 + 1;

/// One stable exact bytecode operation code.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Opcode(u16);

impl Opcode {
    /// The reserved invalid opcode.
    pub const INVALID: Self = Self(0);
    /// Copy one register word.
    pub const MOVE: Self = Self(1);
    /// Select one of two register words.
    pub const SELECT: Self = Self(2);
    /// Materialize one immutable byte sequence.
    pub const CONSTANT_BYTES: Self = Self(3);
    /// Materialize one 128-bit integer constant.
    pub const CONSTANT_INT128: Self = Self(4);
    /// Materialize one null address.
    pub const CONSTANT_NULL: Self = Self(5);
    /// Materialize one linked runtime type id.
    pub const TYPE_ID: Self = Self(6);

    /// Materialize one linked global address.
    pub const GLOBAL_ADDRESS: Self = Self(7);
    /// Add one byte offset to an address.
    pub const ADDRESS_OFFSET: Self = Self(8);
    /// Add one scaled element offset to an address.
    pub const ADDRESS_ELEMENT: Self = Self(9);
    /// Compute the signed byte distance between two addresses.
    pub const ADDRESS_DISTANCE: Self = Self(10);
    /// Copy non-overlapping bytes.
    pub const MEMORY_COPY: Self = Self(11);
    /// Move possibly overlapping bytes.
    pub const MEMORY_MOVE: Self = Self(12);
    /// Fill one byte range.
    pub const MEMORY_FILL: Self = Self(13);
    /// Compare two byte ranges.
    pub const MEMORY_COMPARE: Self = Self(14);
    /// Prefetch memory for reading.
    pub const MEMORY_PREFETCH_READ: Self = Self(15);
    /// Prefetch memory for writing.
    pub const MEMORY_PREFETCH_WRITE: Self = Self(16);

    /// Materialize one frame slot address.
    pub const FRAME_ADDRESS: Self = Self(17);
    /// Complete one uninitialized allocation.
    pub const NEW_COMPLETE: Self = Self(18);
    /// Pin one managed allocation.
    pub const PIN: Self = Self(19);
    /// Unpin one managed allocation.
    pub const UNPIN: Self = Self(20);
    /// Release one unique allocation.
    pub const FREE: Self = Self(21);
    /// Destroy one initialized value.
    pub const DROP: Self = Self(22);
    /// Publish one managed reference write.
    pub const BARRIER_WRITE: Self = Self(23);

    /// Materialize one function pointer.
    pub const FUNCTION_ADDRESS: Self = Self(24);
    /// Bind one payload to its dynamic dispatch table.
    pub const DYNAMIC_BIND: Self = Self(25);
    /// Read one dynamic value's concrete runtime type.
    pub const DYNAMIC_TYPE: Self = Self(26);
    /// Call one direct function without a local unwind edge.
    pub const CALL: Self = Self(27);
    /// Call one function value without a local unwind edge.
    pub const CALL_INDIRECT: Self = Self(28);
    /// Call one virtual slot without a local unwind edge.
    pub const CALL_VIRTUAL: Self = Self(29);
    /// Call one dynamic slot without a local unwind edge.
    pub const CALL_DYNAMIC: Self = Self(30);
    /// Invoke one direct function with explicit normal and unwind edges.
    pub const INVOKE: Self = Self(31);
    /// Invoke one function value with explicit normal and unwind edges.
    pub const INVOKE_INDIRECT: Self = Self(32);
    /// Invoke one virtual slot with explicit normal and unwind edges.
    pub const INVOKE_VIRTUAL: Self = Self(33);
    /// Invoke one dynamic slot with explicit normal and unwind edges.
    pub const INVOKE_DYNAMIC: Self = Self(34);
    /// Tail call one direct function.
    pub const TAIL_CALL: Self = Self(35);
    /// Tail call one function value.
    pub const TAIL_CALL_INDIRECT: Self = Self(36);
    /// Tail call one virtual slot.
    pub const TAIL_CALL_VIRTUAL: Self = Self(37);
    /// Tail call one dynamic slot.
    pub const TAIL_CALL_DYNAMIC: Self = Self(38);

    /// Jump to one relative instruction offset.
    pub const JUMP: Self = Self(39);
    /// Branch on one boolean word.
    pub const BRANCH: Self = Self(40);
    /// Select one relative instruction offset from inline integer cases.
    pub const SWITCH: Self = Self(41);
    /// Suspend and yield one value.
    pub const YIELD: Self = Self(42);
    /// Return from the current function.
    pub const RETURN: Self = Self(43);
    /// Terminate execution with one trap.
    pub const TRAP: Self = Self(44);
    /// Mark one impossible control flow path.
    pub const UNREACHABLE: Self = Self(45);

    /// Materialize the active panic value.
    pub const CATCH: Self = Self(46);
    /// Begin panic unwinding without a payload.
    pub const PANIC: Self = Self(47);
    /// Begin panic unwinding with a payload.
    pub const PANIC_VALUE: Self = Self(48);
    /// Continue the active panic unwind.
    pub const UNWIND_RESUME: Self = Self(49);

    /// Stop at one debugger breakpoint.
    pub const BREAKPOINT: Self = Self(50);
    /// Publish one explicit managed safepoint.
    pub const SAFEPOINT: Self = Self(51);

    /// Establish one atomic fence.
    pub const ATOMIC_FENCE: Self = Self(52);
    /// Wake waiters at one atomic address.
    pub const ATOMIC_WAKE: Self = Self(53);

    /// Require one non-null address.
    pub const CHECK_NULL: Self = Self(54);
    /// Require one exact runtime type.
    pub const CHECK_TYPE: Self = Self(55);
    /// Require one runtime subtype relation.
    pub const CHECK_SUBTYPE: Self = Self(56);
    /// Convert one address to an unsigned integer.
    pub const CAST_ADDRESS_TO_INT: Self = Self(57);
    /// Convert one unsigned integer to an address.
    pub const CAST_INT_TO_ADDRESS: Self = Self(58);
    /// Increment one explicit profile counter.
    pub const PROFILE_INCREMENT: Self = Self(59);
    /// Record one explicit profile sample.
    pub const PROFILE_SAMPLE: Self = Self(60);
    /// Compare two register words for exact bit equality.
    pub const WORD_EQUAL: Self = Self(61);
    /// Wake every waiter at one atomic address.
    pub const ATOMIC_WAKE_ALL: Self = Self(62);
    /// Copy one logical value spanning multiple register words.
    pub const MOVE_WORDS: Self = Self(63);
    /// Bind one captured environment to a function.
    pub const FUNCTION_BIND: Self = Self(64);
    /// Read the function pointer from one function value.
    pub const FUNCTION_POINTER: Self = Self(65);
    /// Read the captured environment from one function value.
    pub const FUNCTION_ENVIRONMENT: Self = Self(66);
    /// Read the current function's captured environment.
    pub const FUNCTION_ENVIRONMENT_CURRENT: Self = Self(67);
    /// Form one slice over a contiguous subrange.
    pub const SLICE_VIEW: Self = Self(68);
    /// Read one slice's element count.
    pub const SLICE_LENGTH: Self = Self(69);
    /// Read the erased payload from one dynamic value.
    pub const DYNAMIC_PAYLOAD: Self = Self(70);
    /// Call one bare function pointer without a local unwind edge.
    pub const CALL_FUNCTION_POINTER: Self = Self(71);
    /// Invoke one bare function pointer with explicit normal and unwind edges.
    pub const INVOKE_FUNCTION_POINTER: Self = Self(72);
    /// Tail call one bare function pointer.
    pub const TAIL_CALL_FUNCTION_POINTER: Self = Self(73);
    /// Materialize one unsigned 128-bit integer constant.
    pub const CONSTANT_UINT128: Self = Self(74);

    /// Load one reference word.
    pub const LOAD_REFERENCE: Self = Self(1248);
    /// Store one reference word.
    pub const STORE_REFERENCE: Self = Self(1249);

    /// Create one exact scalar constant opcode.
    pub const fn constant(scalar: Scalar) -> Self {
        Self(CONSTANT_BASE + scalar.code() as u16)
    }

    /// Create one exact boolean opcode.
    pub const fn boolean(operation: BooleanOperation) -> Self {
        Self(BOOLEAN_BASE + operation as u16)
    }

    /// Create one exact scalar integer opcode.
    pub const fn integer(operation: IntegerOperation, scalar: Scalar) -> Option<Self> {
        match scalar.integer_index() {
            Some(scalar) => Some(Self(
                INTEGER_BASE + operation as u16 * INTEGER_COUNT + scalar,
            )),
            None => None,
        }
    }

    /// Create one exact 128-bit integer opcode.
    pub const fn integer128(operation: IntegerOperation, is_signed: bool) -> Self {
        Self(INTEGER128_BASE + operation as u16 * 2 + is_signed as u16)
    }

    /// Create one exact scalar floating-point opcode.
    pub const fn float(operation: FloatOperation, scalar: Scalar) -> Option<Self> {
        match scalar.float_index() {
            Some(scalar) => Some(Self(FLOAT_BASE + operation as u16 * FLOAT_COUNT + scalar)),
            None => None,
        }
    }

    /// Create one exact scalar cast opcode.
    pub const fn cast(operation: CastOperation, source: Scalar, target: Scalar) -> Option<Self> {
        let source_integer = source.integer_index();
        let target_integer = target.integer_index();
        let source_float = source.float_index();
        let target_float = target.float_index();

        match operation {
            CastOperation::Truncate
            | CastOperation::Saturate
            | CastOperation::SignExtend
            | CastOperation::ZeroExtend => match (source_integer, target_integer) {
                (Some(source), Some(target)) if operation.supports(source, target) => {
                    let operation = operation as u16;
                    let conversion = source * INTEGER_COUNT + target;

                    Some(Self(
                        CAST_INTEGER_BASE + operation * INTEGER_COUNT * INTEGER_COUNT + conversion,
                    ))
                }
                _ => None,
            },
            CastOperation::FloatToInt | CastOperation::FloatToIntSaturating => {
                match (source_float, target_integer) {
                    (Some(source), Some(target)) => {
                        let operation = operation as u16 - CastOperation::FloatToInt as u16;
                        let conversion = source * INTEGER_COUNT + target;

                        Some(Self(
                            CAST_FLOAT_TO_INT_BASE
                                + operation * FLOAT_COUNT * INTEGER_COUNT
                                + conversion,
                        ))
                    }
                    _ => None,
                }
            }
            CastOperation::IntToFloat => match (source_integer, target_float) {
                (Some(source), Some(target)) => {
                    Some(Self(CAST_INT_TO_FLOAT_BASE + source * FLOAT_COUNT + target))
                }
                _ => None,
            },
            CastOperation::FloatConvert => match (source_float, target_float) {
                (Some(source), Some(target)) if source != target => {
                    Some(Self(CAST_FLOAT_BASE + source * FLOAT_COUNT + target))
                }
                _ => None,
            },
            CastOperation::Bit
                if source.code() != target.code() && source.bit_width() == target.bit_width() =>
            {
                let conversion = source.code() as u16 * SCALAR_COUNT + target.code() as u16;

                Some(Self(CAST_BIT_BASE + conversion))
            }
            CastOperation::Bit => None,
        }
    }

    /// Create one exact scalar memory opcode.
    pub const fn memory(operation: MemoryOperation, scalar: Scalar) -> Self {
        Self(MEMORY_BASE + operation as u16 * SCALAR_COUNT + scalar.code() as u16)
    }

    /// Create one exact scalar atomic opcode.
    pub const fn atomic(operation: AtomicOperation, scalar: Scalar) -> Option<Self> {
        if operation.supports(scalar) {
            Some(Self(
                ATOMIC_BASE + operation as u16 * SCALAR_COUNT + scalar.code() as u16,
            ))
        } else {
            None
        }
    }

    /// Create one exact allocation opcode.
    pub const fn allocation(
        space: Space,
        ownership: ReferenceKind,
        operation: AllocationOperation,
        initialization: AllocationInitialization,
        is_fallible: bool,
    ) -> Option<Self> {
        let space = match space {
            Space::LOCAL => 0,
            Space::SHARED => 1,
            _ => return None,
        };
        let ownership = match ownership {
            ReferenceKind::MANAGED => 0,
            ReferenceKind::UNIQUE => 1,
            _ => return None,
        };
        let bits = space
            | (ownership << ALLOCATION_OWNERSHIP_SHIFT)
            | ((operation as u16) << ALLOCATION_OPERATION_SHIFT)
            | ((initialization as u16) << ALLOCATION_INITIALIZATION_SHIFT)
            | ((is_fallible as u16) << ALLOCATION_FALLIBILITY_SHIFT);

        Some(Self(ALLOCATION_BASE + bits))
    }

    /// Create one exact scalar runtime check opcode.
    pub const fn check(check: ScalarCheck, scalar: Scalar) -> Option<Self> {
        if check.supports(scalar) {
            Some(Self(
                CHECK_BASE + check as u16 * SCALAR_COUNT + scalar.code() as u16,
            ))
        } else {
            None
        }
    }

    /// Create one exact fused scalar branch opcode.
    pub const fn branch(comparison: Comparison, scalar: Scalar) -> Option<Self> {
        if comparison.supports(scalar) {
            Some(Self(
                BRANCH_BASE + comparison as u16 * SCALAR_COUNT + scalar.code() as u16,
            ))
        } else {
            None
        }
    }

    /// Create one vector opcode.
    pub const fn vector(operation: VectorOperation) -> Self {
        Self(VECTOR_BASE + operation as u16)
    }

    /// Create one tensor opcode.
    pub const fn tensor(operation: TensorOperation) -> Self {
        Self(TENSOR_BASE + operation as u16)
    }

    /// Create one opcode from its exact stable code.
    pub const fn from_code(code: u16) -> Self {
        Self(code)
    }

    /// Parse one fixed opcode by its canonical text name.
    pub fn parse_fixed(name: &str) -> Option<Self> {
        (1..=FIXED_END)
            .map(Self)
            .chain([Self::LOAD_REFERENCE, Self::STORE_REFERENCE])
            .find(|opcode| opcode.fixed_name() == Some(name))
    }

    /// Return this fixed opcode's canonical text name.
    pub const fn fixed_name(self) -> Option<&'static str> {
        match self {
            Self::MOVE => Some("move"),
            Self::SELECT => Some("select"),
            Self::CONSTANT_BYTES => Some("constant.bytes"),
            Self::CONSTANT_INT128 => Some("constant.int128"),
            Self::CONSTANT_UINT128 => Some("constant.uint128"),
            Self::CONSTANT_NULL => Some("constant.null"),
            Self::TYPE_ID => Some("type.id"),
            Self::GLOBAL_ADDRESS => Some("global.address"),
            Self::ADDRESS_OFFSET => Some("address.offset"),
            Self::ADDRESS_ELEMENT => Some("address.element"),
            Self::ADDRESS_DISTANCE => Some("address.distance"),
            Self::MEMORY_COPY => Some("memory.copy"),
            Self::MEMORY_MOVE => Some("memory.move"),
            Self::MEMORY_FILL => Some("memory.fill"),
            Self::MEMORY_COMPARE => Some("memory.compare"),
            Self::MEMORY_PREFETCH_READ => Some("memory.prefetch.read"),
            Self::MEMORY_PREFETCH_WRITE => Some("memory.prefetch.write"),
            Self::FRAME_ADDRESS => Some("frame.address"),
            Self::NEW_COMPLETE => Some("new.complete"),
            Self::PIN => Some("pin"),
            Self::UNPIN => Some("unpin"),
            Self::FREE => Some("free"),
            Self::DROP => Some("drop"),
            Self::BARRIER_WRITE => Some("barrier.write"),
            Self::FUNCTION_ADDRESS => Some("function.address"),
            Self::FUNCTION_BIND => Some("function.bind"),
            Self::FUNCTION_POINTER => Some("function.pointer"),
            Self::FUNCTION_ENVIRONMENT => Some("function.environment"),
            Self::FUNCTION_ENVIRONMENT_CURRENT => Some("function.environment.current"),
            Self::SLICE_VIEW => Some("slice.view"),
            Self::SLICE_LENGTH => Some("slice.length"),
            Self::DYNAMIC_BIND => Some("dynamic.bind"),
            Self::DYNAMIC_PAYLOAD => Some("dynamic.payload"),
            Self::DYNAMIC_TYPE => Some("dynamic.type"),
            Self::CALL => Some("call"),
            Self::CALL_INDIRECT => Some("call.indirect"),
            Self::CALL_FUNCTION_POINTER => Some("call.indirect"),
            Self::CALL_VIRTUAL => Some("call.virtual"),
            Self::CALL_DYNAMIC => Some("call.dynamic"),
            Self::INVOKE => Some("invoke"),
            Self::INVOKE_INDIRECT => Some("invoke.indirect"),
            Self::INVOKE_FUNCTION_POINTER => Some("invoke.indirect"),
            Self::INVOKE_VIRTUAL => Some("invoke.virtual"),
            Self::INVOKE_DYNAMIC => Some("invoke.dynamic"),
            Self::TAIL_CALL => Some("tail.call"),
            Self::TAIL_CALL_INDIRECT => Some("tail.call.indirect"),
            Self::TAIL_CALL_FUNCTION_POINTER => Some("tail.call.indirect"),
            Self::TAIL_CALL_VIRTUAL => Some("tail.call.virtual"),
            Self::TAIL_CALL_DYNAMIC => Some("tail.call.dynamic"),
            Self::JUMP => Some("jump"),
            Self::BRANCH => Some("branch"),
            Self::SWITCH => Some("switch"),
            Self::YIELD => Some("yield"),
            Self::RETURN => Some("return"),
            Self::TRAP => Some("trap"),
            Self::UNREACHABLE => Some("unreachable"),
            Self::CATCH => Some("catch"),
            Self::PANIC => Some("panic"),
            Self::PANIC_VALUE => Some("panic"),
            Self::UNWIND_RESUME => Some("unwind.resume"),
            Self::BREAKPOINT => Some("breakpoint"),
            Self::SAFEPOINT => Some("safepoint"),
            Self::ATOMIC_FENCE => Some("atomic.fence"),
            Self::ATOMIC_WAKE => Some("atomic.wake"),
            Self::CHECK_NULL => Some("check.null"),
            Self::CHECK_TYPE => Some("check.type"),
            Self::CHECK_SUBTYPE => Some("check.subtype"),
            Self::CAST_ADDRESS_TO_INT => Some("cast.addressToInt"),
            Self::CAST_INT_TO_ADDRESS => Some("cast.intToAddress"),
            Self::PROFILE_INCREMENT => Some("profile.increment"),
            Self::PROFILE_SAMPLE => Some("profile.sample"),
            Self::WORD_EQUAL => Some("word.eq"),
            Self::ATOMIC_WAKE_ALL => Some("atomic.wakeAll"),
            Self::MOVE_WORDS => Some("move"),
            Self::LOAD_REFERENCE => Some("load.reference"),
            Self::STORE_REFERENCE => Some("store.reference"),
            _ => None,
        }
    }

    /// Decode one scalar constant opcode.
    pub const fn constant_scalar(self) -> Option<Scalar> {
        if self.0 >= CONSTANT_BASE && self.0 < CONSTANT_END {
            Scalar::from_code((self.0 - CONSTANT_BASE) as u8)
        } else {
            None
        }
    }

    /// Decode one scalar boolean opcode.
    pub const fn boolean_operation(self) -> Option<BooleanOperation> {
        if self.0 >= BOOLEAN_BASE && self.0 < BOOLEAN_END {
            BooleanOperation::from_code((self.0 - BOOLEAN_BASE) as u8)
        } else {
            None
        }
    }

    /// Decode one scalar integer opcode.
    pub const fn integer_operation(self) -> Option<(IntegerOperation, Scalar)> {
        if self.0 < INTEGER_BASE || self.0 >= INTEGER_END {
            return None;
        }
        let code = self.0 - INTEGER_BASE;
        let operation = IntegerOperation::from_code((code / INTEGER_COUNT) as u8);
        let scalar = Scalar::from_code((code % INTEGER_COUNT + 1) as u8);

        match (operation, scalar) {
            (Some(operation), Some(scalar)) => Some((operation, scalar)),
            _ => None,
        }
    }

    /// Decode one 128-bit integer opcode.
    pub const fn integer128_operation(self) -> Option<(IntegerOperation, bool)> {
        if self.0 < INTEGER128_BASE || self.0 >= INTEGER128_END {
            return None;
        }
        let code = self.0 - INTEGER128_BASE;
        let operation = IntegerOperation::from_code((code / 2) as u8);

        match operation {
            Some(operation) => Some((operation, code % 2 == 1)),
            None => None,
        }
    }

    /// Decode one scalar floating-point opcode.
    pub const fn float_operation(self) -> Option<(FloatOperation, Scalar)> {
        if self.0 < FLOAT_BASE || self.0 >= FLOAT_END {
            return None;
        }
        let code = self.0 - FLOAT_BASE;
        let operation = FloatOperation::from_code((code / FLOAT_COUNT) as u8);
        let scalar = Scalar::from_code((code % FLOAT_COUNT + 9) as u8);

        match (operation, scalar) {
            (Some(operation), Some(scalar)) => Some((operation, scalar)),
            _ => None,
        }
    }

    /// Decode one scalar cast opcode.
    pub const fn cast_operation(self) -> Option<(CastOperation, Scalar, Scalar)> {
        let code = self.0;

        if code >= CAST_INTEGER_BASE && code < CAST_INTEGER_END {
            let code = code - CAST_INTEGER_BASE;
            let operation =
                CastOperation::from_code((code / (INTEGER_COUNT * INTEGER_COUNT)) as u8);
            let conversion = code % (INTEGER_COUNT * INTEGER_COUNT);
            let source = Scalar::from_code((conversion / INTEGER_COUNT + 1) as u8);
            let target = Scalar::from_code((conversion % INTEGER_COUNT + 1) as u8);

            return match (operation, source, target) {
                (Some(operation), Some(source), Some(target)) => Some((operation, source, target)),
                _ => None,
            };
        }

        if code >= CAST_FLOAT_TO_INT_BASE && code < CAST_FLOAT_TO_INT_END {
            let code = code - CAST_FLOAT_TO_INT_BASE;
            let operation = CastOperation::from_code(
                (code / (FLOAT_COUNT * INTEGER_COUNT)) as u8 + CastOperation::FloatToInt as u8,
            );
            let conversion = code % (FLOAT_COUNT * INTEGER_COUNT);
            let source = Scalar::from_code((conversion / INTEGER_COUNT + 9) as u8);
            let target = Scalar::from_code((conversion % INTEGER_COUNT + 1) as u8);

            return match (operation, source, target) {
                (Some(operation), Some(source), Some(target)) => Some((operation, source, target)),
                _ => None,
            };
        }

        if code >= CAST_INT_TO_FLOAT_BASE && code < CAST_INT_TO_FLOAT_END {
            let conversion = code - CAST_INT_TO_FLOAT_BASE;
            let source = Scalar::from_code((conversion / FLOAT_COUNT + 1) as u8);
            let target = Scalar::from_code((conversion % FLOAT_COUNT + 9) as u8);

            return match (source, target) {
                (Some(source), Some(target)) => Some((CastOperation::IntToFloat, source, target)),
                _ => None,
            };
        }

        if code >= CAST_FLOAT_BASE && code < CAST_FLOAT_END {
            let conversion = code - CAST_FLOAT_BASE;
            let source = Scalar::from_code((conversion / FLOAT_COUNT + 9) as u8);
            let target = Scalar::from_code((conversion % FLOAT_COUNT + 9) as u8);

            return match (source, target) {
                (Some(source), Some(target)) if source.code() != target.code() => {
                    Some((CastOperation::FloatConvert, source, target))
                }
                _ => None,
            };
        }

        if code >= CAST_BIT_BASE && code < CAST_BIT_END {
            let conversion = code - CAST_BIT_BASE;
            let source = Scalar::from_code((conversion / SCALAR_COUNT) as u8);
            let target = Scalar::from_code((conversion % SCALAR_COUNT) as u8);

            return match (source, target) {
                (Some(source), Some(target))
                    if source.code() != target.code()
                        && source.bit_width() == target.bit_width() =>
                {
                    Some((CastOperation::Bit, source, target))
                }
                _ => None,
            };
        }

        None
    }

    /// Decode one scalar memory opcode.
    pub const fn memory_operation(self) -> Option<(MemoryOperation, Scalar)> {
        if self.0 < MEMORY_BASE || self.0 >= MEMORY_END {
            return None;
        }
        let code = self.0 - MEMORY_BASE;
        let operation = if code / SCALAR_COUNT == 0 {
            MemoryOperation::Load
        } else {
            MemoryOperation::Store
        };
        let scalar = Scalar::from_code((code % SCALAR_COUNT) as u8);

        match scalar {
            Some(scalar) => Some((operation, scalar)),
            None => None,
        }
    }

    /// Decode one scalar atomic opcode.
    pub const fn atomic_operation(self) -> Option<(AtomicOperation, Scalar)> {
        if self.0 < ATOMIC_BASE || self.0 >= ATOMIC_END {
            return None;
        }
        let code = self.0 - ATOMIC_BASE;
        let operation = AtomicOperation::from_code((code / SCALAR_COUNT) as u8);
        let scalar = Scalar::from_code((code % SCALAR_COUNT) as u8);

        match (operation, scalar) {
            (Some(operation), Some(scalar)) if operation.supports(scalar) => {
                Some((operation, scalar))
            }
            _ => None,
        }
    }

    /// Decode one allocation opcode.
    pub const fn allocation_operation(
        self,
    ) -> Option<(
        Space,
        ReferenceKind,
        AllocationOperation,
        AllocationInitialization,
        bool,
    )> {
        if self.0 < ALLOCATION_BASE || self.0 >= ALLOCATION_END {
            return None;
        }

        let bits = self.0 - ALLOCATION_BASE;
        let space = if bits & 1 == 0 {
            Space::LOCAL
        } else {
            Space::SHARED
        };
        let ownership = if bits & (1 << ALLOCATION_OWNERSHIP_SHIFT) == 0 {
            ReferenceKind::MANAGED
        } else {
            ReferenceKind::UNIQUE
        };
        let operation = if bits & (1 << ALLOCATION_OPERATION_SHIFT) == 0 {
            AllocationOperation::Value
        } else {
            AllocationOperation::Slice
        };
        let initialization = if bits & (1 << ALLOCATION_INITIALIZATION_SHIFT) == 0 {
            AllocationInitialization::Zeroed
        } else {
            AllocationInitialization::Uninit
        };
        let is_fallible = bits & (1 << ALLOCATION_FALLIBILITY_SHIFT) != 0;

        Some((space, ownership, operation, initialization, is_fallible))
    }

    /// Decode one scalar runtime check opcode.
    pub const fn scalar_check(self) -> Option<(ScalarCheck, Scalar)> {
        if self.0 < CHECK_BASE || self.0 >= CHECK_END {
            return None;
        }
        let code = self.0 - CHECK_BASE;
        let check = ScalarCheck::from_code((code / SCALAR_COUNT) as u8);
        let scalar = Scalar::from_code((code % SCALAR_COUNT) as u8);

        match (check, scalar) {
            (Some(check), Some(scalar)) if check.supports(scalar) => Some((check, scalar)),
            _ => None,
        }
    }

    /// Decode one fused scalar branch opcode.
    pub const fn comparison(self) -> Option<(Comparison, Scalar)> {
        if self.0 < BRANCH_BASE || self.0 >= BRANCH_END {
            return None;
        }
        let code = self.0 - BRANCH_BASE;
        let comparison = Comparison::from_code((code / SCALAR_COUNT) as u8);
        let scalar = Scalar::from_code((code % SCALAR_COUNT) as u8);

        match (comparison, scalar) {
            (Some(comparison), Some(scalar)) if comparison.supports(scalar) => {
                Some((comparison, scalar))
            }
            _ => None,
        }
    }

    /// Decode one vector opcode.
    pub const fn vector_operation(self) -> Option<VectorOperation> {
        if self.0 >= VECTOR_BASE && self.0 < VECTOR_END {
            VectorOperation::from_code((self.0 - VECTOR_BASE) as u8)
        } else {
            None
        }
    }

    /// Decode one tensor opcode.
    pub const fn tensor_operation(self) -> Option<TensorOperation> {
        if self.0 >= TENSOR_BASE && self.0 < TENSOR_END {
            TensorOperation::from_code((self.0 - TENSOR_BASE) as u8)
        } else {
            None
        }
    }

    /// Return this opcode's exact binary operand layout.
    pub fn layout(self) -> Option<InstructionLayout> {
        if !self.is_defined() {
            return None;
        }

        let code = self.0;
        if code <= FIXED_END {
            Self::fixed_layout(self)
        } else if code >= CONSTANT_BASE && code < CONSTANT_END {
            Some(InstructionLayout::new(&[Operand::Result, Operand::Bits64]))
        } else if code >= BOOLEAN_BASE && code < BOOLEAN_END {
            Self::boolean_layout(code)
        } else if code >= INTEGER_BASE && code < INTEGER_END {
            Self::integer_layout(code)
        } else if code >= INTEGER128_BASE && code < INTEGER128_END {
            Self::integer128_layout(code)
        } else if code >= FLOAT_BASE && code < FLOAT_END {
            Self::float_layout(code)
        } else if code >= CAST_INTEGER_BASE && code < CAST_BIT_END {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
            ]))
        } else if code >= MEMORY_BASE && code < MEMORY_END {
            Self::memory_layout(code)
        } else if code == Self::LOAD_REFERENCE.0 {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::Reference,
            ]))
        } else if code == Self::STORE_REFERENCE.0 {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
            ]))
        } else if code >= ATOMIC_BASE && code < ATOMIC_END {
            Self::atomic_layout(code)
        } else if code >= ALLOCATION_BASE && code < ALLOCATION_END {
            Self::allocation_layout(code)
        } else if code >= CHECK_BASE && code < CHECK_END {
            Self::check_layout(code)
        } else if code >= BRANCH_BASE && code < BRANCH_END {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::Branch,
                Operand::Branch,
            ]))
        } else if code >= VECTOR_BASE && code < VECTOR_END {
            Self::vector_layout(code)
        } else if code >= TENSOR_BASE && code < TENSOR_END {
            Self::tensor_layout(code)
        } else {
            None
        }
    }

    /// Return whether this opcode is assigned by the bytecode ISA.
    pub const fn is_defined(self) -> bool {
        let code = self.0;

        (code > 0 && code <= FIXED_END)
            || (code >= CONSTANT_BASE && code < CONSTANT_END)
            || (code >= BOOLEAN_BASE && code < BOOLEAN_END)
            || (code >= INTEGER_BASE && code < INTEGER_END)
            || (code >= INTEGER128_BASE && code < INTEGER128_END)
            || (code >= FLOAT_BASE && code < FLOAT_END)
            || Self::is_integer_cast(code)
            || (code >= CAST_FLOAT_TO_INT_BASE && code < CAST_INT_TO_FLOAT_END)
            || Self::is_float_cast(code)
            || Self::is_bit_cast(code)
            || (code >= MEMORY_BASE && code < MEMORY_END)
            || code == Self::LOAD_REFERENCE.0
            || code == Self::STORE_REFERENCE.0
            || Self::is_atomic(code)
            || (code >= ALLOCATION_BASE && code < ALLOCATION_END)
            || Self::is_check(code)
            || Self::is_branch(code)
            || (code >= VECTOR_BASE && code < VECTOR_END)
            || (code >= TENSOR_BASE && code < TENSOR_END)
    }

    /// Return whether this instruction ends control flow without a successor.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::TAIL_CALL
                | Self::TAIL_CALL_INDIRECT
                | Self::TAIL_CALL_FUNCTION_POINTER
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

    /// Return whether one code names an equal-width scalar bit cast.
    const fn is_bit_cast(code: u16) -> bool {
        if code < CAST_BIT_BASE || code >= CAST_BIT_END {
            return false;
        }

        let conversion = code - CAST_BIT_BASE;
        let source = Scalar::from_code((conversion / SCALAR_COUNT) as u8);
        let target = Scalar::from_code((conversion % SCALAR_COUNT) as u8);

        match (source, target) {
            (Some(source), Some(target)) => {
                source.code() != target.code() && source.bit_width() == target.bit_width()
            }
            _ => false,
        }
    }

    /// Return whether one code names a valid integer representation conversion.
    const fn is_integer_cast(code: u16) -> bool {
        if code < CAST_INTEGER_BASE || code >= CAST_INTEGER_END {
            return false;
        }

        let code = code - CAST_INTEGER_BASE;
        let operation = code / (INTEGER_COUNT * INTEGER_COUNT);
        let conversion = code % (INTEGER_COUNT * INTEGER_COUNT);
        let source = conversion / INTEGER_COUNT;
        let target = conversion % INTEGER_COUNT;
        let operation = CastOperation::from_code(operation as u8);

        match operation {
            Some(operation) => operation.supports(source, target),
            None => false,
        }
    }

    /// Return whether one code names a non-identity floating-point conversion.
    const fn is_float_cast(code: u16) -> bool {
        if code < CAST_FLOAT_BASE || code >= CAST_FLOAT_END {
            return false;
        }

        let conversion = code - CAST_FLOAT_BASE;
        let source = conversion / FLOAT_COUNT;
        let target = conversion % FLOAT_COUNT;

        source != target
    }

    /// Return whether one code names an atomic operation over a supported scalar.
    const fn is_atomic(code: u16) -> bool {
        if code < ATOMIC_BASE || code >= ATOMIC_END {
            return false;
        }

        let code = code - ATOMIC_BASE;
        let operation = AtomicOperation::from_code((code / SCALAR_COUNT) as u8);
        let scalar = Scalar::from_code((code % SCALAR_COUNT) as u8);

        match (operation, scalar) {
            (Some(operation), Some(scalar)) => operation.supports(scalar),
            _ => false,
        }
    }

    /// Return whether one code names a runtime check over a supported scalar.
    const fn is_check(code: u16) -> bool {
        if code < CHECK_BASE || code >= CHECK_END {
            return false;
        }

        let code = code - CHECK_BASE;
        let operation = ScalarCheck::from_code((code / SCALAR_COUNT) as u8);
        let scalar = Scalar::from_code((code % SCALAR_COUNT) as u8);

        match (operation, scalar) {
            (Some(operation), Some(scalar)) => operation.supports(scalar),
            _ => false,
        }
    }

    /// Return whether one code names a fused branch over a supported scalar.
    const fn is_branch(code: u16) -> bool {
        if code < BRANCH_BASE || code >= BRANCH_END {
            return false;
        }

        let code = code - BRANCH_BASE;
        let operation = Comparison::from_code((code / SCALAR_COUNT) as u8);
        let scalar = Scalar::from_code((code % SCALAR_COUNT) as u8);

        match (operation, scalar) {
            (Some(operation), Some(scalar)) => operation.supports(scalar),
            _ => false,
        }
    }

    /// Return this opcode's exact stable code.
    pub const fn code(self) -> u16 {
        self.0
    }

    /// Return the operand layout for one fixed opcode.
    fn fixed_layout(opcode: Self) -> Option<InstructionLayout> {
        let layout = match opcode {
            Self::MOVE => &[Operand::Result, Operand::Register][..],
            Self::SELECT => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Register,
            ],
            Self::CONSTANT_BYTES => &[Operand::ResultRange, Operand::Constant],
            Self::CONSTANT_INT128 | Self::CONSTANT_UINT128 => {
                &[Operand::ResultRange, Operand::Bits128]
            }
            Self::CONSTANT_NULL => &[Operand::Result],
            Self::TYPE_ID => &[Operand::Result, Operand::Type],

            Self::GLOBAL_ADDRESS => &[Operand::Result, Operand::Global],
            Self::ADDRESS_OFFSET => &[Operand::Result, Operand::Register, Operand::Signed32],
            Self::ADDRESS_ELEMENT => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Unsigned32,
            ],
            Self::ADDRESS_DISTANCE => &[Operand::Result, Operand::Register, Operand::Register],
            Self::MEMORY_COPY | Self::MEMORY_MOVE | Self::MEMORY_FILL => {
                &[Operand::Register, Operand::Register, Operand::Register]
            }
            Self::MEMORY_COMPARE => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Register,
            ],
            Self::MEMORY_PREFETCH_READ | Self::MEMORY_PREFETCH_WRITE => &[Operand::Register],

            Self::FRAME_ADDRESS => &[Operand::Result, Operand::FrameSlot],
            Self::NEW_COMPLETE => &[Operand::ResultRange, Operand::RegisterRange],
            Self::PIN | Self::UNPIN | Self::FREE => &[Operand::Register],
            Self::DROP => &[Operand::Register, Operand::Type],
            Self::BARRIER_WRITE => &[Operand::Register, Operand::Register, Operand::Register],

            Self::FUNCTION_ADDRESS => &[Operand::Result, Operand::Function],
            Self::DYNAMIC_BIND => &[
                Operand::ResultRange,
                Operand::Register,
                Operand::Type,
                Operand::Type,
            ],
            Self::DYNAMIC_TYPE => &[Operand::Result, Operand::RegisterRange],
            Self::CALL => &[
                Operand::ResultRange,
                Operand::Function,
                Operand::RegisterRange,
            ],
            Self::CALL_INDIRECT => &[
                Operand::ResultRange,
                Operand::FunctionType,
                Operand::RegisterRange,
                Operand::RegisterRange,
            ],
            Self::CALL_VIRTUAL => &[
                Operand::ResultRange,
                Operand::FunctionType,
                Operand::Register,
                Operand::Unsigned16,
                Operand::RegisterRange,
            ],
            Self::CALL_DYNAMIC => &[
                Operand::ResultRange,
                Operand::FunctionType,
                Operand::RegisterRange,
                Operand::Unsigned16,
                Operand::RegisterRange,
            ],
            Self::INVOKE => &[
                Operand::ResultRange,
                Operand::Function,
                Operand::RegisterRange,
                Operand::Branch,
                Operand::Branch,
            ],
            Self::INVOKE_INDIRECT => &[
                Operand::ResultRange,
                Operand::FunctionType,
                Operand::RegisterRange,
                Operand::RegisterRange,
                Operand::Branch,
                Operand::Branch,
            ],
            Self::INVOKE_VIRTUAL => &[
                Operand::ResultRange,
                Operand::FunctionType,
                Operand::Register,
                Operand::Unsigned16,
                Operand::RegisterRange,
                Operand::Branch,
                Operand::Branch,
            ],
            Self::INVOKE_DYNAMIC => &[
                Operand::ResultRange,
                Operand::FunctionType,
                Operand::RegisterRange,
                Operand::Unsigned16,
                Operand::RegisterRange,
                Operand::Branch,
                Operand::Branch,
            ],
            Self::TAIL_CALL => &[Operand::Function, Operand::RegisterRange],
            Self::TAIL_CALL_INDIRECT => &[
                Operand::FunctionType,
                Operand::RegisterRange,
                Operand::RegisterRange,
            ],
            Self::TAIL_CALL_VIRTUAL => &[
                Operand::FunctionType,
                Operand::Register,
                Operand::Unsigned16,
                Operand::RegisterRange,
            ],
            Self::TAIL_CALL_DYNAMIC => &[
                Operand::FunctionType,
                Operand::RegisterRange,
                Operand::Unsigned16,
                Operand::RegisterRange,
            ],

            Self::JUMP => &[Operand::Branch],
            Self::BRANCH => &[Operand::Register, Operand::Branch, Operand::Branch],
            Self::SWITCH => &[Operand::Register, Operand::Switch, Operand::Branch],
            Self::YIELD => &[
                Operand::ResultRange,
                Operand::RegisterRange,
                Operand::Branch,
                Operand::Branch,
            ],
            Self::RETURN => &[Operand::RegisterRange],
            Self::TRAP => &[Operand::Unsigned16],
            Self::UNREACHABLE => &[],

            Self::CATCH => &[Operand::Result],
            Self::PANIC => &[],
            Self::PANIC_VALUE => &[Operand::Register],
            Self::UNWIND_RESUME => &[],

            Self::BREAKPOINT | Self::SAFEPOINT => &[],
            Self::ATOMIC_FENCE => &[Operand::FenceAccess],
            Self::ATOMIC_WAKE => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Unsigned16,
            ],

            Self::CHECK_NULL => &[Operand::Register, Operand::Branch],
            Self::CHECK_TYPE | Self::CHECK_SUBTYPE => {
                &[Operand::Register, Operand::Type, Operand::Branch]
            }
            Self::CAST_ADDRESS_TO_INT | Self::CAST_INT_TO_ADDRESS => {
                &[Operand::Result, Operand::Register]
            }
            Self::PROFILE_INCREMENT => &[Operand::Counter],
            Self::PROFILE_SAMPLE => &[Operand::Counter, Operand::Register],
            Self::WORD_EQUAL => &[Operand::Result, Operand::Register, Operand::Register],
            Self::ATOMIC_WAKE_ALL => &[Operand::Result, Operand::Register, Operand::Unsigned16],
            Self::MOVE_WORDS => &[Operand::ResultRange, Operand::RegisterRange],
            Self::FUNCTION_BIND => &[Operand::ResultRange, Operand::Function, Operand::Register],
            Self::FUNCTION_POINTER => &[Operand::Result, Operand::RegisterRange],
            Self::FUNCTION_ENVIRONMENT => {
                &[Operand::Result, Operand::ValueType, Operand::RegisterRange]
            }
            Self::FUNCTION_ENVIRONMENT_CURRENT => &[Operand::Result],
            Self::SLICE_VIEW => &[
                Operand::ResultRange,
                Operand::RegisterRange,
                Operand::Register,
                Operand::Register,
            ],
            Self::SLICE_LENGTH | Self::DYNAMIC_PAYLOAD => {
                &[Operand::Result, Operand::RegisterRange]
            }
            Self::CALL_FUNCTION_POINTER => &[
                Operand::ResultRange,
                Operand::FunctionType,
                Operand::Register,
                Operand::RegisterRange,
            ],
            Self::INVOKE_FUNCTION_POINTER => &[
                Operand::ResultRange,
                Operand::FunctionType,
                Operand::Register,
                Operand::RegisterRange,
                Operand::Branch,
                Operand::Branch,
            ],
            Self::TAIL_CALL_FUNCTION_POINTER => &[
                Operand::FunctionType,
                Operand::Register,
                Operand::RegisterRange,
            ],
            _ => return None,
        };

        Some(InstructionLayout::new(layout))
    }

    /// Return the operand layout for one boolean opcode.
    fn boolean_layout(code: u16) -> Option<InstructionLayout> {
        let operation = BooleanOperation::from_code((code - BOOLEAN_BASE) as u8)?;
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
        let operation = IntegerOperation::from_code(((code - INTEGER_BASE) / INTEGER_COUNT) as u8)?;
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
        let operation = IntegerOperation::from_code(((code - INTEGER128_BASE) / 2) as u8)?;
        if operation.is_count() {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::RegisterRange,
            ]))
        } else if operation.is_comparison() {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::RegisterRange,
                Operand::RegisterRange,
            ]))
        } else if operation.is_overflowing() {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::Result,
                Operand::RegisterRange,
                Operand::RegisterRange,
            ]))
        } else if operation.input_count() == 1 {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::RegisterRange,
            ]))
        } else if operation.uses_count() {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::RegisterRange,
                Operand::Register,
            ]))
        } else {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::RegisterRange,
                Operand::RegisterRange,
            ]))
        }
    }

    /// Return the operand layout for one scalar floating-point opcode.
    fn float_layout(code: u16) -> Option<InstructionLayout> {
        let operation = FloatOperation::from_code(((code - FLOAT_BASE) / FLOAT_COUNT) as u8)?;
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
        let operation = (code - MEMORY_BASE) / SCALAR_COUNT;
        if operation == MemoryOperation::Load as u16 {
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

    /// Return the operand layout for one atomic opcode.
    fn atomic_layout(code: u16) -> Option<InstructionLayout> {
        let operation = (code - ATOMIC_BASE) / SCALAR_COUNT;
        if operation == AtomicOperation::Load as u16 {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::AtomicAccess,
            ]))
        } else if operation == AtomicOperation::Store as u16 {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::AtomicAccess,
            ]))
        } else if operation == AtomicOperation::CompareExchange as u16
            || operation == AtomicOperation::CompareExchangeWeak as u16
        {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Register,
                Operand::CompareExchangeAccess,
            ]))
        } else if operation == AtomicOperation::Wait as u16 {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::AtomicAccess,
            ]))
        } else if operation == AtomicOperation::WaitTimed as u16 {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Register,
                Operand::AtomicAccess,
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

    /// Return the operand layout for one allocation opcode.
    fn allocation_layout(code: u16) -> Option<InstructionLayout> {
        let bits = code - ALLOCATION_BASE;
        let is_slice = bits & (1 << ALLOCATION_OPERATION_SHIFT) != 0;
        let is_fallible = bits & (1 << ALLOCATION_FALLIBILITY_SHIFT) != 0;

        if is_slice && is_fallible {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::Type,
                Operand::Register,
                Operand::Branch,
                Operand::Branch,
            ]))
        } else if is_slice {
            Some(InstructionLayout::new(&[
                Operand::ResultRange,
                Operand::Type,
                Operand::Register,
            ]))
        } else if is_fallible {
            Some(InstructionLayout::new(&[
                Operand::Result,
                Operand::Type,
                Operand::Branch,
                Operand::Branch,
            ]))
        } else {
            Some(InstructionLayout::new(&[Operand::Result, Operand::Type]))
        }
    }

    /// Return the operand layout for one runtime check opcode.
    fn check_layout(code: u16) -> Option<InstructionLayout> {
        let operation = (code - CHECK_BASE) / SCALAR_COUNT;
        if operation == ScalarCheck::Nonzero as u16 {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Branch,
            ]))
        } else if operation == ScalarCheck::Shift as u16 {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Unsigned16,
                Operand::Branch,
            ]))
        } else if operation == ScalarCheck::Narrow as u16 {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Scalar,
                Operand::Branch,
            ]))
        } else if operation == ScalarCheck::Bounds as u16 {
            Some(InstructionLayout::new(&[
                Operand::Register,
                Operand::Register,
                Operand::Branch,
            ]))
        } else if operation == ScalarCheck::Range as u16 {
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
        let operation = code - VECTOR_BASE;
        let layout = match operation {
            operation if operation == VectorOperation::Splat as u16 => {
                &[Operand::ResultRange, Operand::Register, Operand::VectorType][..]
            }
            operation if operation == VectorOperation::Insert as u16 => &[
                Operand::ResultRange,
                Operand::RegisterRange,
                Operand::Register,
                Operand::Register,
                Operand::VectorType,
            ],
            operation if operation == VectorOperation::Extract as u16 => &[
                Operand::Result,
                Operand::RegisterRange,
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
                Operand::RegisterRange,
                Operand::VectorType,
                Operand::Operator,
            ],
            operation if operation == VectorOperation::Convert as u16 => &[
                Operand::ResultRange,
                Operand::RegisterRange,
                Operand::VectorType,
                Operand::VectorType,
                Operand::Operator,
            ],
            operation if operation == VectorOperation::Load as u16 => {
                &[Operand::ResultRange, Operand::Register, Operand::VectorType]
            }
            operation if operation == VectorOperation::Store as u16 => &[
                Operand::Register,
                Operand::RegisterRange,
                Operand::VectorType,
            ],
            _ => return None,
        };

        Some(InstructionLayout::new(layout))
    }

    /// Return the operand layout for one tensor opcode.
    fn tensor_layout(code: u16) -> Option<InstructionLayout> {
        let operation = code - TENSOR_BASE;
        let layout = match operation {
            operation
                if operation == TensorOperation::Element as u16
                    || operation == TensorOperation::Compare as u16 =>
            {
                &[
                    Operand::Result,
                    Operand::RegisterList,
                    Operand::Scalar,
                    Operand::Operator,
                    Operand::Scalar,
                    Operand::Type,
                ][..]
            }
            operation if operation == TensorOperation::Select as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Register,
                Operand::Scalar,
                Operand::Scalar,
                Operand::Type,
            ],
            operation
                if operation == TensorOperation::Transpose as u16
                    || operation == TensorOperation::Broadcast as u16 =>
            {
                &[
                    Operand::Result,
                    Operand::Register,
                    Operand::Unsigned16List,
                    Operand::Scalar,
                    Operand::Type,
                ]
            }
            operation if operation == TensorOperation::Reshape as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::RegisterList,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Slice as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Pad as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Scalar,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Concat as u16 => &[
                Operand::Result,
                Operand::RegisterList,
                Operand::Unsigned16,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Splat as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Scalar,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Convert as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Scalar,
                Operand::Scalar,
                Operand::Operator,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Bitcast as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Reduce as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Scalar,
                Operand::Operator,
                Operand::Unsigned16List,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::IndexReduce as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Scalar,
                Operand::Operator,
                Operand::Unsigned16,
                Operand::Unsigned16,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Contract as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Scalar,
                Operand::ContractionAxes,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Gather as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::GatherAxes,
                Operand::Bits64List,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Scatter as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Register,
                Operand::Scalar,
                Operand::ScatterAxes,
                Operand::Operator,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Load as u16 => &[
                Operand::Result,
                Operand::RegisterRange,
                Operand::RegisterList,
                Operand::Scalar,
            ],
            operation if operation == TensorOperation::Extract as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::RegisterList,
                Operand::Scalar,
            ],
            operation if operation == TensorOperation::Store as u16 => &[
                Operand::RegisterRange,
                Operand::RegisterList,
                Operand::Register,
                Operand::Scalar,
            ],
            operation if operation == TensorOperation::Fill as u16 => {
                &[Operand::RegisterRange, Operand::Register, Operand::Scalar]
            }
            operation if operation == TensorOperation::Copy as u16 => &[
                Operand::RegisterRange,
                Operand::RegisterRange,
                Operand::Scalar,
            ],
            operation if operation == TensorOperation::View as u16 => &[
                Operand::ResultRange,
                Operand::RegisterRange,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::RegisterList,
                Operand::Scalar,
                Operand::Type,
            ],
            operation if operation == TensorOperation::Convolution as u16 => &[
                Operand::Result,
                Operand::Register,
                Operand::Register,
                Operand::Scalar,
                Operand::ConvolutionAxes,
                Operand::Window,
                Operand::ConvolutionGroups,
                Operand::Scalar,
                Operand::Type,
            ],
            _ => return None,
        };

        Some(InstructionLayout::new(layout))
    }
}

const _: () = {
    assert!(FIXED_END < CONSTANT_BASE);
    assert!(CONSTANT_END <= BOOLEAN_BASE);
    assert!(BOOLEAN_END <= INTEGER_BASE);
    assert!(INTEGER_END <= INTEGER128_BASE);
    assert!(INTEGER128_END <= FLOAT_BASE);
    assert!(FLOAT_END <= CAST_INTEGER_BASE);
    assert!(CAST_BIT_END <= MEMORY_BASE);
    assert!(MEMORY_END <= Opcode::LOAD_REFERENCE.0);
    assert!(Opcode::STORE_REFERENCE.0 < ATOMIC_BASE);
    assert!(ATOMIC_END <= ALLOCATION_BASE);
    assert!(ALLOCATION_END <= CHECK_BASE);
    assert!(CHECK_END <= BRANCH_BASE);
    assert!(BRANCH_END <= VECTOR_BASE);
    assert!(VECTOR_END <= TENSOR_BASE);
    assert!(TENSOR_END <= OPCODE_MAX);
};
