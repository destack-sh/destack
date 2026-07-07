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
    # the execution scope
    scope: ExecutionScope

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

"""Execution scope for atomic operations and fences."""
ExecutionScope: typing.TypeAlias = (
    typing.Literal["invocation"]
    | typing.Literal["subgroup"]
    | typing.Literal["workgroup"]
    | typing.Literal["device"]
    | typing.Literal["system"]
)

def encode_execution_scope(writer: BinaryWriter, value: ExecutionScope) -> None: ...
def decode_execution_scope(reader: BinaryReader) -> ExecutionScope: ...
def to_json_execution_scope(value: ExecutionScope) -> Json: ...
def from_json_execution_scope(value: Json) -> ExecutionScope: ...

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
    """Fence ordering, scope, and storage."""

    # the memory ordering
    ordering: MemoryOrdering
    # the execution scope
    scope: ExecutionScope
    # the ordered storage regions
    storage: StorageSet

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

"""Set of backing storage regions that an operation may access."""
StorageSet: typing.TypeAlias = int

def encode_storage_set(writer: BinaryWriter, value: StorageSet) -> None: ...
def decode_storage_set(reader: BinaryReader) -> StorageSet: ...
def to_json_storage_set(value: StorageSet) -> Json: ...
def from_json_storage_set(value: Json) -> StorageSet: ...

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
    "ExecutionScope",
    "encode_execution_scope",
    "decode_execution_scope",
    "to_json_execution_scope",
    "from_json_execution_scope",
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
    "StorageSet",
    "encode_storage_set",
    "decode_storage_set",
    "to_json_storage_set",
    "from_json_storage_set",
]
