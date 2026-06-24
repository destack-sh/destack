# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)

from destack._impl.program.vm.meta import (
    ReferenceMetaImpl,
)


@dataclass(frozen=True, slots=True)
class ReferenceMeta(ReferenceMetaImpl):
    """Metadata for reference values."""

    bits: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference_meta(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceMeta:
        """Decode one ReferenceMeta."""
        return decode_reference_meta(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference_meta(self)

    @classmethod
    def from_json(cls, value: Json) -> ReferenceMeta:
        """Return one ReferenceMeta from one JSON value."""
        return from_json_reference_meta(value)


def encode_reference_meta(writer: BinaryWriter, value: ReferenceMeta) -> None:
    """Encode one ReferenceMeta."""
    writer.write_unsigned(value.bits)


def decode_reference_meta(reader: BinaryReader) -> ReferenceMeta:
    """Decode one ReferenceMeta."""
    bits = reader.read_number()

    return ReferenceMeta(
        bits=bits,
    )


def to_json_reference_meta(value: ReferenceMeta) -> Json:
    """Return one JSON value for one ReferenceMeta."""
    return {
        "bits": value.bits,
    }


def from_json_reference_meta(value: Json) -> ReferenceMeta:
    """Return one ReferenceMeta from one JSON value."""
    object_ = json_object(value)

    return ReferenceMeta(
        bits=json_int(json_field(object_, "bits")),
    )


__all__ = [
    "ReferenceMeta",
    "encode_reference_meta",
    "decode_reference_meta",
    "to_json_reference_meta",
    "from_json_reference_meta",
]
