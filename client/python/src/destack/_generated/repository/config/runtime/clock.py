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
class ClockOptions:
    """Runtime clock configuration."""

    # runtime wall-clock epoch in nanoseconds
    epoch_ns: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_clock_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ClockOptions:
        """Decode one ClockOptions."""
        return decode_clock_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_clock_options(self)

    @classmethod
    def from_json(cls, value: Json) -> ClockOptions:
        """Return one ClockOptions from one JSON value."""
        return from_json_clock_options(value)


def encode_clock_options(writer: BinaryWriter, value: ClockOptions) -> None:
    """Encode one ClockOptions."""
    if value.epoch_ns is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.epoch_ns)


def decode_clock_options(reader: BinaryReader) -> ClockOptions:
    """Decode one ClockOptions."""
    epoch_ns = reader.read_option(lambda: reader.read_number())

    return ClockOptions(
        epoch_ns=epoch_ns,
    )


def to_json_clock_options(value: ClockOptions) -> Json:
    """Return one JSON value for one ClockOptions."""
    return {
        **({} if value.epoch_ns is None else {"epochNs": value.epoch_ns}),
    }


def from_json_clock_options(value: Json) -> ClockOptions:
    """Return one ClockOptions from one JSON value."""
    object_ = json_object(value)

    return ClockOptions(
        epoch_ns=json_optional(object_, "epochNs", lambda value: json_int(value)),
    )


__all__ = [
    "ClockOptions",
    "encode_clock_options",
    "decode_clock_options",
    "to_json_clock_options",
    "from_json_clock_options",
]
