# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.program.native.library
import destack._generated.program.native.object

@dataclass(frozen=True, slots=True)
class ImageResident:
    """Native functions are already linked into the current process."""

    kind: typing.Literal["resident"] = "resident"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ImageLibrary:
    """Native functions live in a loadable native library."""

    library: destack._generated.program.native.library.Library
    kind: typing.Literal["library"] = "library"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ImageObject:
    """Native functions live in a relocatable object artifact."""

    object: destack._generated.program.native.object.Object
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Durable native image payload."""
Image: typing.TypeAlias = ImageResident | ImageLibrary | ImageObject

def encode_image(writer: BinaryWriter, value: Image) -> None: ...
def decode_image(reader: BinaryReader) -> Image: ...
def to_json_image(value: Image) -> Json: ...
def from_json_image(value: Json) -> Image: ...

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
