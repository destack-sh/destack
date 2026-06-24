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
    json_int,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

from destack._impl.mir.metadata.memory import (
    MemoryMetadataImpl,
)

import destack._generated.mir.tree.memory
import destack._generated.mir.tree.node
import destack._generated.mir.tree.type
import destack._generated.mir.tree.value


@dataclass(frozen=True, slots=True)
class AllocationSize:
    """Allocation size information for functions returning newly allocated memory."""

    # the parameter index containing the element size in bytes
    stride_index: int
    # the parameter index containing the element count, if any
    element_count_index: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_allocation_size(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AllocationSize:
        """Decode one AllocationSize."""
        return decode_allocation_size(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_allocation_size(self)

    @classmethod
    def from_json(cls, value: Json) -> AllocationSize:
        """Return one AllocationSize from one JSON value."""
        return from_json_allocation_size(value)


def encode_allocation_size(writer: BinaryWriter, value: AllocationSize) -> None:
    """Encode one AllocationSize."""
    writer.write_unsigned(value.stride_index)
    if value.element_count_index is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.element_count_index)


def decode_allocation_size(reader: BinaryReader) -> AllocationSize:
    """Decode one AllocationSize."""
    stride_index = reader.read_number()
    element_count_index = reader.read_option(lambda: reader.read_number())

    return AllocationSize(
        stride_index=stride_index,
        element_count_index=element_count_index,
    )


def to_json_allocation_size(value: AllocationSize) -> Json:
    """Return one JSON value for one AllocationSize."""
    return {
        "strideIndex": value.stride_index,
        **(
            {}
            if value.element_count_index is None
            else {"elementCountIndex": value.element_count_index}
        ),
    }


def from_json_allocation_size(value: Json) -> AllocationSize:
    """Return one AllocationSize from one JSON value."""
    object_ = json_object(value)

    return AllocationSize(
        stride_index=json_int(json_field(object_, "strideIndex")),
        element_count_index=json_optional(
            object_, "elementCountIndex", lambda value: json_int(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class CallArgumentEffect:
    """Memory behavior for one call argument."""

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


"""Access mode for a pointer argument."""
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


"""Escape behavior for a call argument."""
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


@dataclass(frozen=True, slots=True)
class MemoryMetadata(MemoryMetadataImpl):
    """Table of memory metadata entries."""

    # memory access metadata keyed by instruction id
    memory_accesses_by_instruction_id: Mapping[
        destack._generated.mir.tree.node.LocalNodeId, Sequence[MemoryAccessMetadata]
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_metadata(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryMetadata:
        """Decode one MemoryMetadata."""
        return decode_memory_metadata(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_metadata(self)

    @classmethod
    def from_json(cls, value: Json) -> MemoryMetadata:
        """Return one MemoryMetadata from one JSON value."""
        return from_json_memory_metadata(value)


def encode_memory_metadata(writer: BinaryWriter, value: MemoryMetadata) -> None:
    """Encode one MemoryMetadata."""
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
            encode_memory_access_metadata(
                writer, item_entry_value_memory_accesses_by_instruction_id_0_1_1
            )


def decode_memory_metadata(reader: BinaryReader) -> MemoryMetadata:
    """Decode one MemoryMetadata."""
    memory_accesses_by_instruction_id = {
        destack._generated.mir.tree.node.decode_local_node_id(reader): [
            decode_memory_access_metadata(reader) for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }

    return MemoryMetadata(
        memory_accesses_by_instruction_id=memory_accesses_by_instruction_id,
    )


def to_json_memory_metadata(value: MemoryMetadata) -> Json:
    """Return one JSON value for one MemoryMetadata."""
    return {
        "memoryAccessesByInstructionId": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                [to_json_memory_access_metadata(item_1) for item_1 in item_0],
            ]
            for key_0, item_0 in value.memory_accesses_by_instruction_id.items()
        ],
    }


def from_json_memory_metadata(value: Json) -> MemoryMetadata:
    """Return one MemoryMetadata from one JSON value."""
    object_ = json_object(value)

    return MemoryMetadata(
        memory_accesses_by_instruction_id={
            destack._generated.mir.tree.node.from_json_local_node_id(key_0): [
                from_json_memory_access_metadata(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(
                json_field(object_, "memoryAccessesByInstructionId")
            )
        },
    )


@dataclass(frozen=True, slots=True)
class MemoryAccessMetadata:
    """Metadata describing a single memory access in an instruction."""

    # the kind of access performed
    kind: MemoryAccessKind
    # the memory target for the access
    target: MemoryAccessTarget
    # the number of bytes accessed when known
    size: int | None
    # alignment in bytes, when known
    alignment: int | None
    # whether the access is volatile
    is_volatile: bool
    # whether repeated loads observe the same value
    is_load_invariant: bool
    # memory ordering for atomic accesses
    ordering: destack._generated.mir.tree.memory.MemoryOrdering | None
    # synchronization scope for atomic accesses and fences
    scope: destack._generated.mir.tree.memory.SyncScope | None
    # memory visibility scope for fences
    memory_scope: destack._generated.mir.tree.memory.MemoryScope | None
    # memory flags for fences
    flags: destack._generated.mir.tree.memory.MemoryFlags | None
    # space override for the access
    space: destack._generated.mir.tree.type.Space | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_access_metadata(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryAccessMetadata:
        """Decode one MemoryAccessMetadata."""
        return decode_memory_access_metadata(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_access_metadata(self)

    @classmethod
    def from_json(cls, value: Json) -> MemoryAccessMetadata:
        """Return one MemoryAccessMetadata from one JSON value."""
        return from_json_memory_access_metadata(value)


def encode_memory_access_metadata(
    writer: BinaryWriter, value: MemoryAccessMetadata
) -> None:
    """Encode one MemoryAccessMetadata."""
    encode_memory_access_kind(writer, value.kind)
    encode_memory_access_target(writer, value.target)
    if value.size is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.size)
    if value.alignment is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.alignment)
    writer.write_bool(value.is_volatile)
    writer.write_bool(value.is_load_invariant)
    if value.ordering is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.memory.encode_memory_ordering(
            writer, value.ordering
        )
    if value.scope is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.memory.encode_sync_scope(writer, value.scope)
    if value.memory_scope is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.memory.encode_memory_scope(
            writer, value.memory_scope
        )
    if value.flags is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.memory.encode_memory_flags(writer, value.flags)
    if value.space is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.type.encode_space(writer, value.space)


def decode_memory_access_metadata(reader: BinaryReader) -> MemoryAccessMetadata:
    """Decode one MemoryAccessMetadata."""
    kind = decode_memory_access_kind(reader)
    target = decode_memory_access_target(reader)
    size = reader.read_option(lambda: reader.read_number())
    alignment = reader.read_option(lambda: reader.read_number())
    is_volatile = reader.read_bool()
    is_load_invariant = reader.read_bool()
    ordering = reader.read_option(
        lambda: destack._generated.mir.tree.memory.decode_memory_ordering(reader)
    )
    scope = reader.read_option(
        lambda: destack._generated.mir.tree.memory.decode_sync_scope(reader)
    )
    memory_scope = reader.read_option(
        lambda: destack._generated.mir.tree.memory.decode_memory_scope(reader)
    )
    flags = reader.read_option(
        lambda: destack._generated.mir.tree.memory.decode_memory_flags(reader)
    )
    space = reader.read_option(
        lambda: destack._generated.mir.tree.type.decode_space(reader)
    )

    return MemoryAccessMetadata(
        kind=kind,
        target=target,
        size=size,
        alignment=alignment,
        is_volatile=is_volatile,
        is_load_invariant=is_load_invariant,
        ordering=ordering,
        scope=scope,
        memory_scope=memory_scope,
        flags=flags,
        space=space,
    )


def to_json_memory_access_metadata(value: MemoryAccessMetadata) -> Json:
    """Return one JSON value for one MemoryAccessMetadata."""
    return {
        "kind": to_json_memory_access_kind(value.kind),
        "target": to_json_memory_access_target(value.target),
        **({} if value.size is None else {"size": value.size}),
        **({} if value.alignment is None else {"alignment": value.alignment}),
        "isVolatile": value.is_volatile,
        "isLoadInvariant": value.is_load_invariant,
        **(
            {}
            if value.ordering is None
            else {
                "ordering": destack._generated.mir.tree.memory.to_json_memory_ordering(
                    value.ordering
                )
            }
        ),
        **(
            {}
            if value.scope is None
            else {
                "scope": destack._generated.mir.tree.memory.to_json_sync_scope(
                    value.scope
                )
            }
        ),
        **(
            {}
            if value.memory_scope is None
            else {
                "memoryScope": destack._generated.mir.tree.memory.to_json_memory_scope(
                    value.memory_scope
                )
            }
        ),
        **(
            {}
            if value.flags is None
            else {
                "flags": destack._generated.mir.tree.memory.to_json_memory_flags(
                    value.flags
                )
            }
        ),
        **(
            {}
            if value.space is None
            else {"space": destack._generated.mir.tree.type.to_json_space(value.space)}
        ),
    }


def from_json_memory_access_metadata(value: Json) -> MemoryAccessMetadata:
    """Return one MemoryAccessMetadata from one JSON value."""
    object_ = json_object(value)

    return MemoryAccessMetadata(
        kind=from_json_memory_access_kind(json_field(object_, "kind")),
        target=from_json_memory_access_target(json_field(object_, "target")),
        size=json_optional(object_, "size", lambda value: json_int(value)),
        alignment=json_optional(object_, "alignment", lambda value: json_int(value)),
        is_volatile=json_bool(json_field(object_, "isVolatile")),
        is_load_invariant=json_bool(json_field(object_, "isLoadInvariant")),
        ordering=json_optional(
            object_,
            "ordering",
            lambda value: destack._generated.mir.tree.memory.from_json_memory_ordering(
                value
            ),
        ),
        scope=json_optional(
            object_,
            "scope",
            lambda value: destack._generated.mir.tree.memory.from_json_sync_scope(
                value
            ),
        ),
        memory_scope=json_optional(
            object_,
            "memoryScope",
            lambda value: destack._generated.mir.tree.memory.from_json_memory_scope(
                value
            ),
        ),
        flags=json_optional(
            object_,
            "flags",
            lambda value: destack._generated.mir.tree.memory.from_json_memory_flags(
                value
            ),
        ),
        space=json_optional(
            object_,
            "space",
            lambda value: destack._generated.mir.tree.type.from_json_space(value),
        ),
    )


"""The kind of memory access represented by metadata."""
MemoryAccessKind: typing.TypeAlias = (
    typing.Literal["read"]
    | typing.Literal["write"]
    | typing.Literal["readWrite"]
    | typing.Literal["readModifyWrite"]
    | typing.Literal["fence"]
    | typing.Literal["prefetchRead"]
    | typing.Literal["prefetchWrite"]
)


def encode_memory_access_kind(writer: BinaryWriter, value: MemoryAccessKind) -> None:
    """Encode one MemoryAccessKind."""
    if value == "read":
        writer.write_unsigned(0)
    elif value == "write":
        writer.write_unsigned(1)
    elif value == "readWrite":
        writer.write_unsigned(2)
    elif value == "readModifyWrite":
        writer.write_unsigned(3)
    elif value == "fence":
        writer.write_unsigned(4)
    elif value == "prefetchRead":
        writer.write_unsigned(5)
    elif value == "prefetchWrite":
        writer.write_unsigned(6)
    else:
        raise SerdeError("unknown enum variant")


def decode_memory_access_kind(reader: BinaryReader) -> MemoryAccessKind:
    """Decode one MemoryAccessKind."""
    variant = reader.read_number()

    if variant == 0:
        return "read"
    elif variant == 1:
        return "write"
    elif variant == 2:
        return "readWrite"
    elif variant == 3:
        return "readModifyWrite"
    elif variant == 4:
        return "fence"
    elif variant == 5:
        return "prefetchRead"
    elif variant == 6:
        return "prefetchWrite"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_memory_access_kind(value: MemoryAccessKind) -> Json:
    """Return one JSON value for one MemoryAccessKind."""
    return value


def from_json_memory_access_kind(value: Json) -> MemoryAccessKind:
    """Return one MemoryAccessKind from one JSON value."""
    variant = json_string(value)

    if variant == "read":
        return "read"
    elif variant == "write":
        return "write"
    elif variant == "readWrite":
        return "readWrite"
    elif variant == "readModifyWrite":
        return "readModifyWrite"
    elif variant == "fence":
        return "fence"
    elif variant == "prefetchRead":
        return "prefetchRead"
    elif variant == "prefetchWrite":
        return "prefetchWrite"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class MemoryAccessTargetPointer:
    """Access through a pointer value."""

    pointer: destack._generated.mir.tree.value.Value
    kind: typing.Literal["pointer"] = "pointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_access_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_access_target(self)


@dataclass(frozen=True, slots=True)
class MemoryAccessTargetLocal:
    """Access through a local slot."""

    local: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_access_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_access_target(self)


@dataclass(frozen=True, slots=True)
class MemoryAccessTargetGlobal:
    """Access through a global."""

    global_: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["global"] = "global"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_access_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_access_target(self)


@dataclass(frozen=True, slots=True)
class MemoryAccessTargetUnknown:
    """Access with unknown target."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_access_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_access_target(self)


"""Target of a memory access."""
MemoryAccessTarget: typing.TypeAlias = (
    MemoryAccessTargetPointer
    | MemoryAccessTargetLocal
    | MemoryAccessTargetGlobal
    | MemoryAccessTargetUnknown
)


def encode_memory_access_target(
    writer: BinaryWriter, value: MemoryAccessTarget
) -> None:
    """Encode one MemoryAccessTarget."""
    if value.kind == "pointer":
        writer.write_unsigned(0)
        destack._generated.mir.tree.value.encode_value(writer, value.pointer)
    elif value.kind == "local":
        writer.write_unsigned(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.local)
    elif value.kind == "global":
        writer.write_unsigned(2)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.global_)
    elif value.kind == "unknown":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_memory_access_target(reader: BinaryReader) -> MemoryAccessTarget:
    """Decode one MemoryAccessTarget."""
    variant = reader.read_number()

    if variant == 0:
        pointer = destack._generated.mir.tree.value.decode_value(reader)

        return MemoryAccessTargetPointer(pointer=pointer)
    elif variant == 1:
        local = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return MemoryAccessTargetLocal(local=local)
    elif variant == 2:
        global_ = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return MemoryAccessTargetGlobal(global_=global_)
    elif variant == 3:
        return MemoryAccessTargetUnknown()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_memory_access_target(value: MemoryAccessTarget) -> Json:
    """Return one JSON value for one MemoryAccessTarget."""
    if value.kind == "pointer":
        return {
            "kind": "pointer",
            "pointer": destack._generated.mir.tree.value.to_json_value(value.pointer),
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
    elif value.kind == "unknown":
        return {
            "kind": "unknown",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_memory_access_target(value: Json) -> MemoryAccessTarget:
    """Return one MemoryAccessTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "pointer":
        return MemoryAccessTargetPointer(
            pointer=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "pointer")
            )
        )
    elif kind == "local":
        return MemoryAccessTargetLocal(
            local=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "local")
            )
        )
    elif kind == "global":
        return MemoryAccessTargetGlobal(
            global_=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "global")
            )
        )
    elif kind == "unknown":
        return MemoryAccessTargetUnknown()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "AllocationSize",
    "encode_allocation_size",
    "decode_allocation_size",
    "to_json_allocation_size",
    "from_json_allocation_size",
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
    "MemoryMetadata",
    "encode_memory_metadata",
    "decode_memory_metadata",
    "to_json_memory_metadata",
    "from_json_memory_metadata",
    "MemoryAccessMetadata",
    "encode_memory_access_metadata",
    "decode_memory_access_metadata",
    "to_json_memory_access_metadata",
    "from_json_memory_access_metadata",
    "MemoryAccessKind",
    "encode_memory_access_kind",
    "decode_memory_access_kind",
    "to_json_memory_access_kind",
    "from_json_memory_access_kind",
    "MemoryAccessTarget",
    "encode_memory_access_target",
    "decode_memory_access_target",
    "to_json_memory_access_target",
    "from_json_memory_access_target",
    "MemoryAccessTargetPointer",
    "MemoryAccessTargetLocal",
    "MemoryAccessTargetGlobal",
    "MemoryAccessTargetUnknown",
]
