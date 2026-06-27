# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.mir.table.memory
import destack._generated.mir.tree.call
import destack._generated.mir.tree.memory
import destack._generated.mir.tree.node


@dataclass(frozen=True, slots=True)
class EffectTable:
    """Function and call effect tables for one MIR module."""

    # effects keyed by function id
    functions: Mapping[destack._generated.mir.tree.node.LocalNodeId, FunctionEffect]
    # effects keyed by callsite
    calls: Mapping[destack._generated.mir.tree.call.CallSite, CallEffect]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_effect_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> EffectTable:
        """Decode one EffectTable."""
        return decode_effect_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_effect_table(self)

    @classmethod
    def from_json(cls, value: Json) -> EffectTable:
        """Return one EffectTable from one JSON value."""
        return from_json_effect_table(value)


def encode_effect_table(writer: BinaryWriter, value: EffectTable) -> None:
    """Encode one EffectTable."""
    entries_value_functions_0 = []
    for key_value_functions_0, item_value_functions_0 in value.functions.items():

        def write_key_value_functions_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_functions_0
            )

        key_bytes = nested_bytes(write_key_value_functions_0)
        entries_value_functions_0.append(
            (key_value_functions_0, item_value_functions_0, key_bytes)
        )
    entries_value_functions_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_functions_0))
    for entry_value_functions_0 in entries_value_functions_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_functions_0[0]
        )
        encode_function_effect(writer, entry_value_functions_0[1])
    entries_value_calls_0 = []
    for key_value_calls_0, item_value_calls_0 in value.calls.items():

        def write_key_value_calls_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.call.encode_call_site(writer, key_value_calls_0)

        key_bytes = nested_bytes(write_key_value_calls_0)
        entries_value_calls_0.append((key_value_calls_0, item_value_calls_0, key_bytes))
    entries_value_calls_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_calls_0))
    for entry_value_calls_0 in entries_value_calls_0:
        destack._generated.mir.tree.call.encode_call_site(
            writer, entry_value_calls_0[0]
        )
        encode_call_effect(writer, entry_value_calls_0[1])


def decode_effect_table(reader: BinaryReader) -> EffectTable:
    """Decode one EffectTable."""
    functions = {
        destack._generated.mir.tree.node.decode_local_node_id(
            reader
        ): decode_function_effect(reader)
        for _ in range(reader.read_number())
    }
    calls = {
        destack._generated.mir.tree.call.decode_call_site(reader): decode_call_effect(
            reader
        )
        for _ in range(reader.read_number())
    }

    return EffectTable(
        functions=functions,
        calls=calls,
    )


def to_json_effect_table(value: EffectTable) -> Json:
    """Return one JSON value for one EffectTable."""
    return {
        "functions": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                to_json_function_effect(item_0),
            ]
            for key_0, item_0 in value.functions.items()
        ],
        "calls": [
            [
                destack._generated.mir.tree.call.to_json_call_site(key_0),
                to_json_call_effect(item_0),
            ]
            for key_0, item_0 in value.calls.items()
        ],
    }


def from_json_effect_table(value: Json) -> EffectTable:
    """Return one EffectTable from one JSON value."""
    object_ = json_object(value)

    return EffectTable(
        functions={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): from_json_function_effect(item_0)
            for key_0, item_0 in json_array(json_field(object_, "functions"))
        },
        calls={
            destack._generated.mir.tree.call.from_json_call_site(
                key_0
            ): from_json_call_effect(item_0)
            for key_0, item_0 in json_array(json_field(object_, "calls"))
        },
    )


@dataclass(frozen=True, slots=True)
class FunctionEffect:
    """Effects for one function body or declaration."""

    # memory touched by this function
    memory: MemoryEffect
    # behavioral effects of this function
    behavior: FunctionBehavior

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_effect(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionEffect:
        """Decode one FunctionEffect."""
        return decode_function_effect(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_effect(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionEffect:
        """Return one FunctionEffect from one JSON value."""
        return from_json_function_effect(value)


def encode_function_effect(writer: BinaryWriter, value: FunctionEffect) -> None:
    """Encode one FunctionEffect."""
    encode_memory_effect(writer, value.memory)
    encode_function_behavior(writer, value.behavior)


def decode_function_effect(reader: BinaryReader) -> FunctionEffect:
    """Decode one FunctionEffect."""
    memory = decode_memory_effect(reader)
    behavior = decode_function_behavior(reader)

    return FunctionEffect(
        memory=memory,
        behavior=behavior,
    )


def to_json_function_effect(value: FunctionEffect) -> Json:
    """Return one JSON value for one FunctionEffect."""
    return {
        "memory": to_json_memory_effect(value.memory),
        "behavior": to_json_function_behavior(value.behavior),
    }


def from_json_function_effect(value: Json) -> FunctionEffect:
    """Return one FunctionEffect from one JSON value."""
    object_ = json_object(value)

    return FunctionEffect(
        memory=from_json_memory_effect(json_field(object_, "memory")),
        behavior=from_json_function_behavior(json_field(object_, "behavior")),
    )


@dataclass(frozen=True, slots=True)
class MemoryEffect:
    """Memory access effect for a call or operation."""

    # storage regions this operation may read
    read: destack._generated.mir.tree.memory.StorageSet
    # storage regions this operation may write
    write: destack._generated.mir.tree.memory.StorageSet

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
    destack._generated.mir.tree.memory.encode_storage_set(writer, value.read)
    destack._generated.mir.tree.memory.encode_storage_set(writer, value.write)


def decode_memory_effect(reader: BinaryReader) -> MemoryEffect:
    """Decode one MemoryEffect."""
    read = destack._generated.mir.tree.memory.decode_storage_set(reader)
    write = destack._generated.mir.tree.memory.decode_storage_set(reader)

    return MemoryEffect(
        read=read,
        write=write,
    )


def to_json_memory_effect(value: MemoryEffect) -> Json:
    """Return one JSON value for one MemoryEffect."""
    return {
        "read": destack._generated.mir.tree.memory.to_json_storage_set(value.read),
        "write": destack._generated.mir.tree.memory.to_json_storage_set(value.write),
    }


def from_json_memory_effect(value: Json) -> MemoryEffect:
    """Return one MemoryEffect from one JSON value."""
    object_ = json_object(value)

    return MemoryEffect(
        read=destack._generated.mir.tree.memory.from_json_storage_set(
            json_field(object_, "read")
        ),
        write=destack._generated.mir.tree.memory.from_json_storage_set(
            json_field(object_, "write")
        ),
    )


@dataclass(frozen=True, slots=True)
class FunctionBehavior:
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


@dataclass(frozen=True, slots=True)
class CallEffect:
    """Effects for one callsite."""

    # memory touched by this call
    memory: MemoryEffect
    # behavioral effects of this call
    behavior: FunctionBehavior
    # resolved direct target when dispatch analysis proves one
    target: destack._generated.mir.tree.node.LocalNodeId | None
    # argument memory behavior when known
    arguments: Sequence[destack._generated.mir.table.memory.CallArgumentEffect]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_effect(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallEffect:
        """Decode one CallEffect."""
        return decode_call_effect(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_effect(self)

    @classmethod
    def from_json(cls, value: Json) -> CallEffect:
        """Return one CallEffect from one JSON value."""
        return from_json_call_effect(value)


def encode_call_effect(writer: BinaryWriter, value: CallEffect) -> None:
    """Encode one CallEffect."""
    encode_memory_effect(writer, value.memory)
    encode_function_behavior(writer, value.behavior)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.target)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.mir.table.memory.encode_call_argument_effect(
            writer, item_value_arguments_0
        )


def decode_call_effect(reader: BinaryReader) -> CallEffect:
    """Decode one CallEffect."""
    memory = decode_memory_effect(reader)
    behavior = decode_function_behavior(reader)
    target = reader.read_option(
        lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
    )
    arguments = [
        destack._generated.mir.table.memory.decode_call_argument_effect(reader)
        for _ in range(reader.read_number())
    ]

    return CallEffect(
        memory=memory,
        behavior=behavior,
        target=target,
        arguments=arguments,
    )


def to_json_call_effect(value: CallEffect) -> Json:
    """Return one JSON value for one CallEffect."""
    return {
        "memory": to_json_memory_effect(value.memory),
        "behavior": to_json_function_behavior(value.behavior),
        **(
            {}
            if value.target is None
            else {
                "target": destack._generated.mir.tree.node.to_json_local_node_id(
                    value.target
                )
            }
        ),
        "arguments": [
            destack._generated.mir.table.memory.to_json_call_argument_effect(item_0)
            for item_0 in value.arguments
        ],
    }


def from_json_call_effect(value: Json) -> CallEffect:
    """Return one CallEffect from one JSON value."""
    object_ = json_object(value)

    return CallEffect(
        memory=from_json_memory_effect(json_field(object_, "memory")),
        behavior=from_json_function_behavior(json_field(object_, "behavior")),
        target=json_optional(
            object_,
            "target",
            lambda value: destack._generated.mir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        arguments=[
            destack._generated.mir.table.memory.from_json_call_argument_effect(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


__all__ = [
    "EffectTable",
    "encode_effect_table",
    "decode_effect_table",
    "to_json_effect_table",
    "from_json_effect_table",
    "FunctionEffect",
    "encode_function_effect",
    "decode_function_effect",
    "to_json_function_effect",
    "from_json_function_effect",
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
    "CallEffect",
    "encode_call_effect",
    "decode_call_effect",
    "to_json_call_effect",
    "from_json_call_effect",
]
