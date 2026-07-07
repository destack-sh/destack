# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.core.section


@dataclass(frozen=True, slots=True)
class StaticImage:
    """Section-backed static memory image carried by a program."""

    # static bytes
    bytes: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_image(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticImage:
        """Decode one StaticImage."""
        return decode_static_image(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_image(self)

    @classmethod
    def from_json(cls, value: Json) -> StaticImage:
        """Return one StaticImage from one JSON value."""
        return from_json_static_image(value)


def encode_static_image(writer: BinaryWriter, value: StaticImage) -> None:
    """Encode one StaticImage."""
    destack._generated.core.section.encode_section_slice(writer, value.bytes)


def decode_static_image(reader: BinaryReader) -> StaticImage:
    """Decode one StaticImage."""
    bytes = destack._generated.core.section.decode_section_slice(reader)

    return StaticImage(
        bytes=bytes,
    )


def to_json_static_image(value: StaticImage) -> Json:
    """Return one JSON value for one StaticImage."""
    return {
        "bytes": destack._generated.core.section.to_json_section_slice(value.bytes),
    }


def from_json_static_image(value: Json) -> StaticImage:
    """Return one StaticImage from one JSON value."""
    object_ = json_object(value)

    return StaticImage(
        bytes=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "bytes")
        ),
    )


__all__ = [
    "StaticImage",
    "encode_static_image",
    "decode_static_image",
    "to_json_static_image",
    "from_json_static_image",
]
