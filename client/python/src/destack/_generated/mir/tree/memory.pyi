# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class AtomicAccess:
    """Atomic ordering and scope for one memory operation."""

    # the memory ordering
    ordering: MemoryOrdering
    # the synchronization scope
    scope: SyncScope
    # whether the access must be preserved as a volatile operation
    is_volatile: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AtomicAccess: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AtomicAccess: ...

def encode_atomic_access(writer: BinaryWriter, value: AtomicAccess) -> None: ...
def decode_atomic_access(reader: BinaryReader) -> AtomicAccess: ...
def to_json_atomic_access(value: AtomicAccess) -> Json: ...
def from_json_atomic_access(value: Json) -> AtomicAccess: ...

"""Memory ordering for atomic operations."""
MemoryOrdering: typing.TypeAlias = (
    typing.Literal["relaxed"]
    | typing.Literal["acquire"]
    | typing.Literal["release"]
    | typing.Literal["acquireRelease"]
    | typing.Literal["sequentiallyConsistent"]
)

def encode_memory_ordering(writer: BinaryWriter, value: MemoryOrdering) -> None: ...
def decode_memory_ordering(reader: BinaryReader) -> MemoryOrdering: ...
def to_json_memory_ordering(value: MemoryOrdering) -> Json: ...
def from_json_memory_ordering(value: Json) -> MemoryOrdering: ...

"""Synchronization scope for atomic operations and fences."""
SyncScope: typing.TypeAlias = (
    typing.Literal["invocation"]
    | typing.Literal["subgroup"]
    | typing.Literal["workgroup"]
    | typing.Literal["device"]
    | typing.Literal["system"]
)

def encode_sync_scope(writer: BinaryWriter, value: SyncScope) -> None: ...
def decode_sync_scope(reader: BinaryReader) -> SyncScope: ...
def to_json_sync_scope(value: SyncScope) -> Json: ...
def from_json_sync_scope(value: Json) -> SyncScope: ...

@dataclass(frozen=True, slots=True)
class CompareExchangeAccess:
    """Access for one compare exchange operation."""

    # access used when the comparison succeeds
    success: AtomicAccess
    # ordering used by the failed comparison load
    failure_ordering: MemoryOrdering

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CompareExchangeAccess: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CompareExchangeAccess: ...

def encode_compare_exchange_access(
    writer: BinaryWriter, value: CompareExchangeAccess
) -> None: ...
def decode_compare_exchange_access(reader: BinaryReader) -> CompareExchangeAccess: ...
def to_json_compare_exchange_access(value: CompareExchangeAccess) -> Json: ...
def from_json_compare_exchange_access(value: Json) -> CompareExchangeAccess: ...

"""Read-modify-write operator for atomic memory operations."""
AtomicRmwOperator: typing.TypeAlias = (
    typing.Literal["exchange"]
    | typing.Literal["add"]
    | typing.Literal["sub"]
    | typing.Literal["and"]
    | typing.Literal["or"]
    | typing.Literal["xor"]
    | typing.Literal["min"]
    | typing.Literal["max"]
    | typing.Literal["umin"]
    | typing.Literal["umax"]
    | typing.Literal["fadd"]
    | typing.Literal["fmin"]
    | typing.Literal["fmax"]
)

def encode_atomic_rmw_operator(
    writer: BinaryWriter, value: AtomicRmwOperator
) -> None: ...
def decode_atomic_rmw_operator(reader: BinaryReader) -> AtomicRmwOperator: ...
def to_json_atomic_rmw_operator(value: AtomicRmwOperator) -> Json: ...
def from_json_atomic_rmw_operator(value: Json) -> AtomicRmwOperator: ...

@dataclass(frozen=True, slots=True)
class FenceAccess:
    """Fence ordering, scope, and memory visibility."""

    # the memory ordering
    ordering: MemoryOrdering
    # the synchronization scope
    scope: SyncScope
    # the memory visibility scope
    memory_scope: MemoryScope
    # the memory flags
    flags: MemoryFlags

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FenceAccess: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FenceAccess: ...

def encode_fence_access(writer: BinaryWriter, value: FenceAccess) -> None: ...
def decode_fence_access(reader: BinaryReader) -> FenceAccess: ...
def to_json_fence_access(value: FenceAccess) -> Json: ...
def from_json_fence_access(value: Json) -> FenceAccess: ...

"""Memory scope for fences."""
MemoryScope: typing.TypeAlias = (
    typing.Literal["invocation"]
    | typing.Literal["subgroup"]
    | typing.Literal["workgroup"]
    | typing.Literal["device"]
    | typing.Literal["system"]
)

def encode_memory_scope(writer: BinaryWriter, value: MemoryScope) -> None: ...
def decode_memory_scope(reader: BinaryReader) -> MemoryScope: ...
def to_json_memory_scope(value: MemoryScope) -> Json: ...
def from_json_memory_scope(value: Json) -> MemoryScope: ...

@dataclass(frozen=True, slots=True)
class MemoryFlags:
    """Memory flags for fences."""

    # the memory spaces affected by the fence
    spaces: SpaceSet
    # whether this makes writes available to other scopes
    makes_available: bool
    # whether this makes writes visible to other scopes
    makes_visible: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryFlags: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemoryFlags: ...

def encode_memory_flags(writer: BinaryWriter, value: MemoryFlags) -> None: ...
def decode_memory_flags(reader: BinaryReader) -> MemoryFlags: ...
def to_json_memory_flags(value: MemoryFlags) -> Json: ...
def from_json_memory_flags(value: Json) -> MemoryFlags: ...

"""Set of backing memory spaces that an operation may access."""
SpaceSet: typing.TypeAlias = int

def encode_space_set(writer: BinaryWriter, value: SpaceSet) -> None: ...
def decode_space_set(reader: BinaryReader) -> SpaceSet: ...
def to_json_space_set(value: SpaceSet) -> Json: ...
def from_json_space_set(value: Json) -> SpaceSet: ...

__all__ = [
    "AtomicAccess",
    "encode_atomic_access",
    "decode_atomic_access",
    "to_json_atomic_access",
    "from_json_atomic_access",
    "MemoryOrdering",
    "encode_memory_ordering",
    "decode_memory_ordering",
    "to_json_memory_ordering",
    "from_json_memory_ordering",
    "SyncScope",
    "encode_sync_scope",
    "decode_sync_scope",
    "to_json_sync_scope",
    "from_json_sync_scope",
    "CompareExchangeAccess",
    "encode_compare_exchange_access",
    "decode_compare_exchange_access",
    "to_json_compare_exchange_access",
    "from_json_compare_exchange_access",
    "AtomicRmwOperator",
    "encode_atomic_rmw_operator",
    "decode_atomic_rmw_operator",
    "to_json_atomic_rmw_operator",
    "from_json_atomic_rmw_operator",
    "FenceAccess",
    "encode_fence_access",
    "decode_fence_access",
    "to_json_fence_access",
    "from_json_fence_access",
    "MemoryScope",
    "encode_memory_scope",
    "decode_memory_scope",
    "to_json_memory_scope",
    "from_json_memory_scope",
    "MemoryFlags",
    "encode_memory_flags",
    "decode_memory_flags",
    "to_json_memory_flags",
    "from_json_memory_flags",
    "SpaceSet",
    "encode_space_set",
    "decode_space_set",
    "to_json_space_set",
    "from_json_space_set",
]
