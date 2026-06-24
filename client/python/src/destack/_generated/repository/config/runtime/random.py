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
class RandomOptions:
    """Runtime randomness configuration."""

    # seed for deterministic randomness streams
    seed: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_random_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RandomOptions:
        """Decode one RandomOptions."""
        return decode_random_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_random_options(self)

    @classmethod
    def from_json(cls, value: Json) -> RandomOptions:
        """Return one RandomOptions from one JSON value."""
        return from_json_random_options(value)


def encode_random_options(writer: BinaryWriter, value: RandomOptions) -> None:
    """Encode one RandomOptions."""
    if value.seed is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.seed)


def decode_random_options(reader: BinaryReader) -> RandomOptions:
    """Decode one RandomOptions."""
    seed = reader.read_option(lambda: reader.read_number())

    return RandomOptions(
        seed=seed,
    )


def to_json_random_options(value: RandomOptions) -> Json:
    """Return one JSON value for one RandomOptions."""
    return {
        **({} if value.seed is None else {"seed": value.seed}),
    }


def from_json_random_options(value: Json) -> RandomOptions:
    """Return one RandomOptions from one JSON value."""
    object_ = json_object(value)

    return RandomOptions(
        seed=json_optional(object_, "seed", lambda value: json_int(value)),
    )


__all__ = [
    "RandomOptions",
    "encode_random_options",
    "decode_random_options",
    "to_json_random_options",
    "from_json_random_options",
]
