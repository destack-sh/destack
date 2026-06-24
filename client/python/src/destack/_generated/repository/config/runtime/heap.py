# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
    json_optional,
)


@dataclass(frozen=True, slots=True)
class HeapOptions:
    """Runtime heap configuration."""

    # soft memory limit inherited by local and shared heap collectors
    memory_limit_bytes: int | None
    # the proportional heap growth target percentage
    growth_percent: int
    # worker-local heap policy
    local: LocalHeapOptions
    # runtime-shared heap policy
    shared: SharedHeapOptions

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_heap_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HeapOptions:
        """Decode one HeapOptions."""
        return decode_heap_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_heap_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HeapOptions:
        """Return one HeapOptions from one JSON value."""
        return from_json_heap_options(value)


def encode_heap_options(writer: BinaryWriter, value: HeapOptions) -> None:
    """Encode one HeapOptions."""
    if value.memory_limit_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.memory_limit_bytes)
    writer.write_unsigned(value.growth_percent)
    encode_local_heap_options(writer, value.local)
    encode_shared_heap_options(writer, value.shared)


def decode_heap_options(reader: BinaryReader) -> HeapOptions:
    """Decode one HeapOptions."""
    memory_limit_bytes = reader.read_option(lambda: reader.read_number())
    growth_percent = reader.read_number()
    local = decode_local_heap_options(reader)
    shared = decode_shared_heap_options(reader)

    return HeapOptions(
        memory_limit_bytes=memory_limit_bytes,
        growth_percent=growth_percent,
        local=local,
        shared=shared,
    )


def to_json_heap_options(value: HeapOptions) -> Json:
    """Return one JSON value for one HeapOptions."""
    return {
        **(
            {}
            if value.memory_limit_bytes is None
            else {"memoryLimitBytes": value.memory_limit_bytes}
        ),
        "growthPercent": value.growth_percent,
        "local": to_json_local_heap_options(value.local),
        "shared": to_json_shared_heap_options(value.shared),
    }


def from_json_heap_options(value: Json) -> HeapOptions:
    """Return one HeapOptions from one JSON value."""
    object_ = json_object(value)

    return HeapOptions(
        memory_limit_bytes=json_optional(
            object_, "memoryLimitBytes", lambda value: json_int(value)
        ),
        growth_percent=json_int(json_field(object_, "growthPercent")),
        local=from_json_local_heap_options(json_field(object_, "local")),
        shared=from_json_shared_heap_options(json_field(object_, "shared")),
    )


@dataclass(frozen=True, slots=True)
class LocalHeapOptions:
    """Runtime local-heap policy."""

    # the byte width for worker-local young space
    young_size_bytes: int
    # minimum heap bytes before normal growth pacing applies
    min_bytes: int | None
    # hard limit for total retained local heap bytes
    max_bytes: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_local_heap_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalHeapOptions:
        """Decode one LocalHeapOptions."""
        return decode_local_heap_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_local_heap_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LocalHeapOptions:
        """Return one LocalHeapOptions from one JSON value."""
        return from_json_local_heap_options(value)


def encode_local_heap_options(writer: BinaryWriter, value: LocalHeapOptions) -> None:
    """Encode one LocalHeapOptions."""
    writer.write_unsigned(value.young_size_bytes)
    if value.min_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.min_bytes)
    if value.max_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.max_bytes)


def decode_local_heap_options(reader: BinaryReader) -> LocalHeapOptions:
    """Decode one LocalHeapOptions."""
    young_size_bytes = reader.read_number()
    min_bytes = reader.read_option(lambda: reader.read_number())
    max_bytes = reader.read_option(lambda: reader.read_number())

    return LocalHeapOptions(
        young_size_bytes=young_size_bytes,
        min_bytes=min_bytes,
        max_bytes=max_bytes,
    )


def to_json_local_heap_options(value: LocalHeapOptions) -> Json:
    """Return one JSON value for one LocalHeapOptions."""
    return {
        "youngSizeBytes": value.young_size_bytes,
        **({} if value.min_bytes is None else {"minBytes": value.min_bytes}),
        **({} if value.max_bytes is None else {"maxBytes": value.max_bytes}),
    }


def from_json_local_heap_options(value: Json) -> LocalHeapOptions:
    """Return one LocalHeapOptions from one JSON value."""
    object_ = json_object(value)

    return LocalHeapOptions(
        young_size_bytes=json_int(json_field(object_, "youngSizeBytes")),
        min_bytes=json_optional(object_, "minBytes", lambda value: json_int(value)),
        max_bytes=json_optional(object_, "maxBytes", lambda value: json_int(value)),
    )


@dataclass(frozen=True, slots=True)
class SharedHeapOptions:
    """Runtime shared-heap policy."""

    # minimum heap bytes before normal growth pacing applies
    min_bytes: int | None
    # hard limit for total retained shared heap bytes
    max_bytes: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_shared_heap_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SharedHeapOptions:
        """Decode one SharedHeapOptions."""
        return decode_shared_heap_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_shared_heap_options(self)

    @classmethod
    def from_json(cls, value: Json) -> SharedHeapOptions:
        """Return one SharedHeapOptions from one JSON value."""
        return from_json_shared_heap_options(value)


def encode_shared_heap_options(writer: BinaryWriter, value: SharedHeapOptions) -> None:
    """Encode one SharedHeapOptions."""
    if value.min_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.min_bytes)
    if value.max_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.max_bytes)


def decode_shared_heap_options(reader: BinaryReader) -> SharedHeapOptions:
    """Decode one SharedHeapOptions."""
    min_bytes = reader.read_option(lambda: reader.read_number())
    max_bytes = reader.read_option(lambda: reader.read_number())

    return SharedHeapOptions(
        min_bytes=min_bytes,
        max_bytes=max_bytes,
    )


def to_json_shared_heap_options(value: SharedHeapOptions) -> Json:
    """Return one JSON value for one SharedHeapOptions."""
    return {
        **({} if value.min_bytes is None else {"minBytes": value.min_bytes}),
        **({} if value.max_bytes is None else {"maxBytes": value.max_bytes}),
    }


def from_json_shared_heap_options(value: Json) -> SharedHeapOptions:
    """Return one SharedHeapOptions from one JSON value."""
    object_ = json_object(value)

    return SharedHeapOptions(
        min_bytes=json_optional(object_, "minBytes", lambda value: json_int(value)),
        max_bytes=json_optional(object_, "maxBytes", lambda value: json_int(value)),
    )


__all__ = [
    "HeapOptions",
    "encode_heap_options",
    "decode_heap_options",
    "to_json_heap_options",
    "from_json_heap_options",
    "LocalHeapOptions",
    "encode_local_heap_options",
    "decode_local_heap_options",
    "to_json_local_heap_options",
    "from_json_local_heap_options",
    "SharedHeapOptions",
    "encode_shared_heap_options",
    "decode_shared_heap_options",
    "to_json_shared_heap_options",
    "from_json_shared_heap_options",
]
