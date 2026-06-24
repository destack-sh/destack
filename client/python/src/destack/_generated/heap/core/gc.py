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
class GcOptions:
    """Collector configuration for one heap."""

    # the proportional heap growth target after one cycle
    growth_percent: int
    # the heap trigger as a percentage of the current goal
    trigger_percent: int
    # optional soft memory limit in bytes
    soft_limit_bytes: int | None
    # optional minimum heap floor in bytes
    minimum_heap_bytes: int | None
    # minimum collector work for one safepoint
    minimum_work_bytes: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_gc_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GcOptions:
        """Decode one GcOptions."""
        return decode_gc_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_gc_options(self)

    @classmethod
    def from_json(cls, value: Json) -> GcOptions:
        """Return one GcOptions from one JSON value."""
        return from_json_gc_options(value)


def encode_gc_options(writer: BinaryWriter, value: GcOptions) -> None:
    """Encode one GcOptions."""
    writer.write_unsigned(value.growth_percent)
    writer.write_unsigned(value.trigger_percent)
    if value.soft_limit_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.soft_limit_bytes)
    if value.minimum_heap_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.minimum_heap_bytes)
    writer.write_unsigned(value.minimum_work_bytes)


def decode_gc_options(reader: BinaryReader) -> GcOptions:
    """Decode one GcOptions."""
    growth_percent = reader.read_number()
    trigger_percent = reader.read_number()
    soft_limit_bytes = reader.read_option(lambda: reader.read_number())
    minimum_heap_bytes = reader.read_option(lambda: reader.read_number())
    minimum_work_bytes = reader.read_number()

    return GcOptions(
        growth_percent=growth_percent,
        trigger_percent=trigger_percent,
        soft_limit_bytes=soft_limit_bytes,
        minimum_heap_bytes=minimum_heap_bytes,
        minimum_work_bytes=minimum_work_bytes,
    )


def to_json_gc_options(value: GcOptions) -> Json:
    """Return one JSON value for one GcOptions."""
    return {
        "growthPercent": value.growth_percent,
        "triggerPercent": value.trigger_percent,
        **(
            {}
            if value.soft_limit_bytes is None
            else {"softLimitBytes": value.soft_limit_bytes}
        ),
        **(
            {}
            if value.minimum_heap_bytes is None
            else {"minimumHeapBytes": value.minimum_heap_bytes}
        ),
        "minimumWorkBytes": value.minimum_work_bytes,
    }


def from_json_gc_options(value: Json) -> GcOptions:
    """Return one GcOptions from one JSON value."""
    object_ = json_object(value)

    return GcOptions(
        growth_percent=json_int(json_field(object_, "growthPercent")),
        trigger_percent=json_int(json_field(object_, "triggerPercent")),
        soft_limit_bytes=json_optional(
            object_, "softLimitBytes", lambda value: json_int(value)
        ),
        minimum_heap_bytes=json_optional(
            object_, "minimumHeapBytes", lambda value: json_int(value)
        ),
        minimum_work_bytes=json_int(json_field(object_, "minimumWorkBytes")),
    )


__all__ = [
    "GcOptions",
    "encode_gc_options",
    "decode_gc_options",
    "to_json_gc_options",
    "from_json_gc_options",
]
