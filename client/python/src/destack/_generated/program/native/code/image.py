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
    json_string,
)

import destack._generated.program.native.code.library
import destack._generated.program.native.code.object


@dataclass(frozen=True, slots=True)
class ImageResident:
    """Native functions are already linked into the current process."""

    kind: typing.Literal["resident"] = "resident"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_image(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_image(self)


@dataclass(frozen=True, slots=True)
class ImageLibrary:
    """Native functions live in a loadable native library."""

    library: destack._generated.program.native.code.library.Library
    kind: typing.Literal["library"] = "library"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_image(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_image(self)


@dataclass(frozen=True, slots=True)
class ImageObject:
    """Native functions live in a relocatable object artifact."""

    object: destack._generated.program.native.code.object.Object
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_image(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_image(self)


"""Durable native image payload."""
Image: typing.TypeAlias = ImageResident | ImageLibrary | ImageObject


def encode_image(writer: BinaryWriter, value: Image) -> None:
    """Encode one Image."""
    if value.kind == "resident":
        writer.write_unsigned(0)
    elif value.kind == "library":
        writer.write_unsigned(1)
        destack._generated.program.native.code.library.encode_library(
            writer, value.library
        )
    elif value.kind == "object":
        writer.write_unsigned(2)
        destack._generated.program.native.code.object.encode_object(
            writer, value.object
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_image(reader: BinaryReader) -> Image:
    """Decode one Image."""
    variant = reader.read_number()

    if variant == 0:
        return ImageResident()
    elif variant == 1:
        library = destack._generated.program.native.code.library.decode_library(reader)

        return ImageLibrary(library=library)
    elif variant == 2:
        object = destack._generated.program.native.code.object.decode_object(reader)

        return ImageObject(object=object)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_image(value: Image) -> Json:
    """Return one JSON value for one Image."""
    if value.kind == "resident":
        return {
            "kind": "resident",
        }
    elif value.kind == "library":
        return {
            "kind": "library",
            "library": destack._generated.program.native.code.library.to_json_library(
                value.library
            ),
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "object": destack._generated.program.native.code.object.to_json_object(
                value.object
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_image(value: Json) -> Image:
    """Return one Image from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "resident":
        return ImageResident()
    elif kind == "library":
        return ImageLibrary(
            library=destack._generated.program.native.code.library.from_json_library(
                json_field(object_, "library")
            )
        )
    elif kind == "object":
        return ImageObject(
            object=destack._generated.program.native.code.object.from_json_object(
                json_field(object_, "object")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Image",
    "encode_image",
    "decode_image",
    "to_json_image",
    "from_json_image",
    "ImageResident",
    "ImageLibrary",
    "ImageObject",
]
