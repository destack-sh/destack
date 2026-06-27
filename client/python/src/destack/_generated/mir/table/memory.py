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
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.mir.tree.memory
import destack._generated.mir.tree.node
import destack._generated.mir.tree.value


@dataclass(frozen=True, slots=True)
class MemoryTable:
    """Table of explicit memory accesses."""

    # memory accesses keyed by instruction id
    memory_accesses_by_instruction_id: Mapping[
        destack._generated.mir.tree.node.LocalNodeId, Sequence[MemoryAccess]
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryTable:
        """Decode one MemoryTable."""
        return decode_memory_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_table(self)

    @classmethod
    def from_json(cls, value: Json) -> MemoryTable:
        """Return one MemoryTable from one JSON value."""
        return from_json_memory_table(value)


def encode_memory_table(writer: BinaryWriter, value: MemoryTable) -> None:
    """Encode one MemoryTable."""
    entries_value_memory_accesses_by_instruction_id_0 = []
    for (
        key_value_memory_accesses_by_instruction_id_0,
        item_value_memory_accesses_by_instruction_id_0,
    ) in value.memory_accesses_by_instruction_id.items():

        def write_key_value_memory_accesses_by_instruction_id_0(
            writer: BinaryWriter,
        ) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_memory_accesses_by_instruction_id_0
            )

        key_bytes = nested_bytes(write_key_value_memory_accesses_by_instruction_id_0)
        entries_value_memory_accesses_by_instruction_id_0.append(
            (
                key_value_memory_accesses_by_instruction_id_0,
                item_value_memory_accesses_by_instruction_id_0,
                key_bytes,
            )
        )
    entries_value_memory_accesses_by_instruction_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_memory_accesses_by_instruction_id_0))
    for (
        entry_value_memory_accesses_by_instruction_id_0
    ) in entries_value_memory_accesses_by_instruction_id_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_memory_accesses_by_instruction_id_0[0]
        )
        writer.write_unsigned(len(entry_value_memory_accesses_by_instruction_id_0[1]))
        for (
            item_entry_value_memory_accesses_by_instruction_id_0_1_1
        ) in entry_value_memory_accesses_by_instruction_id_0[1]:
            encode_memory_access(
                writer, item_entry_value_memory_accesses_by_instruction_id_0_1_1
            )


def decode_memory_table(reader: BinaryReader) -> MemoryTable:
    """Decode one MemoryTable."""
    memory_accesses_by_instruction_id = {
        destack._generated.mir.tree.node.decode_local_node_id(reader): [
            decode_memory_access(reader) for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }

    return MemoryTable(
        memory_accesses_by_instruction_id=memory_accesses_by_instruction_id,
    )


def to_json_memory_table(value: MemoryTable) -> Json:
    """Return one JSON value for one MemoryTable."""
    return {
        "memoryAccessesByInstructionId": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                [to_json_memory_access(item_1) for item_1 in item_0],
            ]
            for key_0, item_0 in value.memory_accesses_by_instruction_id.items()
        ],
    }


def from_json_memory_table(value: Json) -> MemoryTable:
    """Return one MemoryTable from one JSON value."""
    object_ = json_object(value)

    return MemoryTable(
        memory_accesses_by_instruction_id={
            destack._generated.mir.tree.node.from_json_local_node_id(key_0): [
                from_json_memory_access(item_1) for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(
                json_field(object_, "memoryAccessesByInstructionId")
            )
        },
    )


@dataclass(frozen=True, slots=True)
class MemoryAccess:
    """Explicit memory access attached to one instruction."""

    # the operation performed
    operation: MemoryOperation
    # the access target
    target: MemoryTarget
    # the number of bytes accessed when known
    byte_len: int | None
    # alignment in bytes, when known
    alignment_bytes: int | None
    # the ordering constraints on this access
    order: MemoryAccessOrder

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_access(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryAccess:
        """Decode one MemoryAccess."""
        return decode_memory_access(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_access(self)

    @classmethod
    def from_json(cls, value: Json) -> MemoryAccess:
        """Return one MemoryAccess from one JSON value."""
        return from_json_memory_access(value)


def encode_memory_access(writer: BinaryWriter, value: MemoryAccess) -> None:
    """Encode one MemoryAccess."""
    encode_memory_operation(writer, value.operation)
    encode_memory_target(writer, value.target)
    if value.byte_len is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.byte_len)
    if value.alignment_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.alignment_bytes)
    encode_memory_access_order(writer, value.order)


def decode_memory_access(reader: BinaryReader) -> MemoryAccess:
    """Decode one MemoryAccess."""
    operation = decode_memory_operation(reader)
    target = decode_memory_target(reader)
    byte_len = reader.read_option(lambda: reader.read_number())
    alignment_bytes = reader.read_option(lambda: reader.read_number())
    order = decode_memory_access_order(reader)

    return MemoryAccess(
        operation=operation,
        target=target,
        byte_len=byte_len,
        alignment_bytes=alignment_bytes,
        order=order,
    )


def to_json_memory_access(value: MemoryAccess) -> Json:
    """Return one JSON value for one MemoryAccess."""
    return {
        "operation": to_json_memory_operation(value.operation),
        "target": to_json_memory_target(value.target),
        **({} if value.byte_len is None else {"byteLen": value.byte_len}),
        **(
            {}
            if value.alignment_bytes is None
            else {"alignmentBytes": value.alignment_bytes}
        ),
        "order": to_json_memory_access_order(value.order),
    }


def from_json_memory_access(value: Json) -> MemoryAccess:
    """Return one MemoryAccess from one JSON value."""
    object_ = json_object(value)

    return MemoryAccess(
        operation=from_json_memory_operation(json_field(object_, "operation")),
        target=from_json_memory_target(json_field(object_, "target")),
        byte_len=json_optional(object_, "byteLen", lambda value: json_int(value)),
        alignment_bytes=json_optional(
            object_, "alignmentBytes", lambda value: json_int(value)
        ),
        order=from_json_memory_access_order(json_field(object_, "order")),
    )


"""The operation performed by a memory access."""
MemoryOperation: typing.TypeAlias = (
    typing.Literal["read"] | typing.Literal["write"] | typing.Literal["readWrite"]
)


def encode_memory_operation(writer: BinaryWriter, value: MemoryOperation) -> None:
    """Encode one MemoryOperation."""
    if value == "read":
        writer.write_unsigned(0)
    elif value == "write":
        writer.write_unsigned(1)
    elif value == "readWrite":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_memory_operation(reader: BinaryReader) -> MemoryOperation:
    """Decode one MemoryOperation."""
    variant = reader.read_number()

    if variant == 0:
        return "read"
    elif variant == 1:
        return "write"
    elif variant == 2:
        return "readWrite"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_memory_operation(value: MemoryOperation) -> Json:
    """Return one JSON value for one MemoryOperation."""
    return value


def from_json_memory_operation(value: Json) -> MemoryOperation:
    """Return one MemoryOperation from one JSON value."""
    variant = json_string(value)

    if variant == "read":
        return "read"
    elif variant == "write":
        return "write"
    elif variant == "readWrite":
        return "readWrite"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class MemoryTargetReference:
    """Access through a reference value."""

    reference: destack._generated.mir.tree.value.Value
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_target(self)


@dataclass(frozen=True, slots=True)
class MemoryTargetLocal:
    """Access through a local slot."""

    local: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_target(self)


@dataclass(frozen=True, slots=True)
class MemoryTargetGlobal:
    """Access through a global."""

    global_: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["global"] = "global"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_target(self)


"""Target of one memory access."""
MemoryTarget: typing.TypeAlias = (
    MemoryTargetReference | MemoryTargetLocal | MemoryTargetGlobal
)


def encode_memory_target(writer: BinaryWriter, value: MemoryTarget) -> None:
    """Encode one MemoryTarget."""
    if value.kind == "reference":
        writer.write_unsigned(0)
        destack._generated.mir.tree.value.encode_value(writer, value.reference)
    elif value.kind == "local":
        writer.write_unsigned(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.local)
    elif value.kind == "global":
        writer.write_unsigned(2)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.global_)
    else:
        raise SerdeError("unknown enum variant")


def decode_memory_target(reader: BinaryReader) -> MemoryTarget:
    """Decode one MemoryTarget."""
    variant = reader.read_number()

    if variant == 0:
        reference = destack._generated.mir.tree.value.decode_value(reader)

        return MemoryTargetReference(reference=reference)
    elif variant == 1:
        local = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return MemoryTargetLocal(local=local)
    elif variant == 2:
        global_ = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return MemoryTargetGlobal(global_=global_)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_memory_target(value: MemoryTarget) -> Json:
    """Return one JSON value for one MemoryTarget."""
    if value.kind == "reference":
        return {
            "kind": "reference",
            "reference": destack._generated.mir.tree.value.to_json_value(
                value.reference
            ),
        }
    elif value.kind == "local":
        return {
            "kind": "local",
            "local": destack._generated.mir.tree.node.to_json_local_node_id(
                value.local
            ),
        }
    elif value.kind == "global":
        return {
            "kind": "global",
            "global": destack._generated.mir.tree.node.to_json_local_node_id(
                value.global_
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_memory_target(value: Json) -> MemoryTarget:
    """Return one MemoryTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "reference":
        return MemoryTargetReference(
            reference=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "reference")
            )
        )
    elif kind == "local":
        return MemoryTargetLocal(
            local=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "local")
            )
        )
    elif kind == "global":
        return MemoryTargetGlobal(
            global_=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "global")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class MemoryAccessOrderPlain:
    """Ordinary memory access."""

    kind: typing.Literal["plain"] = "plain"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_access_order(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_access_order(self)


@dataclass(frozen=True, slots=True)
class MemoryAccessOrderVolatile:
    """Externally observable memory access."""

    kind: typing.Literal["volatile"] = "volatile"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_access_order(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_access_order(self)


@dataclass(frozen=True, slots=True)
class MemoryAccessOrderAtomic:
    """Atomic memory access."""

    atomic: destack._generated.mir.tree.memory.AtomicAccess
    kind: typing.Literal["atomic"] = "atomic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_access_order(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_access_order(self)


"""Ordering constraints for one memory access."""
MemoryAccessOrder: typing.TypeAlias = (
    MemoryAccessOrderPlain | MemoryAccessOrderVolatile | MemoryAccessOrderAtomic
)


def encode_memory_access_order(writer: BinaryWriter, value: MemoryAccessOrder) -> None:
    """Encode one MemoryAccessOrder."""
    if value.kind == "plain":
        writer.write_unsigned(0)
    elif value.kind == "volatile":
        writer.write_unsigned(1)
    elif value.kind == "atomic":
        writer.write_unsigned(2)
        destack._generated.mir.tree.memory.encode_atomic_access(writer, value.atomic)
    else:
        raise SerdeError("unknown enum variant")


def decode_memory_access_order(reader: BinaryReader) -> MemoryAccessOrder:
    """Decode one MemoryAccessOrder."""
    variant = reader.read_number()

    if variant == 0:
        return MemoryAccessOrderPlain()
    elif variant == 1:
        return MemoryAccessOrderVolatile()
    elif variant == 2:
        atomic = destack._generated.mir.tree.memory.decode_atomic_access(reader)

        return MemoryAccessOrderAtomic(atomic=atomic)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_memory_access_order(value: MemoryAccessOrder) -> Json:
    """Return one JSON value for one MemoryAccessOrder."""
    if value.kind == "plain":
        return {
            "kind": "plain",
        }
    elif value.kind == "volatile":
        return {
            "kind": "volatile",
        }
    elif value.kind == "atomic":
        return {
            "kind": "atomic",
            "atomic": destack._generated.mir.tree.memory.to_json_atomic_access(
                value.atomic
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_memory_access_order(value: Json) -> MemoryAccessOrder:
    """Return one MemoryAccessOrder from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "plain":
        return MemoryAccessOrderPlain()
    elif kind == "volatile":
        return MemoryAccessOrderVolatile()
    elif kind == "atomic":
        return MemoryAccessOrderAtomic(
            atomic=destack._generated.mir.tree.memory.from_json_atomic_access(
                json_field(object_, "atomic")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class CallArgumentEffect:
    """Summary behavior for one argument passed to a bodyless call."""

    # access mode for this argument
    access: ArgumentAccess
    # escape behavior for this argument
    escape: ArgumentEscape

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_argument_effect(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallArgumentEffect:
        """Decode one CallArgumentEffect."""
        return decode_call_argument_effect(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_argument_effect(self)

    @classmethod
    def from_json(cls, value: Json) -> CallArgumentEffect:
        """Return one CallArgumentEffect from one JSON value."""
        return from_json_call_argument_effect(value)


def encode_call_argument_effect(
    writer: BinaryWriter, value: CallArgumentEffect
) -> None:
    """Encode one CallArgumentEffect."""
    encode_argument_access(writer, value.access)
    encode_argument_escape(writer, value.escape)


def decode_call_argument_effect(reader: BinaryReader) -> CallArgumentEffect:
    """Decode one CallArgumentEffect."""
    access = decode_argument_access(reader)
    escape = decode_argument_escape(reader)

    return CallArgumentEffect(
        access=access,
        escape=escape,
    )


def to_json_call_argument_effect(value: CallArgumentEffect) -> Json:
    """Return one JSON value for one CallArgumentEffect."""
    return {
        "access": to_json_argument_access(value.access),
        "escape": to_json_argument_escape(value.escape),
    }


def from_json_call_argument_effect(value: Json) -> CallArgumentEffect:
    """Return one CallArgumentEffect from one JSON value."""
    object_ = json_object(value)

    return CallArgumentEffect(
        access=from_json_argument_access(json_field(object_, "access")),
        escape=from_json_argument_escape(json_field(object_, "escape")),
    )


"""Access mode for a bodyless call pointer argument."""
ArgumentAccess: typing.TypeAlias = (
    typing.Literal["none"]
    | typing.Literal["read"]
    | typing.Literal["write"]
    | typing.Literal["readWrite"]
)


def encode_argument_access(writer: BinaryWriter, value: ArgumentAccess) -> None:
    """Encode one ArgumentAccess."""
    if value == "none":
        writer.write_unsigned(0)
    elif value == "read":
        writer.write_unsigned(1)
    elif value == "write":
        writer.write_unsigned(2)
    elif value == "readWrite":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_argument_access(reader: BinaryReader) -> ArgumentAccess:
    """Decode one ArgumentAccess."""
    variant = reader.read_number()

    if variant == 0:
        return "none"
    elif variant == 1:
        return "read"
    elif variant == 2:
        return "write"
    elif variant == 3:
        return "readWrite"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_argument_access(value: ArgumentAccess) -> Json:
    """Return one JSON value for one ArgumentAccess."""
    return value


def from_json_argument_access(value: Json) -> ArgumentAccess:
    """Return one ArgumentAccess from one JSON value."""
    variant = json_string(value)

    if variant == "none":
        return "none"
    elif variant == "read":
        return "read"
    elif variant == "write":
        return "write"
    elif variant == "readWrite":
        return "readWrite"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Escape behavior for a bodyless call argument."""
ArgumentEscape: typing.TypeAlias = (
    typing.Literal["none"] | typing.Literal["return"] | typing.Literal["escape"]
)


def encode_argument_escape(writer: BinaryWriter, value: ArgumentEscape) -> None:
    """Encode one ArgumentEscape."""
    if value == "none":
        writer.write_unsigned(0)
    elif value == "return":
        writer.write_unsigned(1)
    elif value == "escape":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_argument_escape(reader: BinaryReader) -> ArgumentEscape:
    """Decode one ArgumentEscape."""
    variant = reader.read_number()

    if variant == 0:
        return "none"
    elif variant == 1:
        return "return"
    elif variant == 2:
        return "escape"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_argument_escape(value: ArgumentEscape) -> Json:
    """Return one JSON value for one ArgumentEscape."""
    return value


def from_json_argument_escape(value: Json) -> ArgumentEscape:
    """Return one ArgumentEscape from one JSON value."""
    variant = json_string(value)

    if variant == "none":
        return "none"
    elif variant == "return":
        return "return"
    elif variant == "escape":
        return "escape"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "MemoryTable",
    "encode_memory_table",
    "decode_memory_table",
    "to_json_memory_table",
    "from_json_memory_table",
    "MemoryAccess",
    "encode_memory_access",
    "decode_memory_access",
    "to_json_memory_access",
    "from_json_memory_access",
    "MemoryOperation",
    "encode_memory_operation",
    "decode_memory_operation",
    "to_json_memory_operation",
    "from_json_memory_operation",
    "MemoryTarget",
    "encode_memory_target",
    "decode_memory_target",
    "to_json_memory_target",
    "from_json_memory_target",
    "MemoryTargetReference",
    "MemoryTargetLocal",
    "MemoryTargetGlobal",
    "MemoryAccessOrder",
    "encode_memory_access_order",
    "decode_memory_access_order",
    "to_json_memory_access_order",
    "from_json_memory_access_order",
    "MemoryAccessOrderPlain",
    "MemoryAccessOrderVolatile",
    "MemoryAccessOrderAtomic",
    "CallArgumentEffect",
    "encode_call_argument_effect",
    "decode_call_argument_effect",
    "to_json_call_argument_effect",
    "from_json_call_argument_effect",
    "ArgumentAccess",
    "encode_argument_access",
    "decode_argument_access",
    "to_json_argument_access",
    "from_json_argument_access",
    "ArgumentEscape",
    "encode_argument_escape",
    "decode_argument_escape",
    "to_json_argument_escape",
    "from_json_argument_escape",
]
