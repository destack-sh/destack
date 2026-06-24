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
    json_object,
    json_optional,
    json_string,
)

import destack._generated.source.file.model.file


@dataclass(frozen=True, slots=True)
class Object:
    """Relocatable native object image."""

    # the object file format
    format: ObjectFormat
    # the object file bytes
    content: destack._generated.source.file.model.file.ContentId
    # native unwind metadata bytes
    unwind: destack._generated.source.file.model.file.ContentId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_object(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Object:
        """Decode one Object."""
        return decode_object(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_object(self)

    @classmethod
    def from_json(cls, value: Json) -> Object:
        """Return one Object from one JSON value."""
        return from_json_object(value)


def encode_object(writer: BinaryWriter, value: Object) -> None:
    """Encode one Object."""
    encode_object_format(writer, value.format)
    destack._generated.source.file.model.file.encode_content_id(writer, value.content)
    if value.unwind is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.file.encode_content_id(
            writer, value.unwind
        )


def decode_object(reader: BinaryReader) -> Object:
    """Decode one Object."""
    format = decode_object_format(reader)
    content = destack._generated.source.file.model.file.decode_content_id(reader)
    unwind = reader.read_option(
        lambda: destack._generated.source.file.model.file.decode_content_id(reader)
    )

    return Object(
        format=format,
        content=content,
        unwind=unwind,
    )


def to_json_object(value: Object) -> Json:
    """Return one JSON value for one Object."""
    return {
        "format": to_json_object_format(value.format),
        "content": destack._generated.source.file.model.file.to_json_content_id(
            value.content
        ),
        **(
            {}
            if value.unwind is None
            else {
                "unwind": destack._generated.source.file.model.file.to_json_content_id(
                    value.unwind
                )
            }
        ),
    }


def from_json_object(value: Json) -> Object:
    """Return one Object from one JSON value."""
    object_ = json_object(value)

    return Object(
        format=from_json_object_format(json_field(object_, "format")),
        content=destack._generated.source.file.model.file.from_json_content_id(
            json_field(object_, "content")
        ),
        unwind=json_optional(
            object_,
            "unwind",
            lambda value: (
                destack._generated.source.file.model.file.from_json_content_id(value)
            ),
        ),
    )


"""Native object file format."""
ObjectFormat: typing.TypeAlias = (
    typing.Literal["elf"] | typing.Literal["machO"] | typing.Literal["coff"]
)


def encode_object_format(writer: BinaryWriter, value: ObjectFormat) -> None:
    """Encode one ObjectFormat."""
    if value == "elf":
        writer.write_unsigned(0)
    elif value == "machO":
        writer.write_unsigned(1)
    elif value == "coff":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_object_format(reader: BinaryReader) -> ObjectFormat:
    """Decode one ObjectFormat."""
    variant = reader.read_number()

    if variant == 0:
        return "elf"
    elif variant == 1:
        return "machO"
    elif variant == 2:
        return "coff"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_object_format(value: ObjectFormat) -> Json:
    """Return one JSON value for one ObjectFormat."""
    return value


def from_json_object_format(value: Json) -> ObjectFormat:
    """Return one ObjectFormat from one JSON value."""
    variant = json_string(value)

    if variant == "elf":
        return "elf"
    elif variant == "machO":
        return "machO"
    elif variant == "coff":
        return "coff"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Object",
    "encode_object",
    "decode_object",
    "to_json_object",
    "from_json_object",
    "ObjectFormat",
    "encode_object_format",
    "decode_object_format",
    "to_json_object_format",
    "from_json_object_format",
]
