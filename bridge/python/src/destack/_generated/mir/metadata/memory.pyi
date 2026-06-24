# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AllocationSize: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AllocationSize: ...

def encode_allocation_size(writer: BinaryWriter, value: AllocationSize) -> None: ...
def decode_allocation_size(reader: BinaryReader) -> AllocationSize: ...
def to_json_allocation_size(value: AllocationSize) -> Json: ...
def from_json_allocation_size(value: Json) -> AllocationSize: ...

@dataclass(frozen=True, slots=True)
class CallArgumentEffect:
    """Memory behavior for one call argument."""

    # access mode for this argument
    access: ArgumentAccess
    # escape behavior for this argument
    escape: ArgumentEscape

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallArgumentEffect: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallArgumentEffect: ...

def encode_call_argument_effect(
    writer: BinaryWriter, value: CallArgumentEffect
) -> None: ...
def decode_call_argument_effect(reader: BinaryReader) -> CallArgumentEffect: ...
def to_json_call_argument_effect(value: CallArgumentEffect) -> Json: ...
def from_json_call_argument_effect(value: Json) -> CallArgumentEffect: ...

"""Access mode for a pointer argument."""
ArgumentAccess: typing.TypeAlias = (
    typing.Literal["none"]
    | typing.Literal["read"]
    | typing.Literal["write"]
    | typing.Literal["readWrite"]
)

def encode_argument_access(writer: BinaryWriter, value: ArgumentAccess) -> None: ...
def decode_argument_access(reader: BinaryReader) -> ArgumentAccess: ...
def to_json_argument_access(value: ArgumentAccess) -> Json: ...
def from_json_argument_access(value: Json) -> ArgumentAccess: ...

"""Escape behavior for a call argument."""
ArgumentEscape: typing.TypeAlias = (
    typing.Literal["none"] | typing.Literal["return"] | typing.Literal["escape"]
)

def encode_argument_escape(writer: BinaryWriter, value: ArgumentEscape) -> None: ...
def decode_argument_escape(reader: BinaryReader) -> ArgumentEscape: ...
def to_json_argument_escape(value: ArgumentEscape) -> Json: ...
def from_json_argument_escape(value: Json) -> ArgumentEscape: ...

@dataclass(frozen=True, slots=True)
class MemoryMetadata(MemoryMetadataImpl):
    """Table of memory metadata entries."""

    # memory access metadata keyed by instruction id
    memory_accesses_by_instruction_id: Mapping[
        destack._generated.mir.tree.node.LocalNodeId, Sequence[MemoryAccessMetadata]
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryMetadata: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemoryMetadata: ...

def encode_memory_metadata(writer: BinaryWriter, value: MemoryMetadata) -> None: ...
def decode_memory_metadata(reader: BinaryReader) -> MemoryMetadata: ...
def to_json_memory_metadata(value: MemoryMetadata) -> Json: ...
def from_json_memory_metadata(value: Json) -> MemoryMetadata: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryAccessMetadata: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemoryAccessMetadata: ...

def encode_memory_access_metadata(
    writer: BinaryWriter, value: MemoryAccessMetadata
) -> None: ...
def decode_memory_access_metadata(reader: BinaryReader) -> MemoryAccessMetadata: ...
def to_json_memory_access_metadata(value: MemoryAccessMetadata) -> Json: ...
def from_json_memory_access_metadata(value: Json) -> MemoryAccessMetadata: ...

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

def encode_memory_access_kind(
    writer: BinaryWriter, value: MemoryAccessKind
) -> None: ...
def decode_memory_access_kind(reader: BinaryReader) -> MemoryAccessKind: ...
def to_json_memory_access_kind(value: MemoryAccessKind) -> Json: ...
def from_json_memory_access_kind(value: Json) -> MemoryAccessKind: ...

@dataclass(frozen=True, slots=True)
class MemoryAccessTargetPointer:
    """Access through a pointer value."""

    pointer: destack._generated.mir.tree.value.Value
    kind: typing.Literal["pointer"] = "pointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryAccessTargetLocal:
    """Access through a local slot."""

    local: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryAccessTargetGlobal:
    """Access through a global."""

    global_: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["global"] = "global"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryAccessTargetUnknown:
    """Access with unknown target."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Target of a memory access."""
MemoryAccessTarget: typing.TypeAlias = (
    MemoryAccessTargetPointer
    | MemoryAccessTargetLocal
    | MemoryAccessTargetGlobal
    | MemoryAccessTargetUnknown
)

def encode_memory_access_target(
    writer: BinaryWriter, value: MemoryAccessTarget
) -> None: ...
def decode_memory_access_target(reader: BinaryReader) -> MemoryAccessTarget: ...
def to_json_memory_access_target(value: MemoryAccessTarget) -> Json: ...
def from_json_memory_access_target(value: Json) -> MemoryAccessTarget: ...

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
