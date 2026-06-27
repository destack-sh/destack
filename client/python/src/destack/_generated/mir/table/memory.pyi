# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemoryTable: ...

def encode_memory_table(writer: BinaryWriter, value: MemoryTable) -> None: ...
def decode_memory_table(reader: BinaryReader) -> MemoryTable: ...
def to_json_memory_table(value: MemoryTable) -> Json: ...
def from_json_memory_table(value: Json) -> MemoryTable: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryAccess: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemoryAccess: ...

def encode_memory_access(writer: BinaryWriter, value: MemoryAccess) -> None: ...
def decode_memory_access(reader: BinaryReader) -> MemoryAccess: ...
def to_json_memory_access(value: MemoryAccess) -> Json: ...
def from_json_memory_access(value: Json) -> MemoryAccess: ...

"""The operation performed by a memory access."""
MemoryOperation: typing.TypeAlias = (
    typing.Literal["read"] | typing.Literal["write"] | typing.Literal["readWrite"]
)

def encode_memory_operation(writer: BinaryWriter, value: MemoryOperation) -> None: ...
def decode_memory_operation(reader: BinaryReader) -> MemoryOperation: ...
def to_json_memory_operation(value: MemoryOperation) -> Json: ...
def from_json_memory_operation(value: Json) -> MemoryOperation: ...

@dataclass(frozen=True, slots=True)
class MemoryTargetReference:
    """Access through a reference value."""

    reference: destack._generated.mir.tree.value.Value
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryTargetLocal:
    """Access through a local slot."""

    local: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryTargetGlobal:
    """Access through a global."""

    global_: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["global"] = "global"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Target of one memory access."""
MemoryTarget: typing.TypeAlias = (
    MemoryTargetReference | MemoryTargetLocal | MemoryTargetGlobal
)

def encode_memory_target(writer: BinaryWriter, value: MemoryTarget) -> None: ...
def decode_memory_target(reader: BinaryReader) -> MemoryTarget: ...
def to_json_memory_target(value: MemoryTarget) -> Json: ...
def from_json_memory_target(value: Json) -> MemoryTarget: ...

@dataclass(frozen=True, slots=True)
class MemoryAccessOrderPlain:
    """Ordinary memory access."""

    kind: typing.Literal["plain"] = "plain"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryAccessOrderVolatile:
    """Externally observable memory access."""

    kind: typing.Literal["volatile"] = "volatile"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryAccessOrderAtomic:
    """Atomic memory access."""

    atomic: destack._generated.mir.tree.memory.AtomicAccess
    kind: typing.Literal["atomic"] = "atomic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Ordering constraints for one memory access."""
MemoryAccessOrder: typing.TypeAlias = (
    MemoryAccessOrderPlain | MemoryAccessOrderVolatile | MemoryAccessOrderAtomic
)

def encode_memory_access_order(
    writer: BinaryWriter, value: MemoryAccessOrder
) -> None: ...
def decode_memory_access_order(reader: BinaryReader) -> MemoryAccessOrder: ...
def to_json_memory_access_order(value: MemoryAccessOrder) -> Json: ...
def from_json_memory_access_order(value: Json) -> MemoryAccessOrder: ...

@dataclass(frozen=True, slots=True)
class CallArgumentEffect:
    """Summary behavior for one argument passed to a bodyless call."""

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

"""Access mode for a bodyless call pointer argument."""
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

"""Escape behavior for a bodyless call argument."""
ArgumentEscape: typing.TypeAlias = (
    typing.Literal["none"] | typing.Literal["return"] | typing.Literal["escape"]
)

def encode_argument_escape(writer: BinaryWriter, value: ArgumentEscape) -> None: ...
def decode_argument_escape(reader: BinaryReader) -> ArgumentEscape: ...
def to_json_argument_escape(value: ArgumentEscape) -> Json: ...
def from_json_argument_escape(value: Json) -> ArgumentEscape: ...

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
