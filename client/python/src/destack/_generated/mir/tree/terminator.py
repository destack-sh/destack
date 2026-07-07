# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.mir.table.dispatch
import destack._generated.mir.tree.call
import destack._generated.mir.tree.constant
import destack._generated.mir.tree.node
import destack._generated.mir.tree.operator
import destack._generated.mir.tree.value


@dataclass(frozen=True, slots=True)
class TerminatorError:
    """Recovered invalid terminator syntax."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorReturn:
    """Return from the function."""

    # the value to return, or None for void functions
    value: destack._generated.mir.tree.value.Value | None
    kind: typing.Literal["return"] = "return"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorJump:
    """Unconditional jump to another block."""

    # the block to jump to
    target: BlockTarget
    kind: typing.Literal["jump"] = "jump"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorYield:
    """Yield from a coroutine to its current owner."""

    # the yielded value
    value: destack._generated.mir.tree.value.Value
    # the block entered when the coroutine receives a resume command
    resume: BlockTarget
    # the cleanup block when the yield is left by panic unwinding
    unwind: BlockTarget | None
    kind: typing.Literal["yield"] = "yield"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorCallVirtual:
    """Class call with an explicit continuation."""

    # the receiver value for dispatch
    receiver: destack._generated.mir.tree.value.Value
    # the class type declaring this dispatch slot
    class_: destack._generated.mir.tree.node.LocalNodeId
    # the dispatch slot for the method
    slot: destack._generated.mir.table.dispatch.DispatchSlot
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    # the continuation block
    target: BlockTarget
    # the cleanup block when this call panics
    unwind: BlockTarget | None
    kind: typing.Literal["callVirtual"] = "callVirtual"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorCallDynamic:
    """Dynamic call with an explicit continuation."""

    # the receiver value for dispatch
    receiver: destack._generated.mir.tree.value.Value
    # the dynamic constraint type declaring this dispatch slot
    constraint: destack._generated.mir.tree.node.LocalNodeId
    # the dispatch slot for the method
    slot: destack._generated.mir.table.dispatch.DispatchSlot
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    # the continuation block
    target: BlockTarget
    # the cleanup block when this call panics
    unwind: BlockTarget | None
    kind: typing.Literal["callDynamic"] = "callDynamic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorPanic:
    """Start language panic unwinding."""

    # optional panic payload
    payload: destack._generated.mir.tree.value.Value | None
    kind: typing.Literal["panic"] = "panic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorUnwindResume:
    """Continue the active unwind after a cleanup block."""

    kind: typing.Literal["unwindResume"] = "unwindResume"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorTrap:
    """Unrecoverable runtime termination."""

    # the trap kind
    kind_value: TrapKind
    # optional trap payload
    payload: destack._generated.mir.tree.value.Value | None
    kind: typing.Literal["trap"] = "trap"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorUnreachable:
    """Unreachable code."""

    kind: typing.Literal["unreachable"] = "unreachable"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorTailCall:
    """Tail call to a function."""

    # the function to tail call
    function: destack._generated.mir.tree.node.LocalNodeId
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["tailCall"] = "tailCall"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorTailCallIndirect:
    """Tail call through a function pointer."""

    # the function pointer or function value to tail call
    callee: destack._generated.mir.tree.value.Value
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["tailCallIndirect"] = "tailCallIndirect"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorTailCallVirtual:
    """Tail call through a virtual dispatch slot."""

    # the receiver value for dispatch
    receiver: destack._generated.mir.tree.value.Value
    # the class type declaring this dispatch slot
    class_: destack._generated.mir.tree.node.LocalNodeId
    # the dispatch slot for the method
    slot: destack._generated.mir.table.dispatch.DispatchSlot
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["tailCallVirtual"] = "tailCallVirtual"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


@dataclass(frozen=True, slots=True)
class TerminatorTailCallDynamic:
    """Tail call through a dynamic dispatch slot."""

    # the receiver value for dispatch
    receiver: destack._generated.mir.tree.value.Value
    # the dynamic constraint type declaring this dispatch slot
    constraint: destack._generated.mir.tree.node.LocalNodeId
    # the dispatch slot for the method
    slot: destack._generated.mir.table.dispatch.DispatchSlot
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["tailCallDynamic"] = "tailCallDynamic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_terminator(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_terminator(self)


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


def encode_terminator(writer: BinaryWriter, value: Terminator) -> None:
    """Encode one Terminator."""
    if value.kind == "error":
        writer.write_unsigned(0)
    elif value.kind == "return":
        writer.write_unsigned(1)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "jump":
        writer.write_unsigned(2)
        encode_block_target(writer, value.target)
    elif value.kind == "branch":
        writer.write_unsigned(3)
        destack._generated.mir.tree.value.encode_value(writer, value.condition)
        encode_block_target(writer, value.then_target)
        encode_block_target(writer, value.else_target)
    elif value.kind == "check":
        writer.write_unsigned(4)
        encode_check_constraint(writer, value.constraint)
        encode_block_target(writer, value.success)
        encode_block_target(writer, value.failure)
    elif value.kind == "switch":
        writer.write_unsigned(5)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        encode_block_target(writer, value.default)
        encode_switch_case_slice(writer, value.cases)
    elif value.kind == "yield":
        writer.write_unsigned(6)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        encode_block_target(writer, value.resume)
        if value.unwind is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_block_target(writer, value.unwind)
    elif value.kind == "call":
        writer.write_unsigned(7)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
        destack._generated.mir.tree.call.encode_call(writer, value.call)
        encode_block_target(writer, value.target)
        if value.unwind is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_block_target(writer, value.unwind)
    elif value.kind == "callIndirect":
        writer.write_unsigned(8)
        destack._generated.mir.tree.value.encode_value(writer, value.callee)
        destack._generated.mir.tree.call.encode_call(writer, value.call)
        encode_block_target(writer, value.target)
        if value.unwind is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_block_target(writer, value.unwind)
    elif value.kind == "callVirtual":
        writer.write_unsigned(9)
        destack._generated.mir.tree.value.encode_value(writer, value.receiver)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.class_)
        destack._generated.mir.table.dispatch.encode_dispatch_slot(writer, value.slot)
        destack._generated.mir.tree.call.encode_call(writer, value.call)
        encode_block_target(writer, value.target)
        if value.unwind is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_block_target(writer, value.unwind)
    elif value.kind == "callDynamic":
        writer.write_unsigned(10)
        destack._generated.mir.tree.value.encode_value(writer, value.receiver)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.constraint)
        destack._generated.mir.table.dispatch.encode_dispatch_slot(writer, value.slot)
        destack._generated.mir.tree.call.encode_call(writer, value.call)
        encode_block_target(writer, value.target)
        if value.unwind is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_block_target(writer, value.unwind)
    elif value.kind == "newZeroedTry":
        writer.write_unsigned(11)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.layout)
        encode_block_target(writer, value.success)
        encode_block_target(writer, value.failure)
    elif value.kind == "newUninitTry":
        writer.write_unsigned(12)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.layout)
        encode_block_target(writer, value.success)
        encode_block_target(writer, value.failure)
    elif value.kind == "newSliceZeroedTry":
        writer.write_unsigned(13)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
        destack._generated.mir.tree.value.encode_value(writer, value.length)
        encode_block_target(writer, value.success)
        encode_block_target(writer, value.failure)
    elif value.kind == "newSliceUninitTry":
        writer.write_unsigned(14)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
        destack._generated.mir.tree.value.encode_value(writer, value.length)
        encode_block_target(writer, value.success)
        encode_block_target(writer, value.failure)
    elif value.kind == "panic":
        writer.write_unsigned(15)
        if value.payload is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.value.encode_value(writer, value.payload)
    elif value.kind == "unwindResume":
        writer.write_unsigned(16)
    elif value.kind == "trap":
        writer.write_unsigned(17)
        encode_trap_kind(writer, value.kind_value)
        if value.payload is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.value.encode_value(writer, value.payload)
    elif value.kind == "unreachable":
        writer.write_unsigned(18)
    elif value.kind == "tailCall":
        writer.write_unsigned(19)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
        destack._generated.mir.tree.call.encode_call(writer, value.call)
    elif value.kind == "tailCallIndirect":
        writer.write_unsigned(20)
        destack._generated.mir.tree.value.encode_value(writer, value.callee)
        destack._generated.mir.tree.call.encode_call(writer, value.call)
    elif value.kind == "tailCallVirtual":
        writer.write_unsigned(21)
        destack._generated.mir.tree.value.encode_value(writer, value.receiver)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.class_)
        destack._generated.mir.table.dispatch.encode_dispatch_slot(writer, value.slot)
        destack._generated.mir.tree.call.encode_call(writer, value.call)
    elif value.kind == "tailCallDynamic":
        writer.write_unsigned(22)
        destack._generated.mir.tree.value.encode_value(writer, value.receiver)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.constraint)
        destack._generated.mir.table.dispatch.encode_dispatch_slot(writer, value.slot)
        destack._generated.mir.tree.call.encode_call(writer, value.call)
    else:
        raise SerdeError("unknown enum variant")


def decode_terminator(reader: BinaryReader) -> Terminator:
    """Decode one Terminator."""
    variant = reader.read_number()

    if variant == 0:
        return TerminatorError()
    elif variant == 1:
        value_ = reader.read_option(
            lambda: destack._generated.mir.tree.value.decode_value(reader)
        )

        return TerminatorReturn(
            value=value_,
        )
    elif variant == 2:
        target = decode_block_target(reader)

        return TerminatorJump(
            target=target,
        )
    elif variant == 3:
        condition = destack._generated.mir.tree.value.decode_value(reader)
        then_target = decode_block_target(reader)
        else_target = decode_block_target(reader)

        return TerminatorBranch(
            condition=condition,
            then_target=then_target,
            else_target=else_target,
        )
    elif variant == 4:
        constraint = decode_check_constraint(reader)
        success = decode_block_target(reader)
        failure = decode_block_target(reader)

        return TerminatorCheck(
            constraint=constraint,
            success=success,
            failure=failure,
        )
    elif variant == 5:
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        default = decode_block_target(reader)
        cases = decode_switch_case_slice(reader)

        return TerminatorSwitch(
            value=value_,
            default=default,
            cases=cases,
        )
    elif variant == 6:
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        resume = decode_block_target(reader)
        unwind = reader.read_option(lambda: decode_block_target(reader))

        return TerminatorYield(
            value=value_,
            resume=resume,
            unwind=unwind,
        )
    elif variant == 7:
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)
        target = decode_block_target(reader)
        unwind = reader.read_option(lambda: decode_block_target(reader))

        return TerminatorCall(
            function=function,
            call=call,
            target=target,
            unwind=unwind,
        )
    elif variant == 8:
        callee = destack._generated.mir.tree.value.decode_value(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)
        target = decode_block_target(reader)
        unwind = reader.read_option(lambda: decode_block_target(reader))

        return TerminatorCallIndirect(
            callee=callee,
            call=call,
            target=target,
            unwind=unwind,
        )
    elif variant == 9:
        receiver = destack._generated.mir.tree.value.decode_value(reader)
        class_ = destack._generated.mir.tree.node.decode_local_node_id(reader)
        slot = destack._generated.mir.table.dispatch.decode_dispatch_slot(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)
        target = decode_block_target(reader)
        unwind = reader.read_option(lambda: decode_block_target(reader))

        return TerminatorCallVirtual(
            receiver=receiver,
            class_=class_,
            slot=slot,
            call=call,
            target=target,
            unwind=unwind,
        )
    elif variant == 10:
        receiver = destack._generated.mir.tree.value.decode_value(reader)
        constraint = destack._generated.mir.tree.node.decode_local_node_id(reader)
        slot = destack._generated.mir.table.dispatch.decode_dispatch_slot(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)
        target = decode_block_target(reader)
        unwind = reader.read_option(lambda: decode_block_target(reader))

        return TerminatorCallDynamic(
            receiver=receiver,
            constraint=constraint,
            slot=slot,
            call=call,
            target=target,
            unwind=unwind,
        )
    elif variant == 11:
        layout = destack._generated.mir.tree.node.decode_local_node_id(reader)
        success = decode_block_target(reader)
        failure = decode_block_target(reader)

        return TerminatorNewZeroedTry(
            layout=layout,
            success=success,
            failure=failure,
        )
    elif variant == 12:
        layout = destack._generated.mir.tree.node.decode_local_node_id(reader)
        success = decode_block_target(reader)
        failure = decode_block_target(reader)

        return TerminatorNewUninitTry(
            layout=layout,
            success=success,
            failure=failure,
        )
    elif variant == 13:
        element = destack._generated.mir.tree.node.decode_local_node_id(reader)
        length = destack._generated.mir.tree.value.decode_value(reader)
        success = decode_block_target(reader)
        failure = decode_block_target(reader)

        return TerminatorNewSliceZeroedTry(
            element=element,
            length=length,
            success=success,
            failure=failure,
        )
    elif variant == 14:
        element = destack._generated.mir.tree.node.decode_local_node_id(reader)
        length = destack._generated.mir.tree.value.decode_value(reader)
        success = decode_block_target(reader)
        failure = decode_block_target(reader)

        return TerminatorNewSliceUninitTry(
            element=element,
            length=length,
            success=success,
            failure=failure,
        )
    elif variant == 15:
        payload = reader.read_option(
            lambda: destack._generated.mir.tree.value.decode_value(reader)
        )

        return TerminatorPanic(
            payload=payload,
        )
    elif variant == 16:
        return TerminatorUnwindResume()
    elif variant == 17:
        kind_value = decode_trap_kind(reader)
        payload = reader.read_option(
            lambda: destack._generated.mir.tree.value.decode_value(reader)
        )

        return TerminatorTrap(
            kind_value=kind_value,
            payload=payload,
        )
    elif variant == 18:
        return TerminatorUnreachable()
    elif variant == 19:
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)

        return TerminatorTailCall(
            function=function,
            call=call,
        )
    elif variant == 20:
        callee = destack._generated.mir.tree.value.decode_value(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)

        return TerminatorTailCallIndirect(
            callee=callee,
            call=call,
        )
    elif variant == 21:
        receiver = destack._generated.mir.tree.value.decode_value(reader)
        class_ = destack._generated.mir.tree.node.decode_local_node_id(reader)
        slot = destack._generated.mir.table.dispatch.decode_dispatch_slot(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)

        return TerminatorTailCallVirtual(
            receiver=receiver,
            class_=class_,
            slot=slot,
            call=call,
        )
    elif variant == 22:
        receiver = destack._generated.mir.tree.value.decode_value(reader)
        constraint = destack._generated.mir.tree.node.decode_local_node_id(reader)
        slot = destack._generated.mir.table.dispatch.decode_dispatch_slot(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)

        return TerminatorTailCallDynamic(
            receiver=receiver,
            constraint=constraint,
            slot=slot,
            call=call,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_terminator(value: Terminator) -> Json:
    """Return one JSON value for one Terminator."""
    if value.kind == "error":
        return {
            "kind": "error",
        }
    elif value.kind == "return":
        return {
            "kind": "return",
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.mir.tree.value.to_json_value(
                        value.value
                    )
                }
            ),
        }
    elif value.kind == "jump":
        return {
            "kind": "jump",
            "target": to_json_block_target(value.target),
        }
    elif value.kind == "branch":
        return {
            "kind": "branch",
            "condition": destack._generated.mir.tree.value.to_json_value(
                value.condition
            ),
            "thenTarget": to_json_block_target(value.then_target),
            "elseTarget": to_json_block_target(value.else_target),
        }
    elif value.kind == "check":
        return {
            "kind": "check",
            "constraint": to_json_check_constraint(value.constraint),
            "success": to_json_block_target(value.success),
            "failure": to_json_block_target(value.failure),
        }
    elif value.kind == "switch":
        return {
            "kind": "switch",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "default": to_json_block_target(value.default),
            "cases": to_json_switch_case_slice(value.cases),
        }
    elif value.kind == "yield":
        return {
            "kind": "yield",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "resume": to_json_block_target(value.resume),
            **(
                {}
                if value.unwind is None
                else {"unwind": to_json_block_target(value.unwind)}
            ),
        }
    elif value.kind == "call":
        return {
            "kind": "call",
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
            "target": to_json_block_target(value.target),
            **(
                {}
                if value.unwind is None
                else {"unwind": to_json_block_target(value.unwind)}
            ),
        }
    elif value.kind == "callIndirect":
        return {
            "kind": "callIndirect",
            "callee": destack._generated.mir.tree.value.to_json_value(value.callee),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
            "target": to_json_block_target(value.target),
            **(
                {}
                if value.unwind is None
                else {"unwind": to_json_block_target(value.unwind)}
            ),
        }
    elif value.kind == "callVirtual":
        return {
            "kind": "callVirtual",
            "receiver": destack._generated.mir.tree.value.to_json_value(value.receiver),
            "class": destack._generated.mir.tree.node.to_json_local_node_id(
                value.class_
            ),
            "slot": destack._generated.mir.table.dispatch.to_json_dispatch_slot(
                value.slot
            ),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
            "target": to_json_block_target(value.target),
            **(
                {}
                if value.unwind is None
                else {"unwind": to_json_block_target(value.unwind)}
            ),
        }
    elif value.kind == "callDynamic":
        return {
            "kind": "callDynamic",
            "receiver": destack._generated.mir.tree.value.to_json_value(value.receiver),
            "constraint": destack._generated.mir.tree.node.to_json_local_node_id(
                value.constraint
            ),
            "slot": destack._generated.mir.table.dispatch.to_json_dispatch_slot(
                value.slot
            ),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
            "target": to_json_block_target(value.target),
            **(
                {}
                if value.unwind is None
                else {"unwind": to_json_block_target(value.unwind)}
            ),
        }
    elif value.kind == "newZeroedTry":
        return {
            "kind": "newZeroedTry",
            "layout": destack._generated.mir.tree.node.to_json_local_node_id(
                value.layout
            ),
            "success": to_json_block_target(value.success),
            "failure": to_json_block_target(value.failure),
        }
    elif value.kind == "newUninitTry":
        return {
            "kind": "newUninitTry",
            "layout": destack._generated.mir.tree.node.to_json_local_node_id(
                value.layout
            ),
            "success": to_json_block_target(value.success),
            "failure": to_json_block_target(value.failure),
        }
    elif value.kind == "newSliceZeroedTry":
        return {
            "kind": "newSliceZeroedTry",
            "element": destack._generated.mir.tree.node.to_json_local_node_id(
                value.element
            ),
            "length": destack._generated.mir.tree.value.to_json_value(value.length),
            "success": to_json_block_target(value.success),
            "failure": to_json_block_target(value.failure),
        }
    elif value.kind == "newSliceUninitTry":
        return {
            "kind": "newSliceUninitTry",
            "element": destack._generated.mir.tree.node.to_json_local_node_id(
                value.element
            ),
            "length": destack._generated.mir.tree.value.to_json_value(value.length),
            "success": to_json_block_target(value.success),
            "failure": to_json_block_target(value.failure),
        }
    elif value.kind == "panic":
        return {
            "kind": "panic",
            **(
                {}
                if value.payload is None
                else {
                    "payload": destack._generated.mir.tree.value.to_json_value(
                        value.payload
                    )
                }
            ),
        }
    elif value.kind == "unwindResume":
        return {
            "kind": "unwindResume",
        }
    elif value.kind == "trap":
        return {
            "kind": "trap",
            "kind": to_json_trap_kind(value.kind_value),
            **(
                {}
                if value.payload is None
                else {
                    "payload": destack._generated.mir.tree.value.to_json_value(
                        value.payload
                    )
                }
            ),
        }
    elif value.kind == "unreachable":
        return {
            "kind": "unreachable",
        }
    elif value.kind == "tailCall":
        return {
            "kind": "tailCall",
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
        }
    elif value.kind == "tailCallIndirect":
        return {
            "kind": "tailCallIndirect",
            "callee": destack._generated.mir.tree.value.to_json_value(value.callee),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
        }
    elif value.kind == "tailCallVirtual":
        return {
            "kind": "tailCallVirtual",
            "receiver": destack._generated.mir.tree.value.to_json_value(value.receiver),
            "class": destack._generated.mir.tree.node.to_json_local_node_id(
                value.class_
            ),
            "slot": destack._generated.mir.table.dispatch.to_json_dispatch_slot(
                value.slot
            ),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
        }
    elif value.kind == "tailCallDynamic":
        return {
            "kind": "tailCallDynamic",
            "receiver": destack._generated.mir.tree.value.to_json_value(value.receiver),
            "constraint": destack._generated.mir.tree.node.to_json_local_node_id(
                value.constraint
            ),
            "slot": destack._generated.mir.table.dispatch.to_json_dispatch_slot(
                value.slot
            ),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_terminator(value: Json) -> Terminator:
    """Return one Terminator from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "error":
        return TerminatorError()
    elif kind == "return":
        return TerminatorReturn(
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.mir.tree.value.from_json_value(value),
            ),
        )
    elif kind == "jump":
        return TerminatorJump(
            target=from_json_block_target(json_field(object_, "target")),
        )
    elif kind == "branch":
        return TerminatorBranch(
            condition=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "condition")
            ),
            then_target=from_json_block_target(json_field(object_, "thenTarget")),
            else_target=from_json_block_target(json_field(object_, "elseTarget")),
        )
    elif kind == "check":
        return TerminatorCheck(
            constraint=from_json_check_constraint(json_field(object_, "constraint")),
            success=from_json_block_target(json_field(object_, "success")),
            failure=from_json_block_target(json_field(object_, "failure")),
        )
    elif kind == "switch":
        return TerminatorSwitch(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            default=from_json_block_target(json_field(object_, "default")),
            cases=from_json_switch_case_slice(json_field(object_, "cases")),
        )
    elif kind == "yield":
        return TerminatorYield(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            resume=from_json_block_target(json_field(object_, "resume")),
            unwind=json_optional(
                object_, "unwind", lambda value: from_json_block_target(value)
            ),
        )
    elif kind == "call":
        return TerminatorCall(
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
            target=from_json_block_target(json_field(object_, "target")),
            unwind=json_optional(
                object_, "unwind", lambda value: from_json_block_target(value)
            ),
        )
    elif kind == "callIndirect":
        return TerminatorCallIndirect(
            callee=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "callee")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
            target=from_json_block_target(json_field(object_, "target")),
            unwind=json_optional(
                object_, "unwind", lambda value: from_json_block_target(value)
            ),
        )
    elif kind == "callVirtual":
        return TerminatorCallVirtual(
            receiver=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "receiver")
            ),
            class_=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "class")
            ),
            slot=destack._generated.mir.table.dispatch.from_json_dispatch_slot(
                json_field(object_, "slot")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
            target=from_json_block_target(json_field(object_, "target")),
            unwind=json_optional(
                object_, "unwind", lambda value: from_json_block_target(value)
            ),
        )
    elif kind == "callDynamic":
        return TerminatorCallDynamic(
            receiver=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "receiver")
            ),
            constraint=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "constraint")
            ),
            slot=destack._generated.mir.table.dispatch.from_json_dispatch_slot(
                json_field(object_, "slot")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
            target=from_json_block_target(json_field(object_, "target")),
            unwind=json_optional(
                object_, "unwind", lambda value: from_json_block_target(value)
            ),
        )
    elif kind == "newZeroedTry":
        return TerminatorNewZeroedTry(
            layout=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "layout")
            ),
            success=from_json_block_target(json_field(object_, "success")),
            failure=from_json_block_target(json_field(object_, "failure")),
        )
    elif kind == "newUninitTry":
        return TerminatorNewUninitTry(
            layout=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "layout")
            ),
            success=from_json_block_target(json_field(object_, "success")),
            failure=from_json_block_target(json_field(object_, "failure")),
        )
    elif kind == "newSliceZeroedTry":
        return TerminatorNewSliceZeroedTry(
            element=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
            length=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "length")
            ),
            success=from_json_block_target(json_field(object_, "success")),
            failure=from_json_block_target(json_field(object_, "failure")),
        )
    elif kind == "newSliceUninitTry":
        return TerminatorNewSliceUninitTry(
            element=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
            length=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "length")
            ),
            success=from_json_block_target(json_field(object_, "success")),
            failure=from_json_block_target(json_field(object_, "failure")),
        )
    elif kind == "panic":
        return TerminatorPanic(
            payload=json_optional(
                object_,
                "payload",
                lambda value: destack._generated.mir.tree.value.from_json_value(value),
            ),
        )
    elif kind == "unwindResume":
        return TerminatorUnwindResume()
    elif kind == "trap":
        return TerminatorTrap(
            kind_value=from_json_trap_kind(json_field(object_, "kind")),
            payload=json_optional(
                object_,
                "payload",
                lambda value: destack._generated.mir.tree.value.from_json_value(value),
            ),
        )
    elif kind == "unreachable":
        return TerminatorUnreachable()
    elif kind == "tailCall":
        return TerminatorTailCall(
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
        )
    elif kind == "tailCallIndirect":
        return TerminatorTailCallIndirect(
            callee=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "callee")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
        )
    elif kind == "tailCallVirtual":
        return TerminatorTailCallVirtual(
            receiver=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "receiver")
            ),
            class_=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "class")
            ),
            slot=destack._generated.mir.table.dispatch.from_json_dispatch_slot(
                json_field(object_, "slot")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
        )
    elif kind == "tailCallDynamic":
        return TerminatorTailCallDynamic(
            receiver=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "receiver")
            ),
            constraint=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "constraint")
            ),
            slot=destack._generated.mir.table.dispatch.from_json_dispatch_slot(
                json_field(object_, "slot")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class BlockTarget:
    """One control-flow edge target."""

    # the block to transfer control to
    block: destack._generated.mir.tree.node.LocalNodeId
    # arguments for the target block's parameters
    arguments: destack._generated.mir.tree.value.ValueSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_block_target(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BlockTarget:
        """Decode one BlockTarget."""
        return decode_block_target(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_block_target(self)

    @classmethod
    def from_json(cls, value: Json) -> BlockTarget:
        """Return one BlockTarget from one JSON value."""
        return from_json_block_target(value)


def encode_block_target(writer: BinaryWriter, value: BlockTarget) -> None:
    """Encode one BlockTarget."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.block)
    destack._generated.mir.tree.value.encode_value_slice(writer, value.arguments)


def decode_block_target(reader: BinaryReader) -> BlockTarget:
    """Decode one BlockTarget."""
    block = destack._generated.mir.tree.node.decode_local_node_id(reader)
    arguments = destack._generated.mir.tree.value.decode_value_slice(reader)

    return BlockTarget(
        block=block,
        arguments=arguments,
    )


def to_json_block_target(value: BlockTarget) -> Json:
    """Return one JSON value for one BlockTarget."""
    return {
        "block": destack._generated.mir.tree.node.to_json_local_node_id(value.block),
        "arguments": destack._generated.mir.tree.value.to_json_value_slice(
            value.arguments
        ),
    }


def from_json_block_target(value: Json) -> BlockTarget:
    """Return one BlockTarget from one JSON value."""
    object_ = json_object(value)

    return BlockTarget(
        block=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "block")
        ),
        arguments=destack._generated.mir.tree.value.from_json_value_slice(
            json_field(object_, "arguments")
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_constraint(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_constraint(self)


@dataclass(frozen=True, slots=True)
class CheckConstraintNull:
    """Null check on a reference."""

    # the value being checked for null
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_constraint(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_constraint(self)


@dataclass(frozen=True, slots=True)
class CheckConstraintDivZero:
    """Division by zero check."""

    # the divisor being checked for zero
    divisor: destack._generated.mir.tree.value.Value
    kind: typing.Literal["divZero"] = "divZero"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_constraint(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_constraint(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_constraint(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_constraint(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_constraint(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_constraint(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_constraint(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_constraint(self)


@dataclass(frozen=True, slots=True)
class CheckConstraintIsType:
    """Exact runtime type check for a value."""

    # the value being checked
    value: destack._generated.mir.tree.value.Value
    # the expected concrete runtime type
    expected: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["isType"] = "isType"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_constraint(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_constraint(self)


@dataclass(frozen=True, slots=True)
class CheckConstraintVariant:
    """Variant tag check for a physical tagged sum value."""

    # the tag value being checked
    value: destack._generated.mir.tree.value.Value
    # the expected tag constant
    expected: destack._generated.mir.tree.constant.Constant
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_constraint(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_constraint(self)


@dataclass(frozen=True, slots=True)
class CheckConstraintIsSubtype:
    """Runtime subtype relation check for a value."""

    # the value being checked
    value: destack._generated.mir.tree.value.Value
    # the expected supertype
    expected: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["isSubtype"] = "isSubtype"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_constraint(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_constraint(self)


"""Semantic constraint for a runtime check."""
CheckConstraint: typing.TypeAlias = (
    CheckConstraintBounds
    | CheckConstraintNull
    | CheckConstraintDivZero
    | CheckConstraintShiftRange
    | CheckConstraintNarrow
    | CheckConstraintOverflow
    | CheckConstraintIsType
    | CheckConstraintVariant
    | CheckConstraintIsSubtype
)


def encode_check_constraint(writer: BinaryWriter, value: CheckConstraint) -> None:
    """Encode one CheckConstraint."""
    if value.kind == "bounds":
        writer.write_unsigned(0)
        destack._generated.mir.tree.value.encode_value(writer, value.index)
        destack._generated.mir.tree.value.encode_value(writer, value.length)
        destack._generated.mir.tree.value.encode_value(writer, value.collection)
        writer.write_bool(value.is_signed)
    elif value.kind == "null":
        writer.write_unsigned(1)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "divZero":
        writer.write_unsigned(2)
        destack._generated.mir.tree.value.encode_value(writer, value.divisor)
    elif value.kind == "shiftRange":
        writer.write_unsigned(3)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        writer.write_byte(value.bit_width)
        writer.write_bool(value.is_signed)
    elif value.kind == "narrow":
        writer.write_unsigned(4)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        writer.write_byte(value.to_width)
        writer.write_bool(value.is_signed)
    elif value.kind == "overflow":
        writer.write_unsigned(5)
        destack._generated.mir.tree.operator.encode_binary_operator(
            writer, value.operator
        )
        destack._generated.mir.tree.value.encode_value(writer, value.left)
        destack._generated.mir.tree.value.encode_value(writer, value.right)
        writer.write_bool(value.is_signed)
    elif value.kind == "isType":
        writer.write_unsigned(6)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.expected)
    elif value.kind == "variant":
        writer.write_unsigned(7)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        destack._generated.mir.tree.constant.encode_constant(writer, value.expected)
    elif value.kind == "isSubtype":
        writer.write_unsigned(8)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.expected)
    else:
        raise SerdeError("unknown enum variant")


def decode_check_constraint(reader: BinaryReader) -> CheckConstraint:
    """Decode one CheckConstraint."""
    variant = reader.read_number()

    if variant == 0:
        index = destack._generated.mir.tree.value.decode_value(reader)
        length = destack._generated.mir.tree.value.decode_value(reader)
        collection = destack._generated.mir.tree.value.decode_value(reader)
        is_signed = reader.read_bool()

        return CheckConstraintBounds(
            index=index,
            length=length,
            collection=collection,
            is_signed=is_signed,
        )
    elif variant == 1:
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return CheckConstraintNull(
            value=value_,
        )
    elif variant == 2:
        divisor = destack._generated.mir.tree.value.decode_value(reader)

        return CheckConstraintDivZero(
            divisor=divisor,
        )
    elif variant == 3:
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        bit_width = reader.read_byte()
        is_signed = reader.read_bool()

        return CheckConstraintShiftRange(
            value=value_,
            bit_width=bit_width,
            is_signed=is_signed,
        )
    elif variant == 4:
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        to_width = reader.read_byte()
        is_signed = reader.read_bool()

        return CheckConstraintNarrow(
            value=value_,
            to_width=to_width,
            is_signed=is_signed,
        )
    elif variant == 5:
        operator = destack._generated.mir.tree.operator.decode_binary_operator(reader)
        left = destack._generated.mir.tree.value.decode_value(reader)
        right = destack._generated.mir.tree.value.decode_value(reader)
        is_signed = reader.read_bool()

        return CheckConstraintOverflow(
            operator=operator,
            left=left,
            right=right,
            is_signed=is_signed,
        )
    elif variant == 6:
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        expected = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return CheckConstraintIsType(
            value=value_,
            expected=expected,
        )
    elif variant == 7:
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        expected = destack._generated.mir.tree.constant.decode_constant(reader)

        return CheckConstraintVariant(
            value=value_,
            expected=expected,
        )
    elif variant == 8:
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        expected = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return CheckConstraintIsSubtype(
            value=value_,
            expected=expected,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_check_constraint(value: CheckConstraint) -> Json:
    """Return one JSON value for one CheckConstraint."""
    if value.kind == "bounds":
        return {
            "kind": "bounds",
            "index": destack._generated.mir.tree.value.to_json_value(value.index),
            "length": destack._generated.mir.tree.value.to_json_value(value.length),
            "collection": destack._generated.mir.tree.value.to_json_value(
                value.collection
            ),
            "isSigned": value.is_signed,
        }
    elif value.kind == "null":
        return {
            "kind": "null",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "divZero":
        return {
            "kind": "divZero",
            "divisor": destack._generated.mir.tree.value.to_json_value(value.divisor),
        }
    elif value.kind == "shiftRange":
        return {
            "kind": "shiftRange",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "bitWidth": value.bit_width,
            "isSigned": value.is_signed,
        }
    elif value.kind == "narrow":
        return {
            "kind": "narrow",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "toWidth": value.to_width,
            "isSigned": value.is_signed,
        }
    elif value.kind == "overflow":
        return {
            "kind": "overflow",
            "operator": destack._generated.mir.tree.operator.to_json_binary_operator(
                value.operator
            ),
            "left": destack._generated.mir.tree.value.to_json_value(value.left),
            "right": destack._generated.mir.tree.value.to_json_value(value.right),
            "isSigned": value.is_signed,
        }
    elif value.kind == "isType":
        return {
            "kind": "isType",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "expected": destack._generated.mir.tree.node.to_json_local_node_id(
                value.expected
            ),
        }
    elif value.kind == "variant":
        return {
            "kind": "variant",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "expected": destack._generated.mir.tree.constant.to_json_constant(
                value.expected
            ),
        }
    elif value.kind == "isSubtype":
        return {
            "kind": "isSubtype",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "expected": destack._generated.mir.tree.node.to_json_local_node_id(
                value.expected
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_check_constraint(value: Json) -> CheckConstraint:
    """Return one CheckConstraint from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "bounds":
        return CheckConstraintBounds(
            index=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "index")
            ),
            length=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "length")
            ),
            collection=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "collection")
            ),
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "null":
        return CheckConstraintNull(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "divZero":
        return CheckConstraintDivZero(
            divisor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "divisor")
            ),
        )
    elif kind == "shiftRange":
        return CheckConstraintShiftRange(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            bit_width=json_int(json_field(object_, "bitWidth")),
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "narrow":
        return CheckConstraintNarrow(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            to_width=json_int(json_field(object_, "toWidth")),
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "overflow":
        return CheckConstraintOverflow(
            operator=destack._generated.mir.tree.operator.from_json_binary_operator(
                json_field(object_, "operator")
            ),
            left=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "left")
            ),
            right=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "right")
            ),
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "isType":
        return CheckConstraintIsType(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            expected=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "expected")
            ),
        )
    elif kind == "variant":
        return CheckConstraintVariant(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            expected=destack._generated.mir.tree.constant.from_json_constant(
                json_field(object_, "expected")
            ),
        )
    elif kind == "isSubtype":
        return CheckConstraintIsSubtype(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            expected=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "expected")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class SwitchCaseSlice:
    """Compact reference to a switch case list stored in the MIR tree."""

    # start index in the switch case buffer
    start: int
    # number of switch cases in the slice
    count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_switch_case_slice(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SwitchCaseSlice:
        """Decode one SwitchCaseSlice."""
        return decode_switch_case_slice(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_switch_case_slice(self)

    @classmethod
    def from_json(cls, value: Json) -> SwitchCaseSlice:
        """Return one SwitchCaseSlice from one JSON value."""
        return from_json_switch_case_slice(value)


def encode_switch_case_slice(writer: BinaryWriter, value: SwitchCaseSlice) -> None:
    """Encode one SwitchCaseSlice."""
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.count)


def decode_switch_case_slice(reader: BinaryReader) -> SwitchCaseSlice:
    """Decode one SwitchCaseSlice."""
    start = reader.read_number()
    count = reader.read_number()

    return SwitchCaseSlice(
        start=start,
        count=count,
    )


def to_json_switch_case_slice(value: SwitchCaseSlice) -> Json:
    """Return one JSON value for one SwitchCaseSlice."""
    return {
        "start": value.start,
        "count": value.count,
    }


def from_json_switch_case_slice(value: Json) -> SwitchCaseSlice:
    """Return one SwitchCaseSlice from one JSON value."""
    object_ = json_object(value)

    return SwitchCaseSlice(
        start=json_int(json_field(object_, "start")),
        count=json_int(json_field(object_, "count")),
    )


"""Unrecoverable runtime trap kind."""
TrapKind: typing.TypeAlias = typing.Literal["abort"]


def encode_trap_kind(writer: BinaryWriter, value: TrapKind) -> None:
    """Encode one TrapKind."""
    if value == "abort":
        writer.write_unsigned(0)
    else:
        raise SerdeError("unknown enum variant")


def decode_trap_kind(reader: BinaryReader) -> TrapKind:
    """Decode one TrapKind."""
    variant = reader.read_number()

    if variant == 0:
        return "abort"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_trap_kind(value: TrapKind) -> Json:
    """Return one JSON value for one TrapKind."""
    return value


def from_json_trap_kind(value: Json) -> TrapKind:
    """Return one TrapKind from one JSON value."""
    variant = json_string(value)

    if variant == "abort":
        return "abort"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class SwitchCase:
    """One case arm for a switch terminator."""

    # the matched case value
    value: int
    # the target block for this case
    target: BlockTarget

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_switch_case(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SwitchCase:
        """Decode one SwitchCase."""
        return decode_switch_case(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_switch_case(self)

    @classmethod
    def from_json(cls, value: Json) -> SwitchCase:
        """Return one SwitchCase from one JSON value."""
        return from_json_switch_case(value)


def encode_switch_case(writer: BinaryWriter, value: SwitchCase) -> None:
    """Encode one SwitchCase."""
    writer.write_signed(value.value)
    encode_block_target(writer, value.target)


def decode_switch_case(reader: BinaryReader) -> SwitchCase:
    """Decode one SwitchCase."""
    value_ = reader.read_signed_number()
    target = decode_block_target(reader)

    return SwitchCase(
        value=value_,
        target=target,
    )


def to_json_switch_case(value: SwitchCase) -> Json:
    """Return one JSON value for one SwitchCase."""
    return {
        "value": value.value,
        "target": to_json_block_target(value.target),
    }


def from_json_switch_case(value: Json) -> SwitchCase:
    """Return one SwitchCase from one JSON value."""
    object_ = json_object(value)

    return SwitchCase(
        value=json_int(json_field(object_, "value")),
        target=from_json_block_target(json_field(object_, "target")),
    )


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
    "CheckConstraintIsType",
    "CheckConstraintVariant",
    "CheckConstraintIsSubtype",
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
