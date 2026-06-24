# generated bridge target, do not edit

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

import destack._generated.artifact.map
import destack._generated.source.file.model.file

"""One compiled-code object format."""
ObjectFormat: typing.TypeAlias = typing.Literal["object"] | typing.Literal["wasm"]


def encode_object_format(writer: BinaryWriter, value: ObjectFormat) -> None:
    """Encode one ObjectFormat."""
    if value == "object":
        writer.write_unsigned(0)
    elif value == "wasm":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_object_format(reader: BinaryReader) -> ObjectFormat:
    """Decode one ObjectFormat."""
    variant = reader.read_number()

    if variant == 0:
        return "object"
    elif variant == 1:
        return "wasm"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_object_format(value: ObjectFormat) -> Json:
    """Return one JSON value for one ObjectFormat."""
    return value


def from_json_object_format(value: Json) -> ObjectFormat:
    """Return one ObjectFormat from one JSON value."""
    variant = json_string(value)

    if variant == "object":
        return "object"
    elif variant == "wasm":
        return "wasm"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class Object:
    """One compiled-code linker input for a target."""

    # the compiled-code object format
    format: ObjectFormat
    # the encoded object content identity
    content: destack._generated.source.file.model.file.ContentId
    # the source map when one exists
    map: destack._generated.artifact.map.SourceMap | None

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
    if value.map is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.map.encode_source_map(writer, value.map)


def decode_object(reader: BinaryReader) -> Object:
    """Decode one Object."""
    format = decode_object_format(reader)
    content = destack._generated.source.file.model.file.decode_content_id(reader)
    map = reader.read_option(
        lambda: destack._generated.artifact.map.decode_source_map(reader)
    )

    return Object(
        format=format,
        content=content,
        map=map,
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
            if value.map is None
            else {"map": destack._generated.artifact.map.to_json_source_map(value.map)}
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
        map=json_optional(
            object_,
            "map",
            lambda value: destack._generated.artifact.map.from_json_source_map(value),
        ),
    )


__all__ = [
    "ObjectFormat",
    "encode_object_format",
    "decode_object_format",
    "to_json_object_format",
    "from_json_object_format",
    "Object",
    "encode_object",
    "decode_object",
    "to_json_object",
    "from_json_object",
]
