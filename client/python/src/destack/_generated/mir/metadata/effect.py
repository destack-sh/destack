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
    json_object,
    json_string,
)

from destack._impl.mir.metadata.effect import (
    MemoryEffectImpl,
)
from destack._impl.mir.metadata.effect import (
    FunctionBehaviorImpl,
)

import destack._generated.mir.tree.memory


@dataclass(frozen=True, slots=True)
class MemoryEffect(MemoryEffectImpl):
    """Memory effect summary for a call or operation."""

    # whether the operation may read memory
    reads: bool
    # whether the operation may write memory
    writes: bool
    # the memory spaces that may be accessed
    spaces: destack._generated.mir.tree.memory.SpaceSet

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_effect(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryEffect:
        """Decode one MemoryEffect."""
        return decode_memory_effect(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_effect(self)

    @classmethod
    def from_json(cls, value: Json) -> MemoryEffect:
        """Return one MemoryEffect from one JSON value."""
        return from_json_memory_effect(value)


def encode_memory_effect(writer: BinaryWriter, value: MemoryEffect) -> None:
    """Encode one MemoryEffect."""
    writer.write_bool(value.reads)
    writer.write_bool(value.writes)
    destack._generated.mir.tree.memory.encode_space_set(writer, value.spaces)


def decode_memory_effect(reader: BinaryReader) -> MemoryEffect:
    """Decode one MemoryEffect."""
    reads = reader.read_bool()
    writes = reader.read_bool()
    spaces = destack._generated.mir.tree.memory.decode_space_set(reader)

    return MemoryEffect(
        reads=reads,
        writes=writes,
        spaces=spaces,
    )


def to_json_memory_effect(value: MemoryEffect) -> Json:
    """Return one JSON value for one MemoryEffect."""
    return {
        "reads": value.reads,
        "writes": value.writes,
        "spaces": destack._generated.mir.tree.memory.to_json_space_set(value.spaces),
    }


def from_json_memory_effect(value: Json) -> MemoryEffect:
    """Return one MemoryEffect from one JSON value."""
    object_ = json_object(value)

    return MemoryEffect(
        reads=json_bool(json_field(object_, "reads")),
        writes=json_bool(json_field(object_, "writes")),
        spaces=destack._generated.mir.tree.memory.from_json_space_set(
            json_field(object_, "spaces")
        ),
    )


@dataclass(frozen=True, slots=True)
class FunctionBehavior(FunctionBehaviorImpl):
    """Behavioral effects for calls and functions."""

    # determinism for this operation
    determinism: Determinism
    # whether this operation may suspend execution
    suspend: SuspendBehavior
    # panic behavior for this operation
    panic: PanicBehavior
    # return behavior for this operation
    return_behavior: ReturnBehavior
    # whether optimization must not duplicate this operation
    must_not_duplicate: bool
    # whether this operation may allocate storage
    allocates: bool
    # whether this operation may free storage
    frees: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_behavior(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionBehavior:
        """Decode one FunctionBehavior."""
        return decode_function_behavior(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_behavior(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionBehavior:
        """Return one FunctionBehavior from one JSON value."""
        return from_json_function_behavior(value)


def encode_function_behavior(writer: BinaryWriter, value: FunctionBehavior) -> None:
    """Encode one FunctionBehavior."""
    encode_determinism(writer, value.determinism)
    encode_suspend_behavior(writer, value.suspend)
    encode_panic_behavior(writer, value.panic)
    encode_return_behavior(writer, value.return_behavior)
    writer.write_bool(value.must_not_duplicate)
    writer.write_bool(value.allocates)
    writer.write_bool(value.frees)


def decode_function_behavior(reader: BinaryReader) -> FunctionBehavior:
    """Decode one FunctionBehavior."""
    determinism = decode_determinism(reader)
    suspend = decode_suspend_behavior(reader)
    panic = decode_panic_behavior(reader)
    return_behavior = decode_return_behavior(reader)
    must_not_duplicate = reader.read_bool()
    allocates = reader.read_bool()
    frees = reader.read_bool()

    return FunctionBehavior(
        determinism=determinism,
        suspend=suspend,
        panic=panic,
        return_behavior=return_behavior,
        must_not_duplicate=must_not_duplicate,
        allocates=allocates,
        frees=frees,
    )


def to_json_function_behavior(value: FunctionBehavior) -> Json:
    """Return one JSON value for one FunctionBehavior."""
    return {
        "determinism": to_json_determinism(value.determinism),
        "suspend": to_json_suspend_behavior(value.suspend),
        "panic": to_json_panic_behavior(value.panic),
        "returnBehavior": to_json_return_behavior(value.return_behavior),
        "mustNotDuplicate": value.must_not_duplicate,
        "allocates": value.allocates,
        "frees": value.frees,
    }


def from_json_function_behavior(value: Json) -> FunctionBehavior:
    """Return one FunctionBehavior from one JSON value."""
    object_ = json_object(value)

    return FunctionBehavior(
        determinism=from_json_determinism(json_field(object_, "determinism")),
        suspend=from_json_suspend_behavior(json_field(object_, "suspend")),
        panic=from_json_panic_behavior(json_field(object_, "panic")),
        return_behavior=from_json_return_behavior(
            json_field(object_, "returnBehavior")
        ),
        must_not_duplicate=json_bool(json_field(object_, "mustNotDuplicate")),
        allocates=json_bool(json_field(object_, "allocates")),
        frees=json_bool(json_field(object_, "frees")),
    )


"""Determinism for a call or function."""
Determinism: typing.TypeAlias = (
    typing.Literal["deterministic"] | typing.Literal["nonDeterministic"]
)


def encode_determinism(writer: BinaryWriter, value: Determinism) -> None:
    """Encode one Determinism."""
    if value == "deterministic":
        writer.write_unsigned(0)
    elif value == "nonDeterministic":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_determinism(reader: BinaryReader) -> Determinism:
    """Decode one Determinism."""
    variant = reader.read_number()

    if variant == 0:
        return "deterministic"
    elif variant == 1:
        return "nonDeterministic"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_determinism(value: Determinism) -> Json:
    """Return one JSON value for one Determinism."""
    return value


def from_json_determinism(value: Json) -> Determinism:
    """Return one Determinism from one JSON value."""
    variant = json_string(value)

    if variant == "deterministic":
        return "deterministic"
    elif variant == "nonDeterministic":
        return "nonDeterministic"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Suspend behavior for a call or function."""
SuspendBehavior: typing.TypeAlias = (
    typing.Literal["cannotSuspend"] | typing.Literal["maySuspend"]
)


def encode_suspend_behavior(writer: BinaryWriter, value: SuspendBehavior) -> None:
    """Encode one SuspendBehavior."""
    if value == "cannotSuspend":
        writer.write_unsigned(0)
    elif value == "maySuspend":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_suspend_behavior(reader: BinaryReader) -> SuspendBehavior:
    """Decode one SuspendBehavior."""
    variant = reader.read_number()

    if variant == 0:
        return "cannotSuspend"
    elif variant == 1:
        return "maySuspend"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_suspend_behavior(value: SuspendBehavior) -> Json:
    """Return one JSON value for one SuspendBehavior."""
    return value


def from_json_suspend_behavior(value: Json) -> SuspendBehavior:
    """Return one SuspendBehavior from one JSON value."""
    variant = json_string(value)

    if variant == "cannotSuspend":
        return "cannotSuspend"
    elif variant == "maySuspend":
        return "maySuspend"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Panic behavior for a call or function."""
PanicBehavior: typing.TypeAlias = (
    typing.Literal["cannotPanic"] | typing.Literal["mayPanic"]
)


def encode_panic_behavior(writer: BinaryWriter, value: PanicBehavior) -> None:
    """Encode one PanicBehavior."""
    if value == "cannotPanic":
        writer.write_unsigned(0)
    elif value == "mayPanic":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_panic_behavior(reader: BinaryReader) -> PanicBehavior:
    """Decode one PanicBehavior."""
    variant = reader.read_number()

    if variant == 0:
        return "cannotPanic"
    elif variant == 1:
        return "mayPanic"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_panic_behavior(value: PanicBehavior) -> Json:
    """Return one JSON value for one PanicBehavior."""
    return value


def from_json_panic_behavior(value: Json) -> PanicBehavior:
    """Return one PanicBehavior from one JSON value."""
    variant = json_string(value)

    if variant == "cannotPanic":
        return "cannotPanic"
    elif variant == "mayPanic":
        return "mayPanic"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Return behavior for a call or function."""
ReturnBehavior: typing.TypeAlias = (
    typing.Literal["mayReturn"]
    | typing.Literal["noReturn"]
    | typing.Literal["willReturn"]
)


def encode_return_behavior(writer: BinaryWriter, value: ReturnBehavior) -> None:
    """Encode one ReturnBehavior."""
    if value == "mayReturn":
        writer.write_unsigned(0)
    elif value == "noReturn":
        writer.write_unsigned(1)
    elif value == "willReturn":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_return_behavior(reader: BinaryReader) -> ReturnBehavior:
    """Decode one ReturnBehavior."""
    variant = reader.read_number()

    if variant == 0:
        return "mayReturn"
    elif variant == 1:
        return "noReturn"
    elif variant == 2:
        return "willReturn"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_return_behavior(value: ReturnBehavior) -> Json:
    """Return one JSON value for one ReturnBehavior."""
    return value


def from_json_return_behavior(value: Json) -> ReturnBehavior:
    """Return one ReturnBehavior from one JSON value."""
    variant = json_string(value)

    if variant == "mayReturn":
        return "mayReturn"
    elif variant == "noReturn":
        return "noReturn"
    elif variant == "willReturn":
        return "willReturn"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "MemoryEffect",
    "encode_memory_effect",
    "decode_memory_effect",
    "to_json_memory_effect",
    "from_json_memory_effect",
    "FunctionBehavior",
    "encode_function_behavior",
    "decode_function_behavior",
    "to_json_function_behavior",
    "from_json_function_behavior",
    "Determinism",
    "encode_determinism",
    "decode_determinism",
    "to_json_determinism",
    "from_json_determinism",
    "SuspendBehavior",
    "encode_suspend_behavior",
    "decode_suspend_behavior",
    "to_json_suspend_behavior",
    "from_json_suspend_behavior",
    "PanicBehavior",
    "encode_panic_behavior",
    "decode_panic_behavior",
    "to_json_panic_behavior",
    "from_json_panic_behavior",
    "ReturnBehavior",
    "encode_return_behavior",
    "decode_return_behavior",
    "to_json_return_behavior",
    "from_json_return_behavior",
]
