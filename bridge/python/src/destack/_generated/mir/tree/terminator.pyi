# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.metadata.dispatch
import destack._generated.mir.tree.call
import destack._generated.mir.tree.constant
import destack._generated.mir.tree.node
import destack._generated.mir.tree.operator
import destack._generated.mir.tree.value

@dataclass(frozen=True, slots=True)
class TerminatorError:
    """Recovered invalid terminator syntax."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorReturn:
    """Return from the function."""

    # the value to return, or None for void functions
    value: destack._generated.mir.tree.value.Value | None
    kind: typing.Literal["return"] = "return"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorJump:
    """Unconditional jump to another block."""

    # the block to jump to
    target: BlockTarget
    kind: typing.Literal["jump"] = "jump"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorBranch:
    """Conditional branch."""

    # the boolean condition to test
    condition: destack._generated.mir.tree.value.Value
    # the block to jump to if condition is true
    then_target: BlockTarget
    # the block to jump to if condition is false
    else_target: BlockTarget
    kind: typing.Literal["branch"] = "branch"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorCheck:
    """Runtime check with explicit success and failure edges."""

    # semantic constraint for the check
    constraint: CheckConstraint
    # the block to jump to when the check succeeds
    success: BlockTarget
    # the block to jump to when the check fails
    failure: BlockTarget
    kind: typing.Literal["check"] = "check"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorSwitch:
    """Switch on an integer value."""

    # the integer value to switch on
    value: destack._generated.mir.tree.value.Value
    # the block to jump to if no case matches
    default: BlockTarget
    # the cases to match against
    cases: SwitchCaseSlice
    kind: typing.Literal["switch"] = "switch"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorYield:
    """Yield from a coroutine."""

    # the yielded value
    value: destack._generated.mir.tree.value.Value
    # the block to resume at when the coroutine is continued
    resume: BlockTarget
    # the cleanup block when the suspended frame is cancelled or dropped
    unwind: BlockTarget | None
    kind: typing.Literal["yield"] = "yield"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorCall:
    """Direct call with an explicit continuation."""

    # the direct callee function
    function: destack._generated.mir.tree.node.LocalNodeId
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    # the continuation block
    target: BlockTarget
    # the cleanup block when this call panics
    unwind: BlockTarget | None
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorCallIndirect:
    """Indirect call with an explicit continuation."""

    # the function pointer or function value to call
    callee: destack._generated.mir.tree.value.Value
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    # the continuation block
    target: BlockTarget
    # the cleanup block when this call panics
    unwind: BlockTarget | None
    kind: typing.Literal["callIndirect"] = "callIndirect"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorCallVirtual:
    """Class call with an explicit continuation."""

    # the receiver value for dispatch
    receiver: destack._generated.mir.tree.value.Value
    # the class type declaring this dispatch slot
    class_: destack._generated.mir.tree.node.LocalNodeId
    # the dispatch slot for the method
    slot: destack._generated.mir.metadata.dispatch.DispatchSlot
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    # the continuation block
    target: BlockTarget
    # the cleanup block when this call panics
    unwind: BlockTarget | None
    kind: typing.Literal["callVirtual"] = "callVirtual"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorCallDynamic:
    """Dynamic call with an explicit continuation."""

    # the receiver value for dispatch
    receiver: destack._generated.mir.tree.value.Value
    # the dynamic constraint type declaring this dispatch slot
    constraint: destack._generated.mir.tree.node.LocalNodeId
    # the dispatch slot for the method
    slot: destack._generated.mir.metadata.dispatch.DispatchSlot
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    # the continuation block
    target: BlockTarget
    # the cleanup block when this call panics
    unwind: BlockTarget | None
    kind: typing.Literal["callDynamic"] = "callDynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorNewZeroedTry:
    """Fallible zeroed typed heap allocation."""

    # the type of the struct to allocate
    layout: destack._generated.mir.tree.node.LocalNodeId
    # the block to jump to when allocation succeeds
    success: BlockTarget
    # the block to jump to when allocation fails
    failure: BlockTarget
    kind: typing.Literal["newZeroedTry"] = "newZeroedTry"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorNewUninitTry:
    """Fallible uninitialized typed heap allocation."""

    # the type of the struct to allocate
    layout: destack._generated.mir.tree.node.LocalNodeId
    # the block to jump to when allocation succeeds
    success: BlockTarget
    # the block to jump to when allocation fails
    failure: BlockTarget
    kind: typing.Literal["newUninitTry"] = "newUninitTry"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorNewSliceZeroedTry:
    """Fallible zeroed slice backing allocation."""

    # the element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the number of elements
    length: destack._generated.mir.tree.value.Value
    # the block to jump to when allocation succeeds
    success: BlockTarget
    # the block to jump to when allocation fails
    failure: BlockTarget
    kind: typing.Literal["newSliceZeroedTry"] = "newSliceZeroedTry"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorNewSliceUninitTry:
    """Fallible uninitialized slice backing allocation."""

    # the element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the number of elements
    length: destack._generated.mir.tree.value.Value
    # the block to jump to when allocation succeeds
    success: BlockTarget
    # the block to jump to when allocation fails
    failure: BlockTarget
    kind: typing.Literal["newSliceUninitTry"] = "newSliceUninitTry"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorPanic:
    """Start language panic unwinding."""

    # optional panic payload
    payload: destack._generated.mir.tree.value.Value | None
    kind: typing.Literal["panic"] = "panic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorUnwindResume:
    """Continue the active unwind after a cleanup block."""

    kind: typing.Literal["unwindResume"] = "unwindResume"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorTrap:
    """Unrecoverable runtime termination."""

    # the trap kind
    kind_value: TrapKind
    # optional trap payload
    payload: destack._generated.mir.tree.value.Value | None
    kind: typing.Literal["trap"] = "trap"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorUnreachable:
    """Unreachable code."""

    kind: typing.Literal["unreachable"] = "unreachable"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorTailCall:
    """Tail call to a function."""

    # the function to tail call
    function: destack._generated.mir.tree.node.LocalNodeId
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["tailCall"] = "tailCall"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorTailCallIndirect:
    """Tail call through a function pointer."""

    # the function pointer or function value to tail call
    callee: destack._generated.mir.tree.value.Value
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["tailCallIndirect"] = "tailCallIndirect"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorTailCallVirtual:
    """Tail call through a virtual dispatch slot."""

    # the receiver value for dispatch
    receiver: destack._generated.mir.tree.value.Value
    # the class type declaring this dispatch slot
    class_: destack._generated.mir.tree.node.LocalNodeId
    # the dispatch slot for the method
    slot: destack._generated.mir.metadata.dispatch.DispatchSlot
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["tailCallVirtual"] = "tailCallVirtual"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TerminatorTailCallDynamic:
    """Tail call through a dynamic dispatch slot."""

    # the receiver value for dispatch
    receiver: destack._generated.mir.tree.value.Value
    # the dynamic constraint type declaring this dispatch slot
    constraint: destack._generated.mir.tree.node.LocalNodeId
    # the dispatch slot for the method
    slot: destack._generated.mir.metadata.dispatch.DispatchSlot
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["tailCallDynamic"] = "tailCallDynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Block terminator node."""
Terminator: typing.TypeAlias = (
    TerminatorError
    | TerminatorReturn
    | TerminatorJump
    | TerminatorBranch
    | TerminatorCheck
    | TerminatorSwitch
    | TerminatorYield
    | TerminatorCall
    | TerminatorCallIndirect
    | TerminatorCallVirtual
    | TerminatorCallDynamic
    | TerminatorNewZeroedTry
    | TerminatorNewUninitTry
    | TerminatorNewSliceZeroedTry
    | TerminatorNewSliceUninitTry
    | TerminatorPanic
    | TerminatorUnwindResume
    | TerminatorTrap
    | TerminatorUnreachable
    | TerminatorTailCall
    | TerminatorTailCallIndirect
    | TerminatorTailCallVirtual
    | TerminatorTailCallDynamic
)

def encode_terminator(writer: BinaryWriter, value: Terminator) -> None: ...
def decode_terminator(reader: BinaryReader) -> Terminator: ...
def to_json_terminator(value: Terminator) -> Json: ...
def from_json_terminator(value: Json) -> Terminator: ...

@dataclass(frozen=True, slots=True)
class BlockTarget:
    """One control-flow edge target."""

    # the block to transfer control to
    block: destack._generated.mir.tree.node.LocalNodeId
    # arguments for the target block's parameters
    arguments: destack._generated.mir.tree.value.ValueSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> BlockTarget: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> BlockTarget: ...

def encode_block_target(writer: BinaryWriter, value: BlockTarget) -> None: ...
def decode_block_target(reader: BinaryReader) -> BlockTarget: ...
def to_json_block_target(value: BlockTarget) -> Json: ...
def from_json_block_target(value: Json) -> BlockTarget: ...

@dataclass(frozen=True, slots=True)
class CheckConstraintBounds:
    """Bounds check on an index into a collection."""

    # the index being checked
    index: destack._generated.mir.tree.value.Value
    # the length being checked against
    length: destack._generated.mir.tree.value.Value
    # the collection being indexed
    collection: destack._generated.mir.tree.value.Value
    # whether the index is treated as signed
    is_signed: bool
    kind: typing.Literal["bounds"] = "bounds"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckConstraintNull:
    """Null check on a reference."""

    # the value being checked for null
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckConstraintDivZero:
    """Division by zero check."""

    # the divisor being checked for zero
    divisor: destack._generated.mir.tree.value.Value
    kind: typing.Literal["divZero"] = "divZero"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckConstraintShiftRange:
    """Shift amount range check."""

    # the shift amount being checked
    value: destack._generated.mir.tree.value.Value
    # the bit width of the shifted type
    bit_width: int
    # whether the shift amount is signed
    is_signed: bool
    kind: typing.Literal["shiftRange"] = "shiftRange"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckConstraintNarrow:
    """Integer narrowing check."""

    # the value being narrowed
    value: destack._generated.mir.tree.value.Value
    # the target bit width
    to_width: int
    # whether the narrowed value is signed
    is_signed: bool
    kind: typing.Literal["narrow"] = "narrow"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckConstraintOverflow:
    """Overflow check for an arithmetic operation."""

    # the operator being checked
    operator: destack._generated.mir.tree.operator.BinaryOperator
    # the left operand
    left: destack._generated.mir.tree.value.Value
    # the right operand
    right: destack._generated.mir.tree.value.Value
    # whether the overflow check is signed
    is_signed: bool
    kind: typing.Literal["overflow"] = "overflow"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckConstraintType:
    """Runtime type descriptor check for a value."""

    # the descriptor value being checked
    value: destack._generated.mir.tree.value.Value
    # the expected dynamic type for this descriptor
    expected: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckConstraintVariant:
    """Variant tag check for a physical tagged sum value."""

    # the tag value being checked
    value: destack._generated.mir.tree.value.Value
    # the expected tag constant
    expected: destack._generated.mir.tree.constant.Constant
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckConstraintReceiverType:
    """Dynamic receiver type check for a class or concrete receiver."""

    # the receiver being checked
    receiver: destack._generated.mir.tree.value.Value
    # the expected concrete receiver type
    expected: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["receiverType"] = "receiverType"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckConstraintImplements:
    """Interface conformance check for a receiver."""

    # the receiver being checked
    receiver: destack._generated.mir.tree.value.Value
    # the expected interface type
    expected: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["implements"] = "implements"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Semantic constraint for a runtime check."""
CheckConstraint: typing.TypeAlias = (
    CheckConstraintBounds
    | CheckConstraintNull
    | CheckConstraintDivZero
    | CheckConstraintShiftRange
    | CheckConstraintNarrow
    | CheckConstraintOverflow
    | CheckConstraintType
    | CheckConstraintVariant
    | CheckConstraintReceiverType
    | CheckConstraintImplements
)

def encode_check_constraint(writer: BinaryWriter, value: CheckConstraint) -> None: ...
def decode_check_constraint(reader: BinaryReader) -> CheckConstraint: ...
def to_json_check_constraint(value: CheckConstraint) -> Json: ...
def from_json_check_constraint(value: Json) -> CheckConstraint: ...

@dataclass(frozen=True, slots=True)
class SwitchCaseSlice:
    """Compact reference to a switch case list stored in the MIR tree."""

    # start index in the switch case buffer
    start: int
    # number of switch cases in the slice
    count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SwitchCaseSlice: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SwitchCaseSlice: ...

def encode_switch_case_slice(writer: BinaryWriter, value: SwitchCaseSlice) -> None: ...
def decode_switch_case_slice(reader: BinaryReader) -> SwitchCaseSlice: ...
def to_json_switch_case_slice(value: SwitchCaseSlice) -> Json: ...
def from_json_switch_case_slice(value: Json) -> SwitchCaseSlice: ...

"""Unrecoverable runtime trap kind."""
TrapKind: typing.TypeAlias = typing.Literal["abort"]

def encode_trap_kind(writer: BinaryWriter, value: TrapKind) -> None: ...
def decode_trap_kind(reader: BinaryReader) -> TrapKind: ...
def to_json_trap_kind(value: TrapKind) -> Json: ...
def from_json_trap_kind(value: Json) -> TrapKind: ...

@dataclass(frozen=True, slots=True)
class SwitchCase:
    """One case arm for a switch terminator."""

    # the matched case value
    value: int
    # the target block for this case
    target: BlockTarget

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SwitchCase: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SwitchCase: ...

def encode_switch_case(writer: BinaryWriter, value: SwitchCase) -> None: ...
def decode_switch_case(reader: BinaryReader) -> SwitchCase: ...
def to_json_switch_case(value: SwitchCase) -> Json: ...
def from_json_switch_case(value: Json) -> SwitchCase: ...

__all__ = [
    "Terminator",
    "encode_terminator",
    "decode_terminator",
    "to_json_terminator",
    "from_json_terminator",
    "TerminatorError",
    "TerminatorReturn",
    "TerminatorJump",
    "TerminatorBranch",
    "TerminatorCheck",
    "TerminatorSwitch",
    "TerminatorYield",
    "TerminatorCall",
    "TerminatorCallIndirect",
    "TerminatorCallVirtual",
    "TerminatorCallDynamic",
    "TerminatorNewZeroedTry",
    "TerminatorNewUninitTry",
    "TerminatorNewSliceZeroedTry",
    "TerminatorNewSliceUninitTry",
    "TerminatorPanic",
    "TerminatorUnwindResume",
    "TerminatorTrap",
    "TerminatorUnreachable",
    "TerminatorTailCall",
    "TerminatorTailCallIndirect",
    "TerminatorTailCallVirtual",
    "TerminatorTailCallDynamic",
    "BlockTarget",
    "encode_block_target",
    "decode_block_target",
    "to_json_block_target",
    "from_json_block_target",
    "CheckConstraint",
    "encode_check_constraint",
    "decode_check_constraint",
    "to_json_check_constraint",
    "from_json_check_constraint",
    "CheckConstraintBounds",
    "CheckConstraintNull",
    "CheckConstraintDivZero",
    "CheckConstraintShiftRange",
    "CheckConstraintNarrow",
    "CheckConstraintOverflow",
    "CheckConstraintType",
    "CheckConstraintVariant",
    "CheckConstraintReceiverType",
    "CheckConstraintImplements",
    "SwitchCaseSlice",
    "encode_switch_case_slice",
    "decode_switch_case_slice",
    "to_json_switch_case_slice",
    "from_json_switch_case_slice",
    "TrapKind",
    "encode_trap_kind",
    "decode_trap_kind",
    "to_json_trap_kind",
    "from_json_trap_kind",
    "SwitchCase",
    "encode_switch_case",
    "decode_switch_case",
    "to_json_switch_case",
    "from_json_switch_case",
]
