# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class AtomicAccess:
    """Atomic ordering and scope for one memory operation."""

    # the memory ordering
    ordering: MemoryOrdering
    # the execution scope
    scope: ExecutionScope

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_atomic_access(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AtomicAccess:
        """Decode one AtomicAccess."""
        return decode_atomic_access(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_atomic_access(self)

    @classmethod
    def from_json(cls, value: Json) -> AtomicAccess:
        """Return one AtomicAccess from one JSON value."""
        return from_json_atomic_access(value)


def encode_atomic_access(writer: BinaryWriter, value: AtomicAccess) -> None:
    """Encode one AtomicAccess."""
    encode_memory_ordering(writer, value.ordering)
    encode_execution_scope(writer, value.scope)


def decode_atomic_access(reader: BinaryReader) -> AtomicAccess:
    """Decode one AtomicAccess."""
    ordering = decode_memory_ordering(reader)
    scope = decode_execution_scope(reader)

    return AtomicAccess(
        ordering=ordering,
        scope=scope,
    )


def to_json_atomic_access(value: AtomicAccess) -> Json:
    """Return one JSON value for one AtomicAccess."""
    return {
        "ordering": to_json_memory_ordering(value.ordering),
        "scope": to_json_execution_scope(value.scope),
    }


def from_json_atomic_access(value: Json) -> AtomicAccess:
    """Return one AtomicAccess from one JSON value."""
    object_ = json_object(value)

    return AtomicAccess(
        ordering=from_json_memory_ordering(json_field(object_, "ordering")),
        scope=from_json_execution_scope(json_field(object_, "scope")),
    )


"""Memory ordering for atomic operations."""
MemoryOrdering: typing.TypeAlias = (
    typing.Literal["relaxed"]
    | typing.Literal["acquire"]
    | typing.Literal["release"]
    | typing.Literal["acquireRelease"]
    | typing.Literal["sequentiallyConsistent"]
)


def encode_memory_ordering(writer: BinaryWriter, value: MemoryOrdering) -> None:
    """Encode one MemoryOrdering."""
    if value == "relaxed":
        writer.write_unsigned(0)
    elif value == "acquire":
        writer.write_unsigned(1)
    elif value == "release":
        writer.write_unsigned(2)
    elif value == "acquireRelease":
        writer.write_unsigned(3)
    elif value == "sequentiallyConsistent":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_memory_ordering(reader: BinaryReader) -> MemoryOrdering:
    """Decode one MemoryOrdering."""
    variant = reader.read_number()

    if variant == 0:
        return "relaxed"
    elif variant == 1:
        return "acquire"
    elif variant == 2:
        return "release"
    elif variant == 3:
        return "acquireRelease"
    elif variant == 4:
        return "sequentiallyConsistent"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_memory_ordering(value: MemoryOrdering) -> Json:
    """Return one JSON value for one MemoryOrdering."""
    return value


def from_json_memory_ordering(value: Json) -> MemoryOrdering:
    """Return one MemoryOrdering from one JSON value."""
    variant = json_string(value)

    if variant == "relaxed":
        return "relaxed"
    elif variant == "acquire":
        return "acquire"
    elif variant == "release":
        return "release"
    elif variant == "acquireRelease":
        return "acquireRelease"
    elif variant == "sequentiallyConsistent":
        return "sequentiallyConsistent"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Execution scope for atomic operations and fences."""
ExecutionScope: typing.TypeAlias = (
    typing.Literal["invocation"]
    | typing.Literal["subgroup"]
    | typing.Literal["workgroup"]
    | typing.Literal["device"]
    | typing.Literal["system"]
)


def encode_execution_scope(writer: BinaryWriter, value: ExecutionScope) -> None:
    """Encode one ExecutionScope."""
    if value == "invocation":
        writer.write_unsigned(0)
    elif value == "subgroup":
        writer.write_unsigned(1)
    elif value == "workgroup":
        writer.write_unsigned(2)
    elif value == "device":
        writer.write_unsigned(3)
    elif value == "system":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_execution_scope(reader: BinaryReader) -> ExecutionScope:
    """Decode one ExecutionScope."""
    variant = reader.read_number()

    if variant == 0:
        return "invocation"
    elif variant == 1:
        return "subgroup"
    elif variant == 2:
        return "workgroup"
    elif variant == 3:
        return "device"
    elif variant == 4:
        return "system"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_execution_scope(value: ExecutionScope) -> Json:
    """Return one JSON value for one ExecutionScope."""
    return value


def from_json_execution_scope(value: Json) -> ExecutionScope:
    """Return one ExecutionScope from one JSON value."""
    variant = json_string(value)

    if variant == "invocation":
        return "invocation"
    elif variant == "subgroup":
        return "subgroup"
    elif variant == "workgroup":
        return "workgroup"
    elif variant == "device":
        return "device"
    elif variant == "system":
        return "system"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class CompareExchangeAccess:
    """Access for one compare exchange operation."""

    # access used when the comparison succeeds
    success: AtomicAccess
    # ordering used by the failed comparison load
    failure_ordering: MemoryOrdering

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_compare_exchange_access(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CompareExchangeAccess:
        """Decode one CompareExchangeAccess."""
        return decode_compare_exchange_access(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_compare_exchange_access(self)

    @classmethod
    def from_json(cls, value: Json) -> CompareExchangeAccess:
        """Return one CompareExchangeAccess from one JSON value."""
        return from_json_compare_exchange_access(value)


def encode_compare_exchange_access(
    writer: BinaryWriter, value: CompareExchangeAccess
) -> None:
    """Encode one CompareExchangeAccess."""
    encode_atomic_access(writer, value.success)
    encode_memory_ordering(writer, value.failure_ordering)


def decode_compare_exchange_access(reader: BinaryReader) -> CompareExchangeAccess:
    """Decode one CompareExchangeAccess."""
    success = decode_atomic_access(reader)
    failure_ordering = decode_memory_ordering(reader)

    return CompareExchangeAccess(
        success=success,
        failure_ordering=failure_ordering,
    )


def to_json_compare_exchange_access(value: CompareExchangeAccess) -> Json:
    """Return one JSON value for one CompareExchangeAccess."""
    return {
        "success": to_json_atomic_access(value.success),
        "failureOrdering": to_json_memory_ordering(value.failure_ordering),
    }


def from_json_compare_exchange_access(value: Json) -> CompareExchangeAccess:
    """Return one CompareExchangeAccess from one JSON value."""
    object_ = json_object(value)

    return CompareExchangeAccess(
        success=from_json_atomic_access(json_field(object_, "success")),
        failure_ordering=from_json_memory_ordering(
            json_field(object_, "failureOrdering")
        ),
    )


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


def encode_atomic_rmw_operator(writer: BinaryWriter, value: AtomicRmwOperator) -> None:
    """Encode one AtomicRmwOperator."""
    if value == "exchange":
        writer.write_unsigned(0)
    elif value == "add":
        writer.write_unsigned(1)
    elif value == "sub":
        writer.write_unsigned(2)
    elif value == "and":
        writer.write_unsigned(3)
    elif value == "or":
        writer.write_unsigned(4)
    elif value == "xor":
        writer.write_unsigned(5)
    elif value == "min":
        writer.write_unsigned(6)
    elif value == "max":
        writer.write_unsigned(7)
    elif value == "umin":
        writer.write_unsigned(8)
    elif value == "umax":
        writer.write_unsigned(9)
    elif value == "fadd":
        writer.write_unsigned(10)
    elif value == "fmin":
        writer.write_unsigned(11)
    elif value == "fmax":
        writer.write_unsigned(12)
    else:
        raise SerdeError("unknown enum variant")


def decode_atomic_rmw_operator(reader: BinaryReader) -> AtomicRmwOperator:
    """Decode one AtomicRmwOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "exchange"
    elif variant == 1:
        return "add"
    elif variant == 2:
        return "sub"
    elif variant == 3:
        return "and"
    elif variant == 4:
        return "or"
    elif variant == 5:
        return "xor"
    elif variant == 6:
        return "min"
    elif variant == 7:
        return "max"
    elif variant == 8:
        return "umin"
    elif variant == 9:
        return "umax"
    elif variant == 10:
        return "fadd"
    elif variant == 11:
        return "fmin"
    elif variant == 12:
        return "fmax"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_atomic_rmw_operator(value: AtomicRmwOperator) -> Json:
    """Return one JSON value for one AtomicRmwOperator."""
    return value


def from_json_atomic_rmw_operator(value: Json) -> AtomicRmwOperator:
    """Return one AtomicRmwOperator from one JSON value."""
    variant = json_string(value)

    if variant == "exchange":
        return "exchange"
    elif variant == "add":
        return "add"
    elif variant == "sub":
        return "sub"
    elif variant == "and":
        return "and"
    elif variant == "or":
        return "or"
    elif variant == "xor":
        return "xor"
    elif variant == "min":
        return "min"
    elif variant == "max":
        return "max"
    elif variant == "umin":
        return "umin"
    elif variant == "umax":
        return "umax"
    elif variant == "fadd":
        return "fadd"
    elif variant == "fmin":
        return "fmin"
    elif variant == "fmax":
        return "fmax"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class FenceAccess:
    """Fence ordering, scope, and storage."""

    # the memory ordering
    ordering: MemoryOrdering
    # the execution scope
    scope: ExecutionScope
    # the ordered storage regions
    storage: StorageSet

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_fence_access(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FenceAccess:
        """Decode one FenceAccess."""
        return decode_fence_access(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_fence_access(self)

    @classmethod
    def from_json(cls, value: Json) -> FenceAccess:
        """Return one FenceAccess from one JSON value."""
        return from_json_fence_access(value)


def encode_fence_access(writer: BinaryWriter, value: FenceAccess) -> None:
    """Encode one FenceAccess."""
    encode_memory_ordering(writer, value.ordering)
    encode_execution_scope(writer, value.scope)
    encode_storage_set(writer, value.storage)


def decode_fence_access(reader: BinaryReader) -> FenceAccess:
    """Decode one FenceAccess."""
    ordering = decode_memory_ordering(reader)
    scope = decode_execution_scope(reader)
    storage = decode_storage_set(reader)

    return FenceAccess(
        ordering=ordering,
        scope=scope,
        storage=storage,
    )


def to_json_fence_access(value: FenceAccess) -> Json:
    """Return one JSON value for one FenceAccess."""
    return {
        "ordering": to_json_memory_ordering(value.ordering),
        "scope": to_json_execution_scope(value.scope),
        "storage": to_json_storage_set(value.storage),
    }


def from_json_fence_access(value: Json) -> FenceAccess:
    """Return one FenceAccess from one JSON value."""
    object_ = json_object(value)

    return FenceAccess(
        ordering=from_json_memory_ordering(json_field(object_, "ordering")),
        scope=from_json_execution_scope(json_field(object_, "scope")),
        storage=from_json_storage_set(json_field(object_, "storage")),
    )


"""Set of backing storage regions that an operation may access."""
StorageSet: typing.TypeAlias = int


def encode_storage_set(writer: BinaryWriter, value: StorageSet) -> None:
    """Encode one StorageSet."""
    writer.write_unsigned(value)


def decode_storage_set(reader: BinaryReader) -> StorageSet:
    """Decode one StorageSet."""
    return reader.read_number()


def to_json_storage_set(value: StorageSet) -> Json:
    """Return one JSON value for one StorageSet."""
    return value


def from_json_storage_set(value: Json) -> StorageSet:
    """Return one StorageSet from one JSON value."""
    return json_int(value)


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
